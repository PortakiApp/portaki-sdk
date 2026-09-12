//! Shared SDUI styling tokens and nested prop types (semantic roles — resolved by shells).

use serde::{Deserialize, Serialize};

use super::action::Action;
use super::primitives::Component;

/// Semantic color role.
///
/// A role, never a color: each shell resolves it against its own theme — the host dashboard,
/// the guest booklet in the property's brand, light or dark. A module that wants "the brand
/// color" asks for `Primary`; one that wants a quieter companion to it asks for `Secondary`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum Tone {
    /// Default neutral tone.
    #[default]
    Neutral,
    /// Primary brand tone.
    Primary,
    /// Secondary brand tone — a quieter companion to `Primary`.
    Secondary,
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

/// Enter/exit animation kind for shells.
#[portaki_sdk_macros::wire]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum AnimationKind {
    /// Fade up.
    FadeUp,
    /// Fade in.
    FadeIn,
    /// Scale in.
    ScaleIn,
    /// Slide from the right.
    SlideRight,
    /// No animation.
    None,
}

/// Enter/exit animation hints for shells.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct Animation {
    /// Enter animation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enter: Option<AnimationKind>,
    /// Exit animation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit: Option<AnimationKind>,
    /// Stagger delay in milliseconds between children.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stagger: Option<u32>,
}

/// Shell visibility expression (capability gate or client predicate).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(transparent)]
pub struct VisibilityExpr(pub String);

impl VisibilityExpr {
    /// Wraps a raw expression string.
    pub fn new(expr: impl Into<String>) -> Self {
        Self(expr.into())
    }

    /// Borrowed expression text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for VisibilityExpr {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl From<String> for VisibilityExpr {
    fn from(value: String) -> Self {
        Self(value)
    }
}

/// Conditional visibility hint.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Visibility {
    /// Expression or capability gate evaluated by the shell.
    pub when: VisibilityExpr,
}

impl Visibility {
    /// Builds a visibility gate from an expression.
    pub fn when(expr: impl Into<VisibilityExpr>) -> Self {
        Self { when: expr.into() }
    }
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

/// Temperature unit for the [`crate::sdui::primitives::Temperature`] primitive.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum TemperatureUnit {
    /// Degrees Celsius (`C` on the wire).
    #[default]
    #[serde(rename = "C", alias = "celsius")]
    Celsius,
    /// Degrees Fahrenheit (`F` on the wire).
    #[serde(rename = "F", alias = "fahrenheit")]
    Fahrenheit,
}

/// Text typography variant.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum TextVariant {
    /// Body copy.
    #[default]
    Body,
    /// Caption / secondary.
    Caption,
    /// Section title.
    Title,
    /// Display / hero title.
    Display,
}

/// A named tint — for a color that is information, not a theme role.
///
/// The yellow recycling bin is yellow because the municipality says so, whatever the property's
/// brand: that is content, and `Tone` cannot say it. But a free CSS string (`color: "#f4c020"`)
/// hard-codes a value the shell can't adapt — to its palette, to a dark theme, to contrast. A
/// swatch names the tint and lets each shell resolve it.
///
/// Prefer `swatch` over the `color` string on `ColorDotItem` and `Dot`: shells honor `color`
/// only for modules built before swatches existed.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Swatch {
    /// Yellow.
    Yellow,
    /// Green.
    Green,
    /// Blue.
    Blue,
    /// Brown.
    Brown,
    /// Grey.
    Grey,
    /// Black.
    Black,
    /// White.
    White,
    /// Red.
    Red,
    /// Orange.
    Orange,
    /// Purple.
    Purple,
}

/// Button visual variant.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ButtonVariant {
    /// Filled primary.
    #[default]
    Filled,
    /// Outlined.
    Outline,
    /// Ghost / text.
    Ghost,
}

/// Stack layout direction.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum StackDirection {
    /// Vertical stack (default).
    #[default]
    Vertical,
    /// Horizontal row.
    Horizontal,
}

/// Map interaction mode.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum MapInteractionMode {
    /// Pan and zoom enabled.
    #[default]
    #[serde(rename = "pan-zoom")]
    PanZoom,
    /// Static / non-interactive map.
    #[serde(rename = "none")]
    None,
}

/// ChoiceList layout variant.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ChoiceListLayout {
    /// Compact list rows.
    #[default]
    Compact,
    /// Card grid.
    Cards,
}

/// Map marker kind.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum MapMarkerKind {
    /// Property hub marker.
    Property,
    /// Point of interest.
    #[default]
    Poi,
}

/// Geographic coordinate.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct GeoPoint {
    /// Latitude (WGS-84).
    pub lat: f64,
    /// Longitude (WGS-84).
    pub lng: f64,
}

impl GeoPoint {
    /// Creates a coordinate pair.
    pub fn new(lat: f64, lng: f64) -> Self {
        Self { lat, lng }
    }
}

/// Map viewport (center + optional zoom).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MapViewport {
    /// Map center.
    pub center: GeoPoint,
    /// Zoom level.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub zoom: Option<f64>,
}

impl MapViewport {
    /// Creates a viewport centered on `(lat, lng)` with optional zoom.
    pub fn new(lat: f64, lng: f64, zoom: Option<f64>) -> Self {
        Self {
            center: GeoPoint::new(lat, lng),
            zoom,
        }
    }
}

/// Map marker payload.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MapMarker {
    /// Stable marker id.
    pub id: String,
    /// Latitude.
    pub lat: f64,
    /// Longitude.
    pub lng: f64,
    /// Optional label.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Optional category slug.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    /// Optional Lucide icon name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    /// Optional semantic tone.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tone: Option<Tone>,
    /// Marker kind (`property` hub vs `poi` pin).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<MapMarkerKind>,
}

impl MapMarker {
    /// Creates a marker at `(lat, lng)`.
    pub fn new(id: impl Into<String>, lat: f64, lng: f64) -> Self {
        Self {
            id: id.into(),
            lat,
            lng,
            label: None,
            category: None,
            icon: None,
            tone: None,
            kind: None,
        }
    }

    /// Sets the label.
    pub fn label(mut self, value: impl Into<String>) -> Self {
        self.label = Some(value.into());
        self
    }

    /// Sets the marker kind.
    pub fn kind(mut self, value: MapMarkerKind) -> Self {
        self.kind = Some(value);
        self
    }

    /// Sets the tone.
    pub fn tone(mut self, value: Tone) -> Self {
        self.tone = Some(value);
        self
    }

    /// Sets the icon name.
    pub fn icon(mut self, value: impl Into<String>) -> Self {
        self.icon = Some(value.into());
        self
    }
}

/// Option row for [`crate::sdui::primitives::ChoiceList`] / Select / RadioGroup.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChoiceOption {
    /// Wire value submitted on selection.
    pub value: String,
    /// Display label (often an `i18n:` key).
    pub label: String,
    /// Optional description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Optional Lucide icon name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
}

impl ChoiceOption {
    /// Creates a value/label option.
    pub fn new(value: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
            description: None,
            icon: None,
        }
    }

    /// Sets the description.
    pub fn description(mut self, value: impl Into<String>) -> Self {
        self.description = Some(value.into());
        self
    }

    /// Sets the icon.
    pub fn icon(mut self, value: impl Into<String>) -> Self {
        self.icon = Some(value.into());
        self
    }
}

/// Minimal TipTap document (`doc` → paragraphs / lists → text).
///
/// Serialize with [`RichTextDoc::to_json_string`] for `RichTextEditor` values.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RichTextDoc {
    /// Always `"doc"` on the wire.
    #[serde(rename = "type")]
    pub doc_type: String,
    /// Top-level block nodes.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub content: Vec<RichTextBlock>,
}

impl RichTextDoc {
    /// Empty document.
    pub fn new() -> Self {
        Self {
            doc_type: "doc".to_string(),
            content: Vec::new(),
        }
    }

    /// Appends a paragraph with plain text (skipped when empty after trim).
    pub fn paragraph(mut self, text: impl Into<String>) -> Self {
        let text = text.into();
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return self;
        }
        self.content.push(RichTextBlock::Paragraph {
            content: vec![RichTextInline::Text {
                text: trimmed.to_string(),
            }],
        });
        self
    }

    /// Appends a bullet list from plain-text items.
    pub fn bullet_list<I, S>(mut self, items: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let list_items: Vec<RichTextBlock> = items
            .into_iter()
            .map(|s| {
                let text = s.into();
                RichTextBlock::ListItem {
                    content: vec![RichTextBlock::Paragraph {
                        content: vec![RichTextInline::Text { text }],
                    }],
                }
            })
            .collect();
        if !list_items.is_empty() {
            self.content.push(RichTextBlock::BulletList {
                content: list_items,
            });
        }
        self
    }

    /// Ensures at least one empty paragraph (TipTap default empty doc).
    pub fn ensure_non_empty(mut self) -> Self {
        if self.content.is_empty() {
            self.content.push(RichTextBlock::Paragraph {
                content: Vec::new(),
            });
        }
        self
    }

    /// Serializes to a JSON string for editor / storage fields.
    pub fn to_json_string(&self) -> String {
        serde_json::to_string(self)
            .unwrap_or_else(|_| r#"{"type":"doc","content":[{"type":"paragraph"}]}"#.to_string())
    }
}

impl Default for RichTextDoc {
    fn default() -> Self {
        Self::new()
    }
}

/// TipTap block node (subset used by modules).
#[portaki_sdk_macros::wire]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum RichTextBlock {
    /// Paragraph block.
    Paragraph {
        /// Inline content.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        content: Vec<RichTextInline>,
    },
    /// Bullet list.
    BulletList {
        /// List items.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        content: Vec<RichTextBlock>,
    },
    /// List item.
    ListItem {
        /// Nested blocks.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        content: Vec<RichTextBlock>,
    },
}

/// TipTap inline node (subset).
#[portaki_sdk_macros::wire]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type")]
pub enum RichTextInline {
    /// Plain text run.
    Text {
        /// Text content.
        text: String,
    },
}

#[cfg(test)]
mod tone_tests {
    use super::Tone;

    /// The wire names both shells match on.
    #[test]
    fn tones_serialise_in_snake_case() {
        assert_eq!(
            serde_json::to_string(&Tone::Secondary).unwrap(),
            "\"secondary\""
        );
        assert_eq!(
            serde_json::from_str::<Tone>("\"secondary\"").unwrap(),
            Tone::Secondary
        );
    }
}

#[cfg(test)]
mod swatch_tests {
    use crate::sdui::component::Component;
    use crate::sdui::primitives::ColorDotItem;

    use super::Swatch;

    /// The wire shape both shells read: a named tint, no CSS.
    #[test]
    fn a_color_dot_carries_its_swatch_by_name() {
        let dot: Component = ColorDotItem::new()
            .label("Bac jaune")
            .swatch(Swatch::Yellow)
            .into();

        let json = serde_json::to_value(&dot).unwrap();

        assert_eq!(json["swatch"], "yellow");
        assert!(json.get("color").is_none());
    }
}

/// One tab of a [`BottomTabBar`](super::primitives::BottomTabBar).
///
/// The field was `Value` — untyped — and every shell read `{ id, label, action }` from it
/// anyway. Naming the shape is what lets a module be told it got it wrong.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct TabBarItem {
    /// Matches `activeTab` to mark the current tab.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Display label (often an `i18n:` key).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Fired on tap.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<Action>,
}

/// Marker clustering for a [`Map`](super::primitives::Map).
///
/// Also `Value` before: the shells read `{ enabled, radius, maxZoom }` and nothing said so.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct MapClustering {
    /// Cluster nearby markers.
    pub enabled: bool,
    /// Cluster radius in pixels.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub radius: Option<f64>,
    /// Zoom level past which markers separate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_zoom: Option<f64>,
}

/// One chip of a [`FilterBar`](super::primitives::FilterBar).
///
/// Not `FilterChip`: that name is already a primitive of its own in the contract.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct FilterBarChip {
    /// Wire value submitted on selection.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Display label (often an `i18n:` key).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

/// One button of an [`ActionRow`](super::primitives::ActionRow).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ActionRowItem {
    /// Display label (often an `i18n:` key).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Fired on press.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<Action>,
}

/// One section of an [`Accordion`](super::primitives::Accordion).
///
/// `content` is a nested tree, not a string: a collapsed section holds a surface, and forcing it
/// through `children` would tie sections to child order by index.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct AccordionItem {
    /// Summary line, always visible.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// What unfolds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<Box<Component>>,
}

/// One tab of a [`Tabs`](super::primitives::Tabs).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct TabItem {
    /// Selection key.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Display label (often an `i18n:` key).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// The panel this tab shows.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<Box<Component>>,
}
