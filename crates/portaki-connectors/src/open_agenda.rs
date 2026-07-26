//! OpenAgenda connector (`connector_id = "open-agenda"`).
//!
//! Wraps [`portaki_sdk::host::connectors::call`] for the OpenAgenda public API
//! (cross-agenda event listing). Credentials are injected by the gateway as
//! `?key=` (`query_key` auth).
//!
//! # Capabilities
//!
//! Requires one of:
//!
//! - `external.open-agenda.pool` — platform-managed API key
//! - `external.open-agenda.byok` — property-supplied API key
//!
//! # Endpoint
//!
//! `GET /v2/events` — experimental transverse listing with the same filters as
//! agenda-scoped event reads (`geo`, `relative`, `size`, `monolingual`, …).

use portaki_sdk::host::connectors;
use portaki_sdk::Result as SdkResult;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Namespace for OpenAgenda host connector operations.
pub struct OpenAgenda;

/// Arguments for [`OpenAgenda::nearby_events`].
///
/// Coordinates use WGS-84 decimal degrees. `radius_km` is converted to a
/// bounding box by the caller (or via [`bbox_from_radius`]) before dispatch —
/// OpenAgenda expects `geo[northEast|southWest][lat|lng]` query keys.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NearbyEventsArgs {
    /// North-east corner latitude.
    #[serde(rename = "geo[northEast][lat]")]
    pub geo_ne_lat: f64,
    /// North-east corner longitude.
    #[serde(rename = "geo[northEast][lng]")]
    pub geo_ne_lng: f64,
    /// South-west corner latitude.
    #[serde(rename = "geo[southWest][lat]")]
    pub geo_sw_lat: f64,
    /// South-west corner longitude.
    #[serde(rename = "geo[southWest][lng]")]
    pub geo_sw_lng: f64,
    /// Relative timing filter (`upcoming`, `current`, …).
    #[serde(rename = "relative[]")]
    pub relative: String,
    /// Page size (OpenAgenda max 300).
    pub size: u32,
    /// Prefer a single language for multilingual fields (`fr`, `en`, …).
    pub monolingual: String,
}

/// Normalized nearby event row.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NearbyEvent {
    /// OpenAgenda event uid (stringified).
    pub id: String,
    /// Display title (already monolingual when requested).
    pub title: String,
    /// Venue / city label when available.
    pub place: String,
    /// Next / first timing begin instant (ISO-8601), when present.
    pub starts_at: Option<String>,
    /// Canonical or slug-based public URL, when present.
    pub url: Option<String>,
    /// Venue latitude.
    pub lat: Option<f64>,
    /// Venue longitude.
    pub lng: Option<f64>,
}

/// Bundle returned by [`OpenAgenda::nearby_events`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NearbyEventsResponse {
    /// Total matches reported by the provider.
    pub total: u32,
    /// Normalized event rows for this page.
    pub events: Vec<NearbyEvent>,
}

impl OpenAgenda {
    /// Fetches upcoming / current events in a bounding box via
    /// `connectors::call("open-agenda", "nearby_events", ...)`.
    pub fn nearby_events(args: &NearbyEventsArgs) -> SdkResult<NearbyEventsResponse> {
        let raw: Value = connectors::call("open-agenda", "nearby_events", args)?;
        parse_nearby_events(&raw)
    }

    /// Local format check for a BYOK API key before persistence.
    pub fn validate_credentials(api_key: &str) -> super::Result<()> {
        if api_key.trim().is_empty() {
            return Err(super::ConnectorError::InvalidCredentials(
                "open-agenda api key is empty".into(),
            ));
        }
        Ok(())
    }
}

/// Builds a WGS-84 bounding box centered on `(lat, lng)` with approximate
/// radius `radius_km`.
pub fn bbox_from_radius(lat: f64, lng: f64, radius_km: f64) -> (f64, f64, f64, f64) {
    let radius = radius_km.max(0.5);
    let km_per_deg_lat = 111.32_f64;
    let lat_rad = lat.to_radians();
    let km_per_deg_lng = (km_per_deg_lat * lat_rad.cos()).abs().max(0.01);
    let d_lat = radius / km_per_deg_lat;
    let d_lng = radius / km_per_deg_lng;
    let ne_lat = (lat + d_lat).clamp(-90.0, 90.0);
    let sw_lat = (lat - d_lat).clamp(-90.0, 90.0);
    let ne_lng = (lng + d_lng).clamp(-180.0, 180.0);
    let sw_lng = (lng - d_lng).clamp(-180.0, 180.0);
    (ne_lat, ne_lng, sw_lat, sw_lng)
}

fn parse_nearby_events(raw: &Value) -> SdkResult<NearbyEventsResponse> {
    let total = raw
        .get("total")
        .and_then(Value::as_u64)
        .unwrap_or(0)
        .min(u64::from(u32::MAX)) as u32;
    let mut events = Vec::new();
    if let Some(list) = raw.get("events").and_then(Value::as_array) {
        for item in list {
            if let Some(event) = map_event(item) {
                events.push(event);
            }
        }
    }
    Ok(NearbyEventsResponse { total, events })
}

fn map_event(item: &Value) -> Option<NearbyEvent> {
    let uid = item
        .get("uid")
        .and_then(|v| {
            v.as_u64()
                .map(|n| n.to_string())
                .or_else(|| v.as_str().map(|s| s.trim().to_string()))
        })
        .filter(|s| !s.is_empty())?;
    let title = localized_text(item.get("title")).filter(|s| !s.is_empty())?;
    let place = place_label(item.get("location")).unwrap_or_default();
    let starts_at = timing_begin(item);
    let url = public_url(item);
    let lat = item
        .pointer("/location/latitude")
        .and_then(Value::as_f64)
        .or_else(|| item.pointer("/location/lat").and_then(Value::as_f64));
    let lng = item
        .pointer("/location/longitude")
        .and_then(Value::as_f64)
        .or_else(|| item.pointer("/location/lng").and_then(Value::as_f64));
    Some(NearbyEvent {
        id: format!("oa-{uid}"),
        title,
        place,
        starts_at,
        url,
        lat,
        lng,
    })
}

fn localized_text(value: Option<&Value>) -> Option<String> {
    let value = value?;
    if let Some(s) = value.as_str() {
        let trimmed = s.trim();
        return if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        };
    }
    if let Some(map) = value.as_object() {
        for key in ["fr", "en"] {
            if let Some(s) = map.get(key).and_then(Value::as_str) {
                let trimmed = s.trim();
                if !trimmed.is_empty() {
                    return Some(trimmed.to_string());
                }
            }
        }
        for value in map.values() {
            if let Some(s) = value.as_str() {
                let trimmed = s.trim();
                if !trimmed.is_empty() {
                    return Some(trimmed.to_string());
                }
            }
        }
    }
    None
}

fn place_label(location: Option<&Value>) -> Option<String> {
    let location = location?;
    let name = localized_text(location.get("name"))
        .or_else(|| {
            location
                .get("name")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
        })
        .unwrap_or_default();
    let city = location
        .get("city")
        .and_then(Value::as_str)
        .or_else(|| location.get("adminLevel4").and_then(Value::as_str))
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("");
    let label = match (name.is_empty(), city.is_empty()) {
        (false, false) if !name.contains(city) => format!("{name}, {city}"),
        (false, _) => name,
        (true, false) => city.to_string(),
        (true, true) => String::new(),
    };
    if label.is_empty() {
        None
    } else {
        Some(label)
    }
}

fn timing_begin(item: &Value) -> Option<String> {
    for path in [
        "/nextTiming/begin",
        "/firstTiming/begin",
        "/timings/0/begin",
        "/lastTiming/begin",
    ] {
        if let Some(s) = item.pointer(path).and_then(Value::as_str) {
            let trimmed = s.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }
    None
}

fn public_url(item: &Value) -> Option<String> {
    for key in ["canonicalUrl", "canonicalurl", "url"] {
        if let Some(s) = item.get(key).and_then(Value::as_str) {
            let trimmed = s.trim();
            if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
                return Some(trimmed.to_string());
            }
        }
    }
    let slug = item.get("slug").and_then(Value::as_str).map(str::trim)?;
    if slug.is_empty() {
        return None;
    }
    Some(format!("https://openagenda.com/events/{slug}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn bbox_is_symmetric_around_center() {
        let (ne_lat, ne_lng, sw_lat, sw_lng) = bbox_from_radius(43.55, 7.01, 10.0);
        assert!(ne_lat > 43.55);
        assert!(sw_lat < 43.55);
        assert!(ne_lng > 7.01);
        assert!(sw_lng < 7.01);
    }

    #[test]
    fn parse_nearby_events_maps_openagenda_payload() {
        let raw = json!({
            "total": 1,
            "events": [{
                "uid": 56158955,
                "title": "Concert jazz",
                "location": {
                    "name": "Théâtre de la Mer",
                    "city": "Cannes",
                    "latitude": 43.55,
                    "longitude": 7.01
                },
                "nextTiming": { "begin": "2099-07-25T18:00:00.000Z" },
                "canonicalUrl": "https://openagenda.com/events/concert-jazz"
            }]
        });
        let parsed = parse_nearby_events(&raw).expect("parse");
        assert_eq!(parsed.total, 1);
        assert_eq!(parsed.events.len(), 1);
        assert_eq!(parsed.events[0].id, "oa-56158955");
        assert_eq!(parsed.events[0].title, "Concert jazz");
        assert_eq!(parsed.events[0].place, "Théâtre de la Mer, Cannes");
        assert_eq!(
            parsed.events[0].starts_at.as_deref(),
            Some("2099-07-25T18:00:00.000Z")
        );
        assert_eq!(parsed.events[0].lat, Some(43.55));
        assert_eq!(parsed.events[0].lng, Some(7.01));
    }

    #[test]
    fn parse_reads_localized_title_object() {
        let raw = json!({
            "total": 1,
            "events": [{
                "uid": "99",
                "title": { "fr": "Marché", "en": "Market" },
                "location": { "city": "Antibes" }
            }]
        });
        let parsed = parse_nearby_events(&raw).expect("parse");
        assert_eq!(parsed.events[0].title, "Marché");
        assert_eq!(parsed.events[0].place, "Antibes");
    }
}
