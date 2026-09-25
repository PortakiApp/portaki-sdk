//! `statsSummary` — the tile a `property-stats-card` surface shows in the property statistics.
//!
//! A tile is a typed summary, not an SDUI tree. The platform asks with
//! [`StatsSummaryArgs`] (`key` = the surface `pathSegment`), caches the answer 15 minutes and
//! drops it on any event the module emits for that property. The detail behind the tile is the
//! host surface of type `property-stats-detail` whose id is the same `pathSegment`, rendered with
//! `input.periodDays`. Name and icon of the tile come from the manifest. Schema:
//! `contracts/stats-summary.v1.json`.
//!
//! ```
//! use portaki_sdk::contracts::i18n::I18nText;
//! use portaki_sdk::contracts::stats::{self, AttentionLevel, TrendDirection};
//!
//! let tile = stats::summary("5", I18nText::new("signalements", "reports"))
//!     .attention(AttentionLevel::Action, I18nText::new("2 en cours", "2 open"))
//!     .trend("+2", TrendDirection::Up, false);
//! let wire = serde_json::to_value(&tile).unwrap();
//! assert_eq!(wire["attention"]["level"], "action");
//! assert_eq!(wire["trend"]["direction"], "up");
//! ```

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::contracts::i18n::I18nText;
use crate::ids::OperationName;

/// Query name (`statsSummary`).
pub const STATS_SUMMARY: OperationName = OperationName::new("statsSummary");

/// Args of [`STATS_SUMMARY`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatsSummaryArgs {
    /// The property.
    pub property_id: Uuid,
    /// Window in days: 30, 90 or 365.
    pub period: u32,
    /// Which tile: the `pathSegment` of the `property-stats-card` surface.
    pub key: String,
}

impl StatsSummaryArgs {
    /// The window asked for, bounded to 30, 90 or 365 days (30 for anything else).
    pub fn period(&self) -> Period {
        Period::from_days(u64::from(self.period))
    }
}

/// The window of a statistic: the tile's `period`, the detail surface's `input.periodDays` —
/// [`StatsSummaryArgs::period`], [`Context::stats_period`](crate::context::Context::stats_period).
///
/// ```
/// use portaki_sdk::contracts::stats::Period;
///
/// assert_eq!(Period::from_days(90).days(), 90);
/// assert_eq!(Period::from_days(7), Period::Days30); // anything else is the default
/// assert_eq!(Period::Days365.window("fr"), "sur 12 mois");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Period {
    /// 30 days — the default.
    #[default]
    Days30,
    /// 90 days.
    Days90,
    /// 365 days, « 12 months ».
    Days365,
}

impl Period {
    /// 90 and 365 are themselves; anything else is 30.
    pub fn from_days(days: u64) -> Self {
        match days {
            90 => Self::Days90,
            365 => Self::Days365,
            _ => Self::Days30,
        }
    }

    /// 30, 90 or 365 — also the suffix of per-period i18n keys (`stats.tile.90`).
    pub const fn days(self) -> u32 {
        match self {
            Self::Days30 => 30,
            Self::Days90 => 90,
            Self::Days365 => 365,
        }
    }

    /// The start of the window ending at `now`.
    pub fn since(self, now: DateTime<Utc>) -> DateTime<Utc> {
        now - Duration::days(i64::from(self.days()))
    }

    /// The window as a note under a figure: `sur 90 jours`, `over 12 months` — in `lang`
    /// (`en`, `fr`, `es`, `de`, `it`, `nl`; English otherwise).
    pub fn window(self, lang: &str) -> String {
        let [days, months] = WINDOW[crate::host::time::column(lang)];
        match self {
            Self::Days365 => months.replace("{}", "12"),
            other => days.replace("{}", &other.days().to_string()),
        }
    }
}

/// `over {} days`, `over {} months`, per language of [`crate::host::time::LANGUAGES`].
const WINDOW: [[&str; 2]; 6] = [
    ["over {} days", "over {} months"],
    ["sur {} jours", "sur {} mois"],
    ["en {} días", "en {} meses"],
    ["in {} Tagen", "in {} Monaten"],
    ["in {} giorni", "in {} mesi"],
    ["in {} dagen", "in {} maanden"],
];

/// Answer of [`STATS_SUMMARY`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatsSummary {
    /// The figure, as displayed (`5`, `4,8`, `92 %`) — at most 12 characters, or the tile overflows.
    pub value: String,
    /// What the figure counts.
    pub label: I18nText,
    /// Something the host should look at; `null` when there is nothing to do.
    #[serde(default)]
    pub attention: Option<StatsAttention>,
    /// Change over the previous window; `null` when there is none to show.
    #[serde(default)]
    pub trend: Option<StatsTrend>,
}

/// Something to handle, shown on the tile and in the workspace to-do list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatsAttention {
    /// How urgent.
    pub level: AttentionLevel,
    /// What to do.
    pub text: I18nText,
}

/// Urgency of a [`StatsAttention`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AttentionLevel {
    /// The host has something to do.
    Action,
    /// Worth a look.
    Warning,
}

/// Change of the figure over the previous window.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatsTrend {
    /// As displayed (`+2`, `-10 %`).
    pub delta: String,
    /// Which way the figure went.
    pub direction: TrendDirection,
    /// Whether that way is good news — more reports going up is not.
    pub good: bool,
}

/// Direction of a [`StatsTrend`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TrendDirection {
    /// Went up.
    Up,
    /// Went down.
    Down,
}

/// A tile showing `value`, labelled `label`, with nothing to handle and no trend.
pub fn summary(value: impl Into<String>, label: I18nText) -> StatsSummary {
    StatsSummary {
        value: value.into(),
        label,
        attention: None,
        trend: None,
    }
}

impl StatsSummary {
    /// Flags something to handle.
    pub fn attention(mut self, level: AttentionLevel, text: I18nText) -> Self {
        self.attention = Some(StatsAttention { level, text });
        self
    }

    /// Shows a change over the previous window.
    pub fn trend(
        mut self,
        delta: impl Into<String>,
        direction: TrendDirection,
        good: bool,
    ) -> Self {
        self.trend = Some(StatsTrend {
            delta: delta.into(),
            direction,
            good,
        });
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_period_is_bounded_and_written() {
        let args = |period| StatsSummaryArgs {
            property_id: Uuid::nil(),
            period,
            key: "stock".into(),
        };
        let days: Vec<u32> = [0, 30, 90, 365, 400]
            .map(|period| args(period).period().days())
            .to_vec();
        assert_eq!(days, [30, 30, 90, 365, 30]);
        assert_eq!(Period::Days90.window("en"), "over 90 days");
        assert_eq!(Period::Days365.window("de-DE"), "in 12 Monaten");
        let now = chrono::DateTime::parse_from_rfc3339("2026-09-25T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        assert_eq!(
            Period::Days30.since(now).to_rfc3339(),
            "2026-08-26T00:00:00+00:00"
        );
    }
}
