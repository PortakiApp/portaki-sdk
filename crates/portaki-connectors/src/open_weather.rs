//! OpenWeather built-in connector (`open-weather`).

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
        connectors::call("open-weather", "current", args)
    }

    /// Multi-day forecast.
    pub fn forecast(args: &ForecastArgs) -> SdkResult<ForecastResponse> {
        connectors::call("open-weather", "forecast", args)
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
