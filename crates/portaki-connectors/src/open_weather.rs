//! OpenWeather built-in connector (`open-weather`).
//!
//! Typed client used by modules. Parses raw OpenWeather JSON returned by host egress.

use portaki_sdk::host::connectors;
use portaki_sdk::Result as SdkResult;
use serde::{Deserialize, Serialize};

/// OpenWeather connector namespace.
pub struct OpenWeather;

/// Current weather request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentArgs {
    /// Latitude.
    pub lat: f64,
    /// Longitude.
    pub lng: f64,
}

/// Forecast request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForecastArgs {
    /// Latitude.
    pub lat: f64,
    /// Longitude.
    pub lng: f64,
    /// Number of days (max 16 depending on plan).
    pub days: u8,
}

/// Current weather snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentWeather {
    /// Temperature in Celsius.
    pub temp_c: f64,
    /// Weather condition code.
    pub condition: String,
    /// Humidity percent.
    pub humidity: u8,
}

/// Daily forecast row.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForecastDay {
    /// ISO date (`YYYY-MM-DD`).
    pub date: String,
    /// Min temperature Celsius.
    pub min_c: f64,
    /// Max temperature Celsius.
    pub max_c: f64,
    /// Condition id/summary.
    pub condition: String,
}

/// Forecast response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForecastResponse {
    /// Daily rows.
    pub days: Vec<ForecastDay>,
}

impl OpenWeather {
    /// Current weather for coordinates.
    pub fn current(args: &CurrentArgs) -> SdkResult<CurrentWeather> {
        let raw: serde_json::Value = connectors::call("open-weather", "current", args)?;
        parse_current(&raw)
    }

    /// Multi-day forecast.
    pub fn forecast(args: &ForecastArgs) -> SdkResult<ForecastResponse> {
        let raw: serde_json::Value = connectors::call("open-weather", "forecast", args)?;
        parse_forecast(&raw, args.days)
    }

    /// Historical archive (simplified).
    pub fn historical(args: &ForecastArgs) -> SdkResult<serde_json::Value> {
        connectors::call("open-weather", "historical", args)
    }

    /// Validates API key (stub).
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
        }
        rows.push(ForecastDay {
            date,
            min_c: min,
            max_c: max,
            condition,
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
    }
}
