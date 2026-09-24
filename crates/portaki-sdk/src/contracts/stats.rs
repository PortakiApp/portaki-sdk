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
