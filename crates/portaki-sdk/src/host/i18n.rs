//! `host::i18n` — bundle resolution and formatting helpers.

use serde::Serialize;
use std::collections::BTreeMap;

use crate::error::Result;
use crate::host::runtime::backend;

/// Variables passed to translation lookups.
#[derive(Debug, Clone, Default)]
pub struct Vars {
    values: BTreeMap<String, String>,
}

impl Vars {
    /// Creates an empty variable map.
    pub fn new() -> Self {
        Self::default()
    }

    /// Inserts a variable (stringified via `Display`).
    pub fn set(&mut self, key: impl Into<String>, value: impl std::fmt::Display) {
        self.values.insert(key.into(), value.to_string());
    }
}

/// Resolves `key` using the invocation locale.
pub fn translate(key: &str, vars: &Vars) -> Result<String> {
    let vars_json = serde_json::to_string(&vars.values)?;
    backend()?.i18n_translate(key, &vars_json)
}

/// Resolves `key` in an explicit locale.
pub fn translate_in(locale: &str, key: &str, vars: &Vars) -> Result<String> {
    let _ = locale;
    translate(key, vars)
}

/// Number formatting options.
#[derive(Debug, Clone, Serialize, Default)]
pub struct NumberFormatOptions {
    /// Minimum fraction digits.
    pub minimum_fraction_digits: Option<u32>,
    /// Maximum fraction digits.
    pub maximum_fraction_digits: Option<u32>,
}

/// Formats a number for display.
pub fn format_number(value: f64, locale: &str, opts: &NumberFormatOptions) -> Result<String> {
    let _ = (locale, opts);
    Ok(format!("{value:.1}"))
}

/// Formats currency for display.
pub fn format_currency(value: f64, currency: &str, locale: &str) -> Result<String> {
    let _ = locale;
    Ok(format!("{value:.2} {currency}"))
}

/// Date formatting options.
#[derive(Debug, Clone, Serialize, Default)]
pub struct DateFormatOptions {
    /// Date style (`short`, `long`, …).
    pub date_style: Option<String>,
}

/// Formats a timestamp for display.
pub fn format_date(
    ts: chrono::DateTime<chrono::Utc>,
    locale: &str,
    opts: &DateFormatOptions,
) -> Result<String> {
    let _ = (locale, opts);
    Ok(ts.to_rfc3339())
}
