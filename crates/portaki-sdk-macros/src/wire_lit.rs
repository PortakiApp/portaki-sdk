//! Parse a wire-string attribute value: `"…"` or `Type::new("…")`.

use syn::parse::{Parse, ParseStream};
use syn::{Expr, LitStr};

/// String extracted from a lit or `Ident::new("…")` / path call.
pub struct WireLit {
    pub value: String,
}

impl Parse for WireLit {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        if input.peek(LitStr) {
            let lit: LitStr = input.parse()?;
            return Ok(Self { value: lit.value() });
        }

        let expr: Expr = input.parse()?;
        wire_string_from_expr(&expr)
            .map(|value| Self { value })
            .ok_or_else(|| {
                syn::Error::new_spanned(
                    &expr,
                    "expected a string literal or Type::new(\"…\") (e.g. SurfaceId::new(\"home.card\"))",
                )
            })
    }
}

fn wire_string_from_expr(expr: &Expr) -> Option<String> {
    match expr {
        Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Str(lit),
            ..
        }) => Some(lit.value()),
        Expr::Call(call) => {
            let arg = call.args.first()?;
            match arg {
                Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(lit),
                    ..
                }) => Some(lit.value()),
                _ => None,
            }
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::wire_string_from_expr;
    use syn::parse_quote;

    #[test]
    fn extracts_string_literal() {
        let expr: syn::Expr = parse_quote! { "home.card" };
        assert_eq!(wire_string_from_expr(&expr).as_deref(), Some("home.card"));
    }

    #[test]
    fn extracts_new_call() {
        let expr: syn::Expr = parse_quote! { SurfaceId::new("home.card") };
        assert_eq!(wire_string_from_expr(&expr).as_deref(), Some("home.card"));
    }

    #[test]
    fn rejects_bare_path() {
        let expr: syn::Expr = parse_quote! { HOME_CARD };
        assert!(wire_string_from_expr(&expr).is_none());
    }
}
