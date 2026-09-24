//! `surface` expansion — host/guest SDUI renderers in the manifest and Wasm dispatch table.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{ItemFn, LitStr, Token};

use crate::emit::{sanitize_key, write_emission};
use crate::wire_lit::WireLit;

struct SurfaceAttrs {
    context: String,
    id: String,
    display_name_key: Option<String>,
    /// Where the dashboard or the booklet links to it — `portaki build` writes the catalog's
    /// `hostSurfaces` / `guestSurfaces` from it. Empty: the surface is rendered, never linked.
    catalog: serde_json::Map<String, serde_json::Value>,
    /// Compile-time checks that each typed value names a real variant.
    checks: Vec<TokenStream2>,
}

/// Keys that describe where a surface is linked from, per context.
const HOST_KEYS: [&str; 5] = ["placement", "design_id", "label_key", "icon", "path"];
const GUEST_KEYS: [&str; 4] = ["path", "label_key", "role", "embeds"];

impl Parse for SurfaceAttrs {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let context = if input.peek(LitStr) {
            input.parse::<LitStr>()?.value()
        } else {
            let ident: syn::Ident = input.parse()?;
            ident.to_string()
        };
        input.parse::<Token![,]>()?;
        let id_key: syn::Ident = input.parse()?;
        if id_key != "id" {
            return Err(syn::Error::new(
                id_key.span(),
                "expected id = \"...\" or id = SurfaceId::new(\"...\")",
            ));
        }
        input.parse::<Token![=]>()?;
        let id: WireLit = input.parse()?;

        let mut display_name_key = None;
        let mut catalog = serde_json::Map::new();
        let mut checks = Vec::new();
        let allowed: &[&str] = if context == "host" {
            &HOST_KEYS
        } else {
            &GUEST_KEYS
        };
        while input.parse::<Option<Token![,]>>()?.is_some() && !input.is_empty() {
            let key: syn::Ident = input.parse()?;
            input.parse::<Token![=]>()?;
            let name = key.to_string();
            if name == "display_name_key" {
                display_name_key = Some(input.parse::<LitStr>()?.value());
                continue;
            }
            if !allowed.contains(&name.as_str()) {
                return Err(syn::Error::new(
                    key.span(),
                    format!(
                        "unknown #[surface({context})] attribute: {name} — expected one of \
                         display_name_key, {}",
                        allowed.join(", ")
                    ),
                ));
            }
            let value = crate::typed::nav_value(input, &name, &mut checks)?;
            // `placement` and `embeds` repeat: one surface can sit in several places.
            if name == "placement" || name == "embeds" {
                catalog
                    .entry(name)
                    .or_insert_with(|| serde_json::Value::Array(Vec::new()))
                    .as_array_mut()
                    .expect("list")
                    .push(value);
            } else if catalog.insert(name.clone(), value).is_some() {
                return Err(syn::Error::new(
                    key.span(),
                    format!("{name} is given twice"),
                ));
            }
        }
        if !input.is_empty() {
            return Err(input.error("expected `, key = \"…\"`"));
        }

        Ok(SurfaceAttrs {
            context,
            id: id.value,
            display_name_key,
            catalog,
            checks,
        })
    }
}

/// Expands `#[surface(host|guest, id = "…", …)]` on a render function.
pub fn expand(attr: TokenStream, item: TokenStream) -> TokenStream {
    let function_item = syn::parse_macro_input!(item as ItemFn);
    let attrs = syn::parse_macro_input!(attr as SurfaceAttrs);
    let fn_name = function_item.sig.ident.to_string();

    let display_name_key = attrs.display_name_key.unwrap_or_default();
    let display_fragment = if display_name_key.is_empty() {
        String::new()
    } else {
        format!(
            ",\n  \"displayNameKey\": {}",
            serde_json::to_string(&display_name_key).unwrap()
        )
    };

    let mut json = format!(
        r#"{{
  "kind": "surface",
  "context": {},
  "id": {},
  "renderFn": {}{}
}}"#,
        serde_json::to_string(&attrs.context).unwrap(),
        serde_json::to_string(&attrs.id).unwrap(),
        serde_json::to_string(&fn_name).unwrap(),
        display_fragment,
    );
    if !attrs.catalog.is_empty() {
        let mut value: serde_json::Value = serde_json::from_str(&json).unwrap();
        value["catalog"] = serde_json::Value::Object(attrs.catalog.clone());
        json = serde_json::to_string_pretty(&value).unwrap();
    }

    let key = format!("{}_{}", attrs.context, attrs.id);
    let emission = write_emission("surface", &sanitize_key(&key), &json);
    let wasm_registration =
        crate::wasm_handler::register_surface(&attrs.context, &attrs.id, &fn_name, &function_item);
    let checks = &attrs.checks;
    let output: TokenStream2 = quote! {
        #emission
        #(#checks)*
        #function_item
        #wasm_registration
    };

    output.into()
}

#[cfg(test)]
mod tests {
    use super::SurfaceAttrs;

    fn parse(attr: &str) -> syn::Result<SurfaceAttrs> {
        syn::parse_str(attr)
    }

    #[test]
    fn a_host_surface_can_sit_in_several_places() {
        let attrs = parse(
            r#"host, id = "issue-stats", placement = HostPlacement::PropertyStatsCard,
               placement = HostPlacement::PropertyStatsDetail, label_key = "nav.stats",
               icon = IconName::DangerTriangle, design_id = DesignId::RulesEditorV1,"#,
        )
        .unwrap();
        assert_eq!(
            serde_json::Value::Object(attrs.catalog),
            serde_json::json!({
                "placement": ["HostPlacement::PropertyStatsCard", "HostPlacement::PropertyStatsDetail"],
                "label_key": "nav.stats",
                "icon": "IconName::DangerTriangle",
                "design_id": "DesignId::RulesEditorV1",
            })
        );
        assert_eq!(attrs.checks.len(), 4);
    }

    #[test]
    fn a_guest_route_keeps_the_display_name_key_apart() {
        let attrs = parse(
            r#"guest, id = "home.card", display_name_key = "x", path = "pre-arrival-form",
               role = GuestRole::ArrivalFormality, embeds = HostFragmentId::PoliceForm"#,
        )
        .unwrap();
        assert_eq!(attrs.display_name_key.as_deref(), Some("x"));
        assert_eq!(
            attrs.catalog["embeds"],
            serde_json::json!(["HostFragmentId::PoliceForm"])
        );
        assert_eq!(attrs.catalog["role"], "GuestRole::ArrivalFormality");
    }

    #[test]
    fn keys_belong_to_their_context_and_appear_once() {
        assert!(
            parse(r#"guest, id = "home.card", placement = HostPlacement::StayAction"#).is_err()
        );
        assert!(parse(r#"host, id = "main", role = GuestRole::Card"#).is_err());
        assert!(
            parse(r#"host, id = "main", icon = IconName::Key, icon = IconName::Lock"#).is_err()
        );
        assert!(
            parse(r#"host, id = "main", icon = "key""#).is_err(),
            "a string is refused"
        );
        assert!(parse(r#"host, id = "main""#).unwrap().catalog.is_empty());
    }
}
