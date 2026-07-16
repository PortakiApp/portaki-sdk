//! OpenWeather connector (`connector_id = "open-weather"`).
//!
//! Wraps [`portaki_sdk::host::connectors::call`] for the built-in OpenWeather
//! integration. Raw provider JSON from host egress is normalized into
//! [`CurrentWeather`] and [`ForecastResponse`]; [`OpenWeather::historical`]
//! returns unparsed JSON.
//!
//! # Capabilities
//!
//! Requires one of:
//!
//! - `external.open-weather.pool` — platform-managed API key
//! - `external.open-weather.byok` — property-supplied API key
//!
//! # Example
//!
//! ```no_run
//! use portaki_connectors::open_weather::{CurrentArgs, ForecastArgs, OpenWeather};
//!
//! let current = OpenWeather::current(&CurrentArgs {
//!     lat: 43.5513,
//!     lng: 7.0128,
//! })?;
//!
//! let forecast = OpenWeather::forecast(&ForecastArgs {
//!     lat: 43.5513,
//!     lng: 7.0128,
//!     days: 5,
//! })?;
//! # Ok::<(), portaki_sdk::PortakiError>(())
//! ```
//!
//! # Test stubbing
//!
//! Register canned JSON on a `portaki_test_utils::MockContext` (see that crate's docs):
//!
//! ```ignore
//! use portaki_test_utils::MockContext;
//!
//! MockContext::guest()
//!     .with_connector_response(
//!         "open-weather",
//!         "current",
//!         r#"{"main":{"temp":21.5,"humidity":55},"weather":[{"main":"Clear"}]}"#,
//!     )
//!     .run(|_ctx| {
//!         // OpenWeather::current(...) reads the stub above.
//!     });
//! ```

use portaki_sdk::host::connectors;
use portaki_sdk::Result as SdkResult;
use serde::{Deserialize, Serialize};

/// Namespace for OpenWeather host connector operations.
///
/// Zero-sized type; all methods are static. Thread-safe because each call
/// delegates to the thread-local host backend installed by
/// [`portaki_sdk::host::with_host`].
pub struct OpenWeather;

/// Arguments for [`OpenWeather::current`].
///
/// Coordinates use WGS-84 decimal degrees, matching OpenWeather `lat`/`lon`
/// query parameters assembled by the gateway.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentArgs {
    /// Latitude in decimal degrees (`-90.0` … `90.0`).
    pub lat: f64,
    /// Longitude in decimal degrees (`-180.0` … `180.0`).
    pub lng: f64,
}

/// Arguments for [`OpenWeather::forecast`] and [`OpenWeather::historical`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForecastArgs {
    /// Latitude in decimal degrees.
    pub lat: f64,
    /// Longitude in decimal degrees.
    pub lng: f64,
    /// Requested number of daily rows.
    ///
    /// Parsed forecast caps aggregation at 5 days regardless of plan; the host
    /// may return up to 16 depending on subscription.
    pub days: u8,
}

/// Normalized current-conditions snapshot.
///
/// Produced by [`OpenWeather::current`] from `/data/2.5/weather` JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentWeather {
    /// Air temperature in degrees Celsius (`main.temp`).
    pub temp_c: f64,
    /// Lowercased `weather[0].main` label (e.g. `"clear"`, `"clouds"`).
    ///
    /// Defaults to `"unknown"` when the field is absent.
    pub condition: String,
    /// Relative humidity percent (`main.humidity`, `0`–`100`).
    pub humidity: u8,
    /// Wind speed in meters per second (`wind.speed`), when present.
    pub wind_speed_ms: Option<f64>,
}

/// Single aggregated forecast day.
///
/// Built by rolling up 3-hour `/data/2.5/forecast` list items that share the
/// same calendar date (`dt_txt` prefix `YYYY-MM-DD`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForecastDay {
    /// Calendar date in `YYYY-MM-DD` form (UTC date extracted from `dt_txt`).
    pub date: String,
    /// Minimum `main.temp_min` across slots for this date.
    pub min_c: f64,
    /// Maximum `main.temp_max` across slots for this date.
    pub max_c: f64,
    /// Lowercased `weather[0].main` from the last slot processed for the date.
    pub condition: String,
    /// Peak precipitation probability percent (`pop` × 100), when present.
    pub precip_chance_pct: Option<u8>,
}

/// Multi-day forecast bundle returned by [`OpenWeather::forecast`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForecastResponse {
    /// Daily rows, length ≤ `min(requested days, 5)` after local aggregation.
    pub days: Vec<ForecastDay>,
}

impl OpenWeather {
    /// Fetches current weather for `args` via `connectors::call("open-weather", "current", ...)`.
    ///
    /// # Errors
    ///
    /// Returns [`portaki_sdk::PortakiError`] when the host backend is missing,
    /// egress fails, or the response JSON cannot be parsed.
    pub fn current(args: &CurrentArgs) -> SdkResult<CurrentWeather> {
        let raw: serde_json::Value = connectors::call("open-weather", "current", args)?;
        parse_current(&raw)
    }

    /// Fetches a multi-day forecast via `connectors::call("open-weather", "forecast", ...)`.
    ///
    /// Aggregates the provider list payload into at most five [`ForecastDay`] rows.
    ///
    /// # Errors
    ///
    /// Same as [`Self::current`].
    pub fn forecast(args: &ForecastArgs) -> SdkResult<ForecastResponse> {
        let raw: serde_json::Value = connectors::call("open-weather", "forecast", args)?;
        parse_forecast(&raw, args.days)
    }

    /// Fetches historical archive data as raw JSON.
    ///
    /// No response normalization — callers own schema mapping. Dispatches
    /// `connectors::call("open-weather", "historical", args)`.
    pub fn historical(args: &ForecastArgs) -> SdkResult<serde_json::Value> {
        connectors::call("open-weather", "historical", args)
    }

    /// Validates a BYOK API key before persistence (local stub).
    ///
    /// Accepts any non-empty trimmed string. Does not call OpenWeather.
    ///
    /// # Errors
    ///
    /// Returns [`super::ConnectorError::InvalidCredentials`] when `api_key` is
    /// empty or whitespace-only.
    pub fn validate_credentials(api_key: &str) -> super::Result<()> {
        if api_key.trim().is_empty() {
            return Err(super::ConnectorError::InvalidCredentials(
                "open-weather api key is empty".into(),
            ));
        }
        Ok(())
    }
}

fn parse_current(raw: &serde_json::Value) -> SdkResult<CurrentWeather> {
    let condition = raw
        .pointer("/weather/0/main")
        .and_then(|value| value.as_str())
        .unwrap_or("unknown")
        .to_ascii_lowercase();
    Ok(CurrentWeather {
        temp_c: raw
            .pointer("/main/temp")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0),
        humidity: raw
            .pointer("/main/humidity")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u8,
        wind_speed_ms: raw.pointer("/wind/speed").and_then(|v| v.as_f64()),
        condition,
    })
}

fn parse_forecast(raw: &serde_json::Value, days: u8) -> SdkResult<ForecastResponse> {
    let capped_days = days.clamp(1, 5);
    let mut by_date: std::collections::BTreeMap<String, Vec<&serde_json::Value>> =
        std::collections::BTreeMap::new();
    if let Some(list) = raw.get("list").and_then(|v| v.as_array()) {
        for item in list {
            let date = item
                .get("dt_txt")
                .and_then(|v| v.as_str())
                .and_then(|text| text.get(0..10))
                .unwrap_or("");
            if date.len() == 10 {
                by_date.entry(date.to_string()).or_default().push(item);
            }
        }
    }
    let mut rows = Vec::new();
    for (date, items) in by_date {
        if rows.len() >= capped_days as usize {
            break;
        }
        let mut min = f64::MAX;
        let mut max = f64::MIN;
        let mut condition = String::from("clouds");
        let mut max_pop: Option<f64> = None;
        for item in items {
            let item_min = item
                .pointer("/main/temp_min")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            let item_max = item
                .pointer("/main/temp_max")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            min = min.min(item_min);
            max = max.max(item_max);
            if let Some(main) = item.pointer("/weather/0/main").and_then(|v| v.as_str()) {
                condition = main.to_ascii_lowercase();
            }
            if let Some(pop) = item.get("pop").and_then(|v| v.as_f64()) {
                max_pop = Some(max_pop.map_or(pop, |current| current.max(pop)));
            }
        }
        rows.push(ForecastDay {
            date,
            min_c: min,
            max_c: max,
            condition,
            precip_chance_pct: max_pop.map(|pop| (pop.clamp(0.0, 1.0) * 100.0).round() as u8),
        });
    }
    Ok(ForecastResponse { days: rows })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_current_from_openweather_payload() {
        let raw = serde_json::json!({
            "main": { "temp": 18.5, "humidity": 70 },
            "weather": [{ "main": "Clear" }]
        });
        let parsed = parse_current(&raw).expect("parse");
        assert_eq!(parsed.temp_c, 18.5);
        assert_eq!(parsed.humidity, 70);
        assert_eq!(parsed.condition, "clear");
        assert!(parsed.wind_speed_ms.is_none());
    }

    #[test]
    fn parse_forecast_aggregates_precip_chance() {
        let raw = serde_json::json!({
            "list": [
                {
                    "dt_txt": "2026-07-17 09:00:00",
                    "main": { "temp_min": 18.0, "temp_max": 22.0 },
                    "weather": [{ "main": "Rain" }],
                    "pop": 0.4
                },
                {
                    "dt_txt": "2026-07-17 12:00:00",
                    "main": { "temp_min": 19.0, "temp_max": 24.0 },
                    "weather": [{ "main": "Clouds" }],
                    "pop": 0.75
                }
            ]
        });
        let parsed = parse_forecast(&raw, 5).expect("parse");
        assert_eq!(parsed.days.len(), 1);
        assert_eq!(parsed.days[0].date, "2026-07-17");
        assert_eq!(parsed.days[0].precip_chance_pct, Some(75));
        assert_eq!(parsed.days[0].condition, "clouds");
    }
}
