//! `email` expansion — declares an email a command sends, so the manifest carries it.
//!
//! The author never writes the manifest's `emails[]`: the build reads it from here. The command
//! is the one the attribute sits on, which is why it must sit above `#[command]` — below it, the
//! command attribute has already been consumed and there is no name left to read.

use proc_macro::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{ItemFn, LitStr, Token};

use crate::emit::{sanitize_key, write_emission};
use crate::query::NamedOpAttrs;

const AUDIENCES: [&str; 3] = ["guest", "host", "propertyEligibleGuests"];

/// Bare flags, spelled as in Rust and written camelCase at the manifest's root.
const FLAGS: [(&str, &str); 4] = [
    ("dispatch_on_stay_created", "dispatchOnStayCreated"),
    ("catch_up_on_property_publish", "catchUpOnPropertyPublish"),
    ("catch_up_on_config_update", "catchUpOnConfigUpdate"),
    ("requires_guest_email", "requiresGuestEmail"),
];

#[derive(Debug)]
pub(crate) struct EmailAttrs {
    id: String,
    audience: String,
    trigger: String,
    offset: Option<String>,
    at_local_time: Option<String>,
    flags: Vec<&'static str>,
}

impl Parse for EmailAttrs {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let mut id = None;
        let mut audience = None;
        let mut trigger = None;
        let mut offset = None;
        let mut at_local_time = None;
        let mut flags = Vec::new();

        while !input.is_empty() {
            let key: syn::Ident = input.parse()?;
            let name = key.to_string();
            if let Some((_, wire)) = FLAGS.iter().find(|(rust, _)| *rust == name) {
                flags.push(*wire);
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
            // Sent when the command runs, unless the author times it against the stay.
            trigger: trigger.unwrap_or_else(|| "moduleCommand".to_string()),
            offset,
            at_local_time,
            flags,
        })
    }
}

/// The manifest's `emails[]` entry, plus the emission `kind`.
pub(crate) fn declaration(attrs: &EmailAttrs, command: &str) -> serde_json::Value {
    let mut trigger = serde_json::json!({ "type": attrs.trigger });
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
        "command": command,
        "trigger": trigger,
    });
    for flag in &attrs.flags {
        entry[*flag] = true.into();
    }
    entry
}

/// Reads the name of the `#[command]` still attached to the function.
fn command_name(function: &ItemFn) -> Option<syn::Result<String>> {
    function
        .attrs
        .iter()
        .find(|attr| {
            attr.path()
                .segments
                .last()
                .is_some_and(|segment| segment.ident == "command")
        })
        .map(|attr| attr.parse_args::<NamedOpAttrs>().map(|op| op.name))
}

/// Expands `#[email(id = "…", audience = "…")]` on a command handler.
pub fn expand(attr: TokenStream, item: TokenStream) -> TokenStream {
    let function_item = syn::parse_macro_input!(item as ItemFn);
    let attrs = syn::parse_macro_input!(attr as EmailAttrs);

    let command = match command_name(&function_item) {
        Some(Ok(name)) => name,
        Some(Err(error)) => return error.to_compile_error().into(),
        None => {
            return syn::Error::new_spanned(
                &function_item.sig.ident,
                "#[email] goes above #[command(name = \"…\")] on the command that sends it",
            )
            .to_compile_error()
            .into()
        }
    };

    let json = serde_json::to_string_pretty(&declaration(&attrs, &command)).unwrap();
    let emission = write_emission("email", &sanitize_key(&attrs.id), &json);
    quote! {
        #emission
        #function_item
    }
    .into()
}

#[cfg(test)]
mod tests {
    use super::{command_name, declaration, EmailAttrs};

    fn parse(attr: &str) -> syn::Result<EmailAttrs> {
        syn::parse_str(attr)
    }

    #[test]
    fn a_command_email_is_sent_when_the_command_runs() {
        let attrs = parse(r#"id = "submitted", audience = "host""#).unwrap();
        assert_eq!(
            declaration(&attrs, "submit"),
            serde_json::json!({
                "kind": "email",
                "id": "submitted",
                "audience": "host",
                "command": "submit",
                "trigger": { "type": "moduleCommand" },
            })
        );
    }

    #[test]
    fn a_timed_email_carries_its_offset_and_flags() {
        let attrs = parse(
            r#"id = "checkout-j2", audience = "guest", trigger = "relativeToCheckOut",
               offset = "P2D", requires_guest_email, dispatch_on_stay_created,"#,
        )
        .unwrap();
        let entry = declaration(&attrs, "sendCheckoutFollowUp");
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
    fn the_command_is_read_from_its_attribute() {
        let function: syn::ItemFn = syn::parse_quote! {
            #[portaki_sdk::command(name = "submit", guest)]
            fn submit() {}
        };
        assert_eq!(command_name(&function).unwrap().unwrap(), "submit");

        let bare: syn::ItemFn = syn::parse_quote! { fn submit() {} };
        assert!(command_name(&bare).is_none());
    }
}
