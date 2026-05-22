//! Shared SDUI styling tokens (semantic roles — resolved by shells).

use serde::{Deserialize, Serialize};

/// Semantic color role.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum Tone {
    /// Default neutral tone.
    #[default]
    Neutral,
    /// Primary brand tone.
    Primary,
    /// Accent highlight.
    Accent,
    /// Informational.
    Info,
    /// Success state.
    Success,
    /// Warning state.
    Warning,
    /// Danger / destructive.
    Danger,
    /// Emergency (high visibility).
    Emergency,
}

/// Visual emphasis level.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum Emphasis {
    /// Low emphasis.
    Subtle,
    /// Default emphasis.
    #[default]
    Default,
    /// High emphasis.
    Strong,
}

/// Surface elevation level.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum SurfaceLevel {
    /// Default surface.
    #[default]
    Default,
    /// Elevated card/sheet.
    Elevated,
    /// Recessed inset surface.
    Sunken,
}

/// Enter/exit animation hints for shells.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct Animation {
    /// Enter animation id (`fadeUp`, `fadeIn`, `scaleIn`, `slideRight`, `none`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enter: Option<String>,
    /// Exit animation id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit: Option<String>,
    /// Stagger delay in milliseconds between children.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stagger: Option<u32>,
}

/// Conditional visibility hint.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Visibility {
    /// Expression or capability gate evaluated by the shell.
    pub when: String,
}

/// Layout variant for the [`crate::sdui::primitives::Temperature`] primitive.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum TempVariant {
    /// Default inline temperature readout.
    #[default]
    Inline,
    /// Prominent hero-style temperature (home cards).
    Hero,
    /// Compact readout for grids and lists.
    Compact,
}
