//! `email` expansion — declares an email a command or a query sends, so the manifest carries it.
//!
//! The author never writes the manifest's `emails[]`: the build reads it from here. The operation
//! is the one the attribute sits on, which is why it must sit above `#[command]` / `#[query]` —
//! below it, that attribute has already been consumed and there is no name left to read.

use proc_macro::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{ItemFn, LitStr, Token};

use crate::emit::{sanitize_key, write_emission};
use crate::query::NamedOpAttrs;
use crate::typed::{self, EMAIL_AUDIENCE, EMAIL_TRIGGER, SKIP_WHEN};

/// Bare flags, spelled as in Rust and written camelCase at the manifest's root.
///
/// `requires_guest_email` is not among them: the platform reads a missing `requiresGuestEmail` as
/// `true`, so it is always written — `false` unless the flag says otherwise.
const FLAGS: [(&str, &str); 3] = [
    ("dispatch_on_stay_created", "dispatchOnStayCreated"),
    ("catch_up_on_property_publish", "catchUpOnPropertyPublish"),
    ("catch_up_on_config_update", "catchUpOnConfigUpdate"),
];

pub(crate) struct EmailAttrs {
    id: String,
    /// `EmailAudience::Variant` — resolved to the wire string by `portaki build`.
    audience: String,
    /// `None` when the author left it to the default — `moduleCommand`, for a command only.
    trigger: Option<String>,
    offset: Option<String>,
    at_local_time: Option<String>,
    flags: Vec<&'static str>,
    requires_guest_email: bool,
    /// i18n key of the description the dashboard shows; `portaki build` translates it.
    description_key: Option<String>,
    skip_when: Vec<String>,
    /// Compile-time checks that each typed value names a real variant.
    checks: Vec<proc_macro2::TokenStream>,
}

impl Parse for EmailAttrs {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let mut id = None;
        let mut audience = None;
        let mut trigger = None;
        let mut offset = Offset::default();
        let mut at_local_time = None;
        let mut checks = Vec::new();
        let mut flags = Vec::new();
        let mut requires_guest_email = false;
        let mut description_key = None;
        let mut skip_when = Vec::new();

        while !input.is_empty() {
            let key: syn::Ident = input.parse()?;
            let name = key.to_string();
            if let Some((_, wire)) = FLAGS.iter().find(|(rust, _)| *rust == name) {
                flags.push(*wire);
            } else if name == "requires_guest_email" {
                requires_guest_email = true;
            } else {
                input.parse::<Token![=]>()?;
                match name.as_str() {
                    "audience" => {
                        let value = typed::parse(input, &name, EMAIL_AUDIENCE)?;
                        checks.push(value.check);
                        audience = Some(value.emitted);
                    }
                    "trigger" => {
                        let value = typed::parse(input, &name, EMAIL_TRIGGER)?;
                        checks.push(value.check);
                        trigger = Some(value.emitted);
                    }
                    // Repeats: each one a condition the platform skips the send on.
                    "skip_when" => {
                        let value = typed::parse(input, &name, SKIP_WHEN)?;
                        checks.push(value.check);
                        skip_when.push(value.emitted);
                    }
                    "offset_days" | "offset_hours" | "offset_minutes" => {
                        let amount: syn::LitInt = input.parse()?;
                        offset.set(&name, amount.base10_parse()?);
                    }
                    "offset" => return Err(syn::Error::new(
                        key.span(),
                        "write the offset as offset_days / offset_hours / offset_minutes = <int>",
                    )),
                    "at_local_time" => at_local_time = Some(typed::local_time(&input.parse()?)?),
                    "id" => id = Some(input.parse::<LitStr>()?.value()),
                    "description_key" => description_key = Some(input.parse::<LitStr>()?.value()),
                    other => {
                        return Err(syn::Error::new(
                            key.span(),
                            format!("unknown #[email] attribute: {other}"),
                        ))
                    }
                }
            }
            if !input.is_empty() {
                input.parse::<Token![,]>()?;
            }
        }

        Ok(EmailAttrs {
            id: id.ok_or_else(|| input.error("#[email] needs id = \"…\""))?,
            audience: audience
                .ok_or_else(|| input.error("#[email] needs audience = EmailAudience::…"))?,
            trigger,
            offset: offset.iso8601().map_err(|message| input.error(message))?,
            at_local_time,
            flags,
            requires_guest_email,
            description_key,
            skip_when,
            checks,
        })
    }
}

/// An offset against the stay, in whole units — written ISO 8601 (`P2D`, `-PT3H`) for the platform.
#[derive(Default)]
struct Offset {
    days: Option<i64>,
    hours: Option<i64>,
    minutes: Option<i64>,
}

impl Offset {
    fn set(&mut self, unit: &str, value: i64) {
        match unit {
            "offset_days" => self.days = Some(value),
            "offset_hours" => self.hours = Some(value),
            _ => self.minutes = Some(value),
        }
    }

    fn iso8601(&self) -> Result<Option<String>, &'static str> {
        let parts = [self.days, self.hours, self.minutes];
        if parts.iter().all(Option::is_none) {
            return Ok(None);
        }
        let values: Vec<i64> = parts.iter().map(|p| p.unwrap_or(0)).collect();
        if values.iter().any(|v| *v < 0) && values.iter().any(|v| *v > 0) {
            return Err("an offset goes one way: its days, hours and minutes share a sign");
        }
        let sign = if values.iter().any(|v| *v < 0) {
            "-"
        } else {
            ""
        };
        let [days, hours, minutes] = [values[0].abs(), values[1].abs(), values[2].abs()];
        let mut iso = format!("{sign}P");
        if days > 0 {
            iso.push_str(&format!("{days}D"));
        }
        if hours > 0 || minutes > 0 {
            iso.push('T');
            if hours > 0 {
                iso.push_str(&format!("{hours}H"));
            }
            if minutes > 0 {
                iso.push_str(&format!("{minutes}M"));
            }
        }
        if iso.ends_with('P') {
            iso.push_str("0D");
        }
        Ok(Some(iso))
    }
}

/// The operation an email hangs off: a command the platform may dispatch, or a query whose run
/// sends it (ical-sync's `applyFeeds`).
pub(crate) enum Operation {
    Command(String),
    Query,
}

/// The manifest's `emails[]` entry, plus the emission `kind`.
///
/// A command email defaults to `moduleCommand` — sent when the command runs — and names the
/// command. A query email names nothing the platform could dispatch, so it must say its trigger.
pub(crate) fn declaration(
    attrs: &EmailAttrs,
    operation: &Operation,
) -> syn::Result<serde_json::Value> {
    let trigger_type = match (&attrs.trigger, operation) {
        (Some(trigger), _) => trigger.clone(),
        (None, Operation::Command(_)) => "EmailTrigger::ModuleCommand".to_string(),
        (None, Operation::Query) => {
            return Err(typed::error(
                "#[email] on a query needs trigger = EmailTrigger::… — only a command can be dispatched",
            ))
        }
    };
    let mut trigger = serde_json::json!({ "type": trigger_type });
    if let Some(offset) = &attrs.offset {
        trigger["offset"] = offset.as_str().into();
    }
    if let Some(at) = &attrs.at_local_time {
        trigger["atLocalTime"] = at.as_str().into();
    }
    let mut entry = serde_json::json!({
        "kind": "email",
        "id": attrs.id,
        "audience": attrs.audience,
        "trigger": trigger,
        "requiresGuestEmail": attrs.requires_guest_email,
    });
    if let Operation::Command(command) = operation {
        entry["command"] = command.as_str().into();
    }
    for flag in &attrs.flags {
        entry[*flag] = true.into();
    }
    if let Some(key) = &attrs.description_key {
        entry["descriptionKey"] = key.as_str().into();
    }
    if !attrs.skip_when.is_empty() {
        entry["skipWhen"] = attrs.skip_when.clone().into();
    }
    Ok(entry)
}

/// The `#[command]` or `#[query]` still attached to the function.
fn operation(function: &ItemFn) -> Option<syn::Result<Operation>> {
    function.attrs.iter().find_map(|attr| {
        let last = attr.path().segments.last()?.ident.to_string();
        match last.as_str() {
            "command" => Some(
                attr.parse_args::<NamedOpAttrs>()
                    .map(|op| Operation::Command(op.name)),
            ),
            "query" => Some(Ok(Operation::Query)),
            _ => None,
        }
    })
}

/// Expands `#[email(id = "…", audience = "…")]` on a command handler.
pub fn expand(attr: TokenStream, item: TokenStream) -> TokenStream {
    let function_item = syn::parse_macro_input!(item as ItemFn);
    let attrs = syn::parse_macro_input!(attr as EmailAttrs);

    let operation = match operation(&function_item) {
        Some(Ok(operation)) => operation,
        Some(Err(error)) => return error.to_compile_error().into(),
        None => {
            return syn::Error::new_spanned(
                &function_item.sig.ident,
                "#[email] goes above the #[command] or #[query] that sends it",
            )
            .to_compile_error()
            .into()
        }
    };
    let entry = match declaration(&attrs, &operation) {
        Ok(entry) => entry,
        Err(error) => return error.to_compile_error().into(),
    };

    let json = serde_json::to_string_pretty(&entry).unwrap();
    let emission = write_emission("email", &sanitize_key(&attrs.id), &json);
    let checks = &attrs.checks;
    quote! {
        #emission
        #(#checks)*
        #function_item
    }
    .into()
}

#[cfg(test)]
mod tests {
    use super::{declaration, operation, EmailAttrs, Operation};

    fn command(name: &str) -> Operation {
        Operation::Command(name.to_string())
    }

    fn parse(attr: &str) -> syn::Result<EmailAttrs> {
        syn::parse_str(attr)
    }

    #[test]
    fn a_command_email_is_sent_when_the_command_runs() {
        let attrs = parse(r#"id = "submitted", audience = EmailAudience::Host"#).unwrap();
        assert_eq!(
            declaration(&attrs, &command("submit")).unwrap(),
            serde_json::json!({
                "kind": "email",
                "id": "submitted",
                "audience": "EmailAudience::Host",
                "command": "submit",
                "trigger": { "type": "EmailTrigger::ModuleCommand" },
                "requiresGuestEmail": false,
            })
        );
    }

    #[test]
    fn a_timed_email_carries_its_offset_and_flags() {
        let attrs = parse(
            r#"id = "checkout-j2", audience = EmailAudience::Guest,
               trigger = EmailTrigger::RelativeToCheckOut, offset_days = 2,
               requires_guest_email, dispatch_on_stay_created,
               description_key = "email.checkout", skip_when = SkipWhen::GuestEmailMissing,
               skip_when = SkipWhen::StayCancelled,"#,
        )
        .unwrap();
        let entry = declaration(&attrs, &command("sendCheckoutFollowUp")).unwrap();
        assert_eq!(entry["descriptionKey"], "email.checkout");
        assert_eq!(
            entry["skipWhen"],
            serde_json::json!(["SkipWhen::GuestEmailMissing", "SkipWhen::StayCancelled"])
        );
        assert_eq!(
            entry["trigger"],
            serde_json::json!({ "type": "EmailTrigger::RelativeToCheckOut", "offset": "P2D" })
        );
        assert_eq!(entry["requiresGuestEmail"], true);
        assert_eq!(entry["dispatchOnStayCreated"], true);
        assert!(entry.get("catchUpOnPropertyPublish").is_none());
    }

    #[test]
    fn id_and_a_known_audience_are_required() {
        assert!(parse(r#"audience = EmailAudience::Host"#).is_err());
        assert!(parse(r#"id = "x""#).is_err());
        assert!(
            parse(r#"id = "x", audience = "host""#).is_err(),
            "a string is refused"
        );
        assert!(parse(r#"id = "x", audience = GuestRole::Card"#).is_err());
        assert!(parse(r#"id = "x", audience = EmailAudience::Host, subject = "y""#).is_err());
        assert!(parse(r#"id = "x", audience = EmailAudience::Host, offset = "P2D""#).is_err());
    }

    #[test]
    fn the_operation_is_read_from_its_attribute() {
        let function: syn::ItemFn = syn::parse_quote! {
            #[portaki_sdk::command(name = "submit", guest)]
            fn submit() {}
        };
        assert!(matches!(
            operation(&function).unwrap().unwrap(),
            Operation::Command(name) if name == "submit"
        ));

        let query: syn::ItemFn = syn::parse_quote! {
            #[portaki_sdk::query(name = "applyFeeds")]
            fn apply_feeds() {}
        };
        assert!(matches!(
            operation(&query).unwrap().unwrap(),
            Operation::Query
        ));

        let bare: syn::ItemFn = syn::parse_quote! { fn submit() {} };
        assert!(operation(&bare).is_none());
    }

    #[test]
    fn a_query_email_names_no_command_and_says_its_trigger() {
        let timed =
            parse(r#"id = "sync-failed", audience = EmailAudience::Host, trigger = EmailTrigger::OnApplyFeeds"#).unwrap();
        let entry = declaration(&timed, &Operation::Query).unwrap();
        assert!(entry.get("command").is_none());
        assert_eq!(entry["trigger"]["type"], "EmailTrigger::OnApplyFeeds");

        let untimed = parse(r#"id = "sync-failed", audience = EmailAudience::Host"#).unwrap();
        assert!(declaration(&untimed, &Operation::Query).is_err());
    }

    #[test]
    fn an_offset_is_written_iso_8601_with_one_sign() {
        let offset = |attr: &str| {
            parse(&format!(
                r#"id = "x", audience = EmailAudience::Guest, {attr}"#
            ))
            .map(|attrs| attrs.offset)
        };
        assert_eq!(offset("offset_days = 2").unwrap().as_deref(), Some("P2D"));
        assert_eq!(
            offset("offset_hours = -3").unwrap().as_deref(),
            Some("-PT3H")
        );
        assert_eq!(
            offset("offset_days = 1, offset_hours = 6, offset_minutes = 30")
                .unwrap()
                .as_deref(),
            Some("P1DT6H30M")
        );
        assert!(offset("offset_days = 1, offset_hours = -6").is_err());
    }
}
