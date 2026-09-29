//! Viator connector (`connector_id = "viator"`).
//!
//! Wraps [`portaki_sdk::host::connectors::call`] for the **Partner API v2**, Basic affiliate
//! access: read-only product content. Nothing here books: a guest follows the product's
//! `productUrl` to viator.com, where the booking — and the affiliate commission — happens.
//!
//! The platform pins the host (`api.viator.com`), sends the key as `exp-api-key`, sets the
//! versioned `Accept` header, and moves [`FreetextProductsArgs::lang`] into `Accept-Language`.
//! Every other field is the JSON body of `POST /partner/search/freetext`, as Viator names it.
//!
//! # Capabilities
//!
//! Requires `external.viator.pool` — Portaki's affiliate key, counted per workspace and month.
//! There is no BYOK: the Viator licence forbids showing its content outside the key holder's
//! domain, and the booklet lives on Portaki's.
//!
//! # Affiliate tracking
//!
//! On an affiliate key, `productUrl` already carries the tracking parameters (`pid`, `mcid`,
//! `medium`). This client keeps that URL as returned.
//!
//! Reference: <https://docs.viator.com/partner-api/technical/#operation/freetextSearch>

use portaki_sdk::host::connectors;
use portaki_sdk::Result as SdkResult;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Namespace for Viator host connector operations.
pub struct Viator;

/// Upper bound Viator accepts for a page of freetext results.
pub const MAX_COUNT: u32 = 50;

/// Arguments for [`Viator::search_products`] — `POST /partner/search/freetext`, products only.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FreetextProductsArgs {
    /// Content language, sent as `Accept-Language` by the platform (never in the body).
    pub lang: String,
    /// What to search — a city name, typically.
    pub search_term: String,
    /// Price currency (ISO 4217).
    pub currency: String,
    /// Always a single `PRODUCTS` search type with its page.
    pub search_types: Vec<SearchType>,
    /// Optional filters on the products.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub product_filtering: Option<ProductFiltering>,
}

/// One searched entity type and its page.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SearchType {
    /// `PRODUCTS`.
    pub search_type: String,
    /// Page window.
    pub pagination: Pagination,
}

/// A 1-based page window.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Pagination {
    /// First result, 1-based.
    pub start: u32,
    /// Results in the page, `1..=50`.
    pub count: u32,
}

/// Product filters.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProductFiltering {
    /// Average rating range.
    pub rating: RatingRange,
}

/// An inclusive rating range, `0..=5`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RatingRange {
    /// Lowest rating kept.
    pub from: u8,
    /// Highest rating kept.
    pub to: u8,
}

impl FreetextProductsArgs {
    /// Products matching `search_term`, first page of `count` (clamped to `1..=50`).
    ///
    /// `min_rating` of `0` (or above 5) means no rating filter.
    pub fn new(search_term: &str, lang: &str, currency: &str, count: u32, min_rating: u8) -> Self {
        Self {
            lang: lang.to_string(),
            search_term: search_term.trim().to_string(),
            currency: currency.to_string(),
            search_types: vec![SearchType {
                search_type: "PRODUCTS".into(),
                pagination: Pagination {
                    start: 1,
                    count: count.clamp(1, MAX_COUNT),
                },
            }],
            product_filtering: (1..=5).contains(&min_rating).then_some(ProductFiltering {
                rating: RatingRange {
                    from: min_rating,
                    to: 5,
                },
            }),
        }
    }
}

/// A Viator product, normalized for display.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ViatorProduct {
    /// Viator product code.
    pub code: String,
    /// Localized title.
    pub title: String,
    /// Cover image (`https`), the variant closest to 720 px wide.
    pub image_url: Option<String>,
    /// "From" price on viator.com, in [`ViatorProduct::currency`].
    pub price: Option<f64>,
    /// ISO 4217 code of the price.
    pub currency: Option<String>,
    /// Combined average rating, 1–5.
    pub rating: Option<f64>,
    /// Number of reviews behind [`ViatorProduct::rating`].
    pub rating_count: u32,
    /// Duration in minutes — the fixed one, else the lower bound of a range.
    pub duration_minutes: Option<u32>,
    /// Free cancellation offered.
    pub free_cancellation: bool,
    /// viator.com product page, affiliate parameters included — never rewritten.
    pub product_url: String,
}

/// Bundle returned by [`Viator::search_products`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProductsResponse {
    /// Total matches reported by Viator (before the filtering below).
    pub total: u32,
    /// Titled products with a viator.com link.
    pub products: Vec<ViatorProduct>,
}

impl Viator {
    /// Products for a search term via `connectors::call("viator", "search_products", ...)`.
    ///
    /// Untitled products, and those whose link does not point to `viator.com` over `https`, are
    /// dropped.
    pub fn search_products(args: &FreetextProductsArgs) -> SdkResult<ProductsResponse> {
        let raw: Value = connectors::call("viator", "search_products", args)?;
        Ok(parse_products(&raw))
    }
}

/// Parses a freetext search payload. Public for modules that replay recorded responses.
pub fn parse_products(raw: &Value) -> ProductsResponse {
    let products = raw.get("products");
    let total = products
        .and_then(|p| p.get("totalCount"))
        .and_then(Value::as_u64)
        .unwrap_or(0)
        .min(u64::from(u32::MAX)) as u32;
    let products = products
        .and_then(|p| p.get("results"))
        .and_then(Value::as_array)
        .map(|list| list.iter().filter_map(map_product).collect())
        .unwrap_or_default();
    ProductsResponse { total, products }
}

fn map_product(item: &Value) -> Option<ViatorProduct> {
    let code = text(item, "productCode")?;
    let title = text(item, "title")?;
    let product_url = text(item, "productUrl").filter(|url| is_viator_url(url))?;
    let reviews = item.get("reviews");
    let duration = item.get("duration");
    Some(ViatorProduct {
        code,
        title,
        image_url: cover_image(item.get("images")),
        price: item
            .pointer("/pricing/summary/fromPrice")
            .and_then(Value::as_f64)
            .filter(|p| *p > 0.0),
        currency: item
            .pointer("/pricing/currency")
            .and_then(Value::as_str)
            .map(str::to_string),
        rating: reviews
            .and_then(|r| r.get("combinedAverageRating"))
            .and_then(Value::as_f64)
            .filter(|avg| *avg > 0.0),
        rating_count: reviews
            .and_then(|r| r.get("totalReviews"))
            .and_then(Value::as_u64)
            .unwrap_or(0)
            .min(u64::from(u32::MAX)) as u32,
        duration_minutes: duration
            .and_then(|d| {
                d.get("fixedDurationInMinutes")
                    .or_else(|| d.get("variableDurationFromMinutes"))
            })
            .and_then(Value::as_u64)
            .map(|m| m.min(u64::from(u32::MAX)) as u32),
        free_cancellation: item
            .get("flags")
            .and_then(Value::as_array)
            .is_some_and(|flags| {
                flags
                    .iter()
                    .any(|f| f.as_str() == Some("FREE_CANCELLATION"))
            }),
        product_url,
    })
}

fn text(item: &Value, key: &str) -> Option<String> {
    item.get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

/// The cover image (else the first), variant whose width is closest to 720 px.
fn cover_image(images: Option<&Value>) -> Option<String> {
    let images = images?.as_array()?;
    let image = images
        .iter()
        .find(|i| i.get("isCover").and_then(Value::as_bool) == Some(true))
        .or_else(|| images.first())?;
    image
        .get("variants")?
        .as_array()?
        .iter()
        .filter_map(|v| {
            let url = v.get("url")?.as_str()?.trim();
            let width = v.get("width").and_then(Value::as_u64).unwrap_or(0);
            url.starts_with("https://")
                .then_some((width.abs_diff(720), url))
        })
        .min_by_key(|(distance, _)| *distance)
        .map(|(_, url)| url.to_string())
}

/// `https`, and a host that is `viator.com` or one of its subdomains.
///
/// The host check keeps the userinfo trick out (`https://viator.com@elsewhere.example/`).
fn is_viator_url(url: &str) -> bool {
    let Some(rest) = url.strip_prefix("https://") else {
        return false;
    };
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    if authority.contains('@') {
        return false;
    }
    let host = authority
        .split(':')
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    host == "viator.com" || host.ends_with(".viator.com")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn recorded() -> Value {
        json!({
            "products": {
                "totalCount": 3,
                "results": [
                    {
                        "productCode": "5010SYDNEY",
                        "title": "Croisière dans la baie",
                        "images": [
                            { "isCover": false, "variants": [{ "width": 720, "url": "https://cdn.example/other.jpg" }] },
                            { "isCover": true, "variants": [
                                { "width": 200, "url": "https://cdn.example/s.jpg" },
                                { "width": 674, "url": "https://cdn.example/m.jpg" },
                                { "width": 1200, "url": "https://cdn.example/l.jpg" }
                            ]}
                        ],
                        "reviews": { "totalReviews": 812, "combinedAverageRating": 4.7 },
                        "duration": { "variableDurationFromMinutes": 90, "variableDurationToMinutes": 120 },
                        "pricing": { "summary": { "fromPrice": 39.5 }, "currency": "EUR" },
                        "flags": ["LIKELY_TO_SELL_OUT", "FREE_CANCELLATION"],
                        "productUrl": "https://www.viator.com/tours/Nice/Cruise/d478-5010SYDNEY?pid=P1&mcid=42&medium=api"
                    },
                    { "productCode": "NOTITLE", "productUrl": "https://www.viator.com/tours/x" },
                    { "productCode": "EVIL", "title": "Ailleurs", "productUrl": "https://www.viator.com@evil.example/x" }
                ]
            }
        })
    }

    #[test]
    fn a_recorded_payload_keeps_what_the_booklet_shows() {
        let parsed = parse_products(&recorded());
        assert_eq!(parsed.total, 3);
        assert_eq!(parsed.products.len(), 1);
        let product = &parsed.products[0];
        assert_eq!(product.code, "5010SYDNEY");
        assert_eq!(product.title, "Croisière dans la baie");
        assert_eq!(
            product.image_url.as_deref(),
            Some("https://cdn.example/m.jpg")
        );
        assert_eq!(product.price, Some(39.5));
        assert_eq!(product.currency.as_deref(), Some("EUR"));
        assert_eq!(product.rating, Some(4.7));
        assert_eq!(product.rating_count, 812);
        assert_eq!(product.duration_minutes, Some(90));
        assert!(product.free_cancellation);
        assert_eq!(
            product.product_url,
            "https://www.viator.com/tours/Nice/Cruise/d478-5010SYDNEY?pid=P1&mcid=42&medium=api"
        );
    }

    #[test]
    fn only_https_viator_links_pass() {
        for url in [
            "https://www.viator.com/x",
            "https://viator.com/x",
            "https://fr.viator.com/x?pid=a",
        ] {
            assert!(is_viator_url(url), "{url}");
        }
        for url in [
            "http://www.viator.com/x",
            "https://evil-viator.com/x",
            "https://viator.com.evil.example/x",
            "https://www.viator.com@evil.example/x",
            "javascript:alert(1)",
            "",
        ] {
            assert!(!is_viator_url(url), "{url}");
        }
    }

    #[test]
    fn the_body_is_what_viator_expects() {
        let value = serde_json::to_value(FreetextProductsArgs::new(" Nice ", "fr", "EUR", 500, 4))
            .expect("serialize");
        assert_eq!(
            value,
            json!({
                "lang": "fr",
                "searchTerm": "Nice",
                "currency": "EUR",
                "searchTypes": [{ "searchType": "PRODUCTS", "pagination": { "start": 1, "count": 50 } }],
                "productFiltering": { "rating": { "from": 4, "to": 5 } }
            })
        );
    }

    #[test]
    fn no_rating_filter_when_unset() {
        let value = serde_json::to_value(FreetextProductsArgs::new("Nice", "fr", "EUR", 0, 0))
            .expect("serialize");
        assert!(value.get("productFiltering").is_none());
        assert_eq!(value["searchTypes"][0]["pagination"]["count"], 1);
    }
}
