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

const AUDIENCES: [&str; 3] = ["guest", "host", "propertyEligibleGuests"];

/// Bare flags, spelled as in Rust and written camelCase at the manifest's root.
///
/// `requires_guest_email` is not among them: the platform reads a missing `requiresGuestEmail` as
/// `true`, so it is always written — `false` unless the flag says otherwise.
const FLAGS: [(&str, &str); 3] = [
    ("dispatch_on_stay_created", "dispatchOnStayCreated"),
    ("catch_up_on_property_publish", "catchUpOnPropertyPublish"),
    ("catch_up_on_config_update", "catchUpOnConfigUpdate"),
];

#[derive(Debug)]
pub(crate) struct EmailAttrs {
    id: String,
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
}

impl Parse for EmailAttrs {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let mut id = None;
        let mut audience = None;
        let mut trigger = None;
        let mut offset = None;
        let mut at_local_time = None;
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
                let value = input.parse::<LitStr>()?.value();
                match name.as_str() {
                    "id" => id = Some(value),
                    "audience" if AUDIENCES.contains(&value.as_str()) => audience = Some(value),
                    "audience" => {
                        return Err(syn::Error::new(
                            key.span(),
                            format!("audience must be one of {}", AUDIENCES.join(", ")),
                        ))
                    }
                    "trigger" => trigger = Some(value),
                    "offset" => offset = Some(value),
                    "at_local_time" => at_local_time = Some(value),
                    "description_key" => description_key = Some(value),
                    // Repeats: each one a condition the platform skips the send on.
                    "skip_when" => skip_when.push(value),
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
            audience: audience.ok_or_else(|| input.error("#[email] needs audience = \"…\""))?,
            trigger,
            offset,
            at_local_time,
            flags,
            requires_guest_email,
            description_key,
            skip_when,
        })
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
        (None, Operation::Command(_)) => "moduleCommand".to_string(),
        (None, Operation::Query) => {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                "#[email] on a query needs trigger = \"…\" — only a command can be dispatched",
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
    quote! {
        #emission
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
        let attrs = parse(r#"id = "submitted", audience = "host""#).unwrap();
        assert_eq!(
            declaration(&attrs, &command("submit")).unwrap(),
            serde_json::json!({
                "kind": "email",
                "id": "submitted",
                "audience": "host",
                "command": "submit",
                "trigger": { "type": "moduleCommand" },
                "requiresGuestEmail": false,
            })
        );
    }

    #[test]
    fn a_timed_email_carries_its_offset_and_flags() {
        let attrs = parse(
            r#"id = "checkout-j2", audience = "guest", trigger = "relativeToCheckOut",
               offset = "P2D", requires_guest_email, dispatch_on_stay_created,
               description_key = "email.checkout", skip_when = "guest.email.missing",
               skip_when = "stay.cancelled","#,
        )
        .unwrap();
        let entry = declaration(&attrs, &command("sendCheckoutFollowUp")).unwrap();
        assert_eq!(entry["descriptionKey"], "email.checkout");
        assert_eq!(
            entry["skipWhen"],
            serde_json::json!(["guest.email.missing", "stay.cancelled"])
        );
        assert_eq!(
            entry["trigger"],
            serde_json::json!({ "type": "relativeToCheckOut", "offset": "P2D" })
        );
        assert_eq!(entry["requiresGuestEmail"], true);
        assert_eq!(entry["dispatchOnStayCreated"], true);
        assert!(entry.get("catchUpOnPropertyPublish").is_none());
    }

    #[test]
    fn id_and_a_known_audience_are_required() {
        assert!(parse(r#"audience = "host""#).is_err());
        assert!(parse(r#"id = "x""#).is_err());
        assert!(parse(r#"id = "x", audience = "hosts""#).is_err());
        assert!(parse(r#"id = "x", audience = "host", subject = "y""#).is_err());
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
            parse(r#"id = "sync-failed", audience = "host", trigger = "onApplyFeeds""#).unwrap();
        let entry = declaration(&timed, &Operation::Query).unwrap();
        assert!(entry.get("command").is_none());
        assert_eq!(entry["trigger"]["type"], "onApplyFeeds");

        let untimed = parse(r#"id = "sync-failed", audience = "host""#).unwrap();
        assert!(declaration(&untimed, &Operation::Query).is_err());
    }
}
