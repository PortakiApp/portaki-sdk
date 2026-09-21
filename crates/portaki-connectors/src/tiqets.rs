//! Tiqets connector (`connector_id = "tiqets"`).
//!
//! Wraps [`portaki_sdk::host::connectors::call`] for the **Content API** of the Tiqets
//! Distributor API — the read-only part a partner key covers ("Content & Availability").
//! Nothing here books or orders: a guest who wants a ticket follows the product's
//! `product_url` to Tiqets' own checkout.
//!
//! The platform pins the host (`api.tiqets.com`) and sends the key as
//! `Authorization: Token <key>`; the module declares only the operation path, `/v2/products`.
//!
//! # Capabilities
//!
//! Requires one of:
//!
//! - `external.tiqets.pool` — Portaki's partner key, counted per workspace and month
//! - `external.tiqets.byok` — the host's own partner key
//!
//! # Affiliate tracking
//!
//! Tiqets documents that `product_url` already carries the partner's tracking code
//! (`?partner=…`) for the key that made the call. This client keeps that URL as returned:
//! rewriting it would move the commission — to Portaki from a BYOK host, or away from
//! Portaki on the pool key.
//!
//! # Attribution
//!
//! Tiqets requires the image credit to be shown wherever an image is shown, and image
//! caches to be refreshed at least every 14 days. [`TiqetsImage::credit`] is kept for that.
//!
//! Reference: <https://developers.tiqets.dev/basics/openapi/content-api/products/search-and-filter-products>

use portaki_sdk::host::connectors;
use portaki_sdk::Result as SdkResult;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Namespace for Tiqets host connector operations.
pub struct Tiqets;

/// Upper bound Tiqets accepts for `max_distance`, in kilometres.
pub const MAX_DISTANCE_KM: u32 = 100;

/// Upper bound Tiqets accepts for `page_size`.
pub const MAX_PAGE_SIZE: u32 = 100;

/// Arguments for [`Tiqets::nearby_products`] — `GET /v2/products` filtered by coordinates.
///
/// Every field is a scalar query parameter: the runtime sends scalar arguments of a `GET` as
/// the query string and drops anything else, so there is no list-valued filter here.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NearbyProductsArgs {
    /// Latitude (WGS-84).
    pub lat: f64,
    /// Longitude (WGS-84).
    pub lng: f64,
    /// Search radius in km, `1..=100` (Tiqets defaults to 5 when absent).
    pub max_distance: u32,
    /// Content language (ISO 639-1). Tiqets falls back to English.
    pub lang: String,
    /// Price currency (ISO 4217).
    pub currency: String,
    /// Results per page, `1..=100`.
    pub page_size: u32,
    /// Only products carrying this Tiqets tag id (see their `/tags` endpoint).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag_id: Option<u64>,
    /// Minimum average review rating, `1..=5`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_rating: Option<u8>,
}

impl NearbyProductsArgs {
    /// Arguments clamped to the ranges Tiqets documents, so a bad setting never becomes a `400`.
    pub fn new(
        lat: f64,
        lng: f64,
        radius_km: u32,
        lang: &str,
        currency: &str,
        page_size: u32,
    ) -> Self {
        Self {
            lat,
            lng,
            max_distance: radius_km.clamp(1, MAX_DISTANCE_KM),
            lang: lang.to_string(),
            currency: currency.to_string(),
            page_size: page_size.clamp(1, MAX_PAGE_SIZE),
            tag_id: None,
            min_rating: None,
        }
    }
}

/// One image of a product, ready to show.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TiqetsImage {
    /// `https` URL — the medium size when present, else large, else small.
    pub url: String,
    /// Alt text in the requested language, when Tiqets has one.
    pub alt: Option<String>,
    /// Image credit. Tiqets requires it to be displayed with the image.
    pub credit: Option<String>,
}

/// A bookable Tiqets product, normalized for display.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TiqetsProduct {
    /// Tiqets product id.
    pub id: String,
    /// Localized title.
    pub title: String,
    /// Localized tagline, HTML stripped.
    pub tagline: Option<String>,
    /// Localized city name.
    pub city: Option<String>,
    /// First usable image.
    pub image: Option<TiqetsImage>,
    /// Retail price on Tiqets.com, in [`TiqetsProduct::currency`].
    pub price: Option<f64>,
    /// ISO 4217 code of the price.
    pub currency: Option<String>,
    /// Average rating, 1–5.
    pub rating: Option<f64>,
    /// Number of ratings behind [`TiqetsProduct::rating`].
    pub rating_count: u32,
    /// Distance from the searched point, in km, as Tiqets reports it.
    pub distance_km: Option<u32>,
    /// Tiqets.com product page, affiliate code included — never rewritten.
    pub product_url: String,
    /// Product latitude.
    pub lat: Option<f64>,
    /// Product longitude.
    pub lng: Option<f64>,
}

/// Bundle returned by [`Tiqets::nearby_products`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NearbyProductsResponse {
    /// Total matches reported by Tiqets (before the filtering below).
    pub total: u32,
    /// Products on sale, with a title and a Tiqets.com link.
    pub products: Vec<TiqetsProduct>,
}

impl Tiqets {
    /// Products around a point via `connectors::call("tiqets", "nearby_products", ...)`.
    ///
    /// Products that are not on sale (`sale_status: unavailable`), untitled, or whose link does
    /// not point to `tiqets.com` over `https` are dropped.
    pub fn nearby_products(args: &NearbyProductsArgs) -> SdkResult<NearbyProductsResponse> {
        let raw: Value = connectors::call("tiqets", "nearby_products", args)?;
        Ok(parse_products(&raw))
    }

    /// Local format check for a BYOK partner key before persistence — no network call.
    ///
    /// A pasted `Token ` prefix is refused: the platform adds it, and the header would carry it
    /// twice.
    pub fn validate_credentials(api_key: &str) -> super::Result<()> {
        let key = api_key.trim();
        if key.is_empty() {
            return Err(super::ConnectorError::InvalidCredentials(
                "tiqets api key is empty".into(),
            ));
        }
        if key.contains(char::is_whitespace) {
            return Err(super::ConnectorError::InvalidCredentials(
                "tiqets api key must be the bare key, without the Token prefix".into(),
            ));
        }
        Ok(())
    }
}

/// Parses a `GET /v2/products` payload. Public for modules that replay recorded responses.
pub fn parse_products(raw: &Value) -> NearbyProductsResponse {
    let total = raw
        .pointer("/pagination/total")
        .and_then(Value::as_u64)
        .unwrap_or(0)
        .min(u64::from(u32::MAX)) as u32;
    let products = raw
        .get("products")
        .and_then(Value::as_array)
        .map(|list| list.iter().filter_map(map_product).collect())
        .unwrap_or_default();
    NearbyProductsResponse { total, products }
}

fn map_product(item: &Value) -> Option<TiqetsProduct> {
    if item.get("sale_status").and_then(Value::as_str) == Some("unavailable") {
        return None;
    }
    let id = item.get("id").and_then(|v| {
        v.as_str()
            .map(|s| s.trim().to_string())
            .or_else(|| v.as_u64().map(|n| n.to_string()))
    })?;
    if id.is_empty() {
        return None;
    }
    let title = text(item, "title")?;
    let product_url = item
        .get("product_url")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|url| is_tiqets_url(url))?
        .to_string();
    let ratings = item.get("ratings");
    Some(TiqetsProduct {
        id,
        title,
        tagline: text(item, "tagline")
            .map(|t| strip_html(&t))
            .filter(|t| !t.is_empty()),
        city: text(item, "city_name"),
        image: first_image(item.get("images")),
        price: item.get("price").and_then(Value::as_f64),
        currency: text(item, "currency"),
        rating: ratings
            .and_then(|r| r.get("average"))
            .and_then(Value::as_f64)
            .filter(|avg| *avg > 0.0),
        rating_count: ratings
            .and_then(|r| r.get("total"))
            .and_then(Value::as_u64)
            .unwrap_or(0)
            .min(u64::from(u32::MAX)) as u32,
        distance_km: item
            .get("distance")
            .and_then(Value::as_u64)
            .map(|d| d.min(u64::from(u32::MAX)) as u32),
        product_url,
        lat: item.pointer("/geolocation/lat").and_then(Value::as_f64),
        lng: item.pointer("/geolocation/lng").and_then(Value::as_f64),
    })
}

fn text(item: &Value, key: &str) -> Option<String> {
    item.get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

fn first_image(images: Option<&Value>) -> Option<TiqetsImage> {
    images?.as_array()?.iter().find_map(|image| {
        let url = ["medium", "large", "small", "extra_large"]
            .iter()
            .filter_map(|size| image.get(*size).and_then(Value::as_str))
            .map(str::trim)
            .find(|url| url.starts_with("https://"))?;
        Some(TiqetsImage {
            url: url.to_string(),
            alt: text(image, "alt_text"),
            credit: text(image, "credit"),
        })
    })
}

/// `https`, and a host that is `tiqets.com` or one of its subdomains.
///
/// The host check keeps the userinfo trick out (`https://tiqets.com@elsewhere.example/`).
fn is_tiqets_url(url: &str) -> bool {
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
    host == "tiqets.com" || host.ends_with(".tiqets.com")
}

/// Removes the basic markup Tiqets may put in text fields (`<br>`, `<p>`, `<b>`, …).
fn strip_html(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut in_tag = false;
    for c in value.chars() {
        match c {
            '<' => in_tag = true,
            '>' if in_tag => {
                in_tag = false;
                out.push(' ');
            }
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn recorded() -> Value {
        json!({
            "success": true,
            "pagination": { "total": 3, "page": 1, "page_size": 12 },
            "products": [
                {
                    "id": "974079",
                    "title": "Musée Van Gogh : billet d'entrée",
                    "tagline": "<p>Le plus grand <b>ensemble</b> au monde</p>",
                    "city_name": "Amsterdam",
                    "images": [{
                        "small": "https://cdn.example/s.jpg",
                        "medium": "https://cdn.example/m.jpg",
                        "alt_text": "Façade",
                        "credit": "Photo : Studio"
                    }],
                    "geolocation": { "lat": 52.3584, "lng": 4.8811 },
                    "distance": 2,
                    "ratings": { "average": 4.6, "total": 18234 },
                    "price": 22.0,
                    "currency": "EUR",
                    "product_url": "https://www.tiqets.com/fr/p974079/?partner=portaki",
                    "sale_status": "available"
                },
                {
                    "id": "1006522",
                    "title": "Hors saison",
                    "product_url": "https://www.tiqets.com/fr/p1006522/?partner=portaki",
                    "sale_status": "unavailable"
                },
                {
                    "id": "666",
                    "title": "Ailleurs",
                    "product_url": "https://www.tiqets.com@evil.example/p666"
                }
            ]
        })
    }

    #[test]
    fn a_recorded_payload_keeps_what_the_booklet_shows() {
        let parsed = parse_products(&recorded());
        assert_eq!(parsed.total, 3);
        assert_eq!(parsed.products.len(), 1);
        let product = &parsed.products[0];
        assert_eq!(product.id, "974079");
        assert_eq!(product.title, "Musée Van Gogh : billet d'entrée");
        assert_eq!(
            product.tagline.as_deref(),
            Some("Le plus grand ensemble au monde")
        );
        assert_eq!(product.city.as_deref(), Some("Amsterdam"));
        let image = product.image.as_ref().expect("image");
        assert_eq!(image.url, "https://cdn.example/m.jpg");
        assert_eq!(image.credit.as_deref(), Some("Photo : Studio"));
        assert_eq!(product.price, Some(22.0));
        assert_eq!(product.currency.as_deref(), Some("EUR"));
        assert_eq!(product.rating, Some(4.6));
        assert_eq!(product.rating_count, 18234);
        assert_eq!(product.distance_km, Some(2));
        assert_eq!(product.lat, Some(52.3584));
    }

    #[test]
    fn the_affiliate_link_is_kept_as_tiqets_returned_it() {
        let parsed = parse_products(&recorded());
        assert_eq!(
            parsed.products[0].product_url,
            "https://www.tiqets.com/fr/p974079/?partner=portaki"
        );
    }

    #[test]
    fn only_https_tiqets_links_pass() {
        for url in [
            "https://www.tiqets.com/x",
            "https://tiqets.com/x",
            "https://fr.tiqets.com/x?partner=a",
        ] {
            assert!(is_tiqets_url(url), "{url}");
        }
        for url in [
            "http://www.tiqets.com/x",
            "https://evil-tiqets.com/x",
            "https://tiqets.com.evil.example/x",
            "https://www.tiqets.com@evil.example/x",
            "javascript:alert(1)",
            "",
        ] {
            assert!(!is_tiqets_url(url), "{url}");
        }
    }

    #[test]
    fn a_product_without_title_or_link_is_dropped() {
        let parsed = parse_products(&json!({
            "products": [
                { "id": "1", "product_url": "https://www.tiqets.com/p1" },
                { "id": "2", "title": "Sans lien" },
                { "id": "", "title": "Sans id", "product_url": "https://www.tiqets.com/p3" }
            ]
        }));
        assert!(parsed.products.is_empty());
        assert_eq!(parsed.total, 0);
    }

    #[test]
    fn arguments_are_clamped_to_the_documented_ranges() {
        let args = NearbyProductsArgs::new(43.55, 7.01, 500, "fr", "EUR", 0);
        assert_eq!(args.max_distance, MAX_DISTANCE_KM);
        assert_eq!(args.page_size, 1);
        let args = NearbyProductsArgs::new(43.55, 7.01, 0, "fr", "EUR", 1000);
        assert_eq!(args.max_distance, 1);
        assert_eq!(args.page_size, MAX_PAGE_SIZE);
    }

    #[test]
    fn optional_filters_stay_out_of_the_query_when_unset() {
        let value = serde_json::to_value(NearbyProductsArgs::new(1.0, 2.0, 10, "en", "EUR", 12))
            .expect("serialize");
        assert!(value.get("tag_id").is_none());
        assert!(value.get("min_rating").is_none());
        assert_eq!(value["max_distance"], 10);
    }

    #[test]
    fn a_bare_key_validates_and_a_prefixed_one_does_not() {
        assert!(Tiqets::validate_credentials("0123456789abcdef").is_ok());
        assert!(Tiqets::validate_credentials("  ").is_err());
        assert!(Tiqets::validate_credentials("Token 0123456789abcdef").is_err());
    }
}
