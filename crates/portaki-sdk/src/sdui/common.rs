//! Shared SDUI styling tokens and nested prop types (semantic roles — resolved by shells).

use serde::{Deserialize, Serialize};

use super::action::Action;
use super::primitives::Component;
pub use crate::vocab::IconName;

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
    /// Compact rows, chosen inline.
    #[default]
    Compact,
    /// Card grid.
    Cards,
    /// One description per row, stacked.
    List,
    /// Tiles in a grid — emoji plus label, several may be picked.
    Grid,
    /// Segmented control, two to four short choices side by side.
    Segmented,
    /// Checklist with progress, groups and a done banner.
    Checklist,
    /// Free text with suggestions, accent-insensitive (station search).
    Combobox,
    /// One to five stars, draggable, arrow keys included.
    Stars,
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
    /// Optional icon.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<IconName>,
    /// Optional semantic tone.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tone: Option<Tone>,
    /// Marker kind (`property` hub vs `poi` pin).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<MapMarkerKind>,
    /// Emoji drawn in the pin — what the booklet map uses for a module's places.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub emoji: Option<String>,
    /// Second line under the name, on the map and in the row of places.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subtitle: Option<String>,
    /// What tapping the pin does — normally opens the module's own detail.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action: Option<Action>,
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
            emoji: None,
            subtitle: None,
            action: None,
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
    pub fn icon(mut self, value: IconName) -> Self {
        self.icon = Some(value);
        self
    }

    /// Sets the emoji drawn in the pin.
    pub fn emoji(mut self, value: impl Into<String>) -> Self {
        self.emoji = Some(value.into());
        self
    }

    /// Sets the second line under the name.
    pub fn subtitle(mut self, value: impl Into<String>) -> Self {
        self.subtitle = Some(value.into());
        self
    }

    /// Sets what tapping the pin does.
    pub fn action(mut self, value: Action) -> Self {
        self.action = Some(value);
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
    /// Optional icon.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<IconName>,
    /// Optional emoji, shown instead of an icon on tile layouts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub emoji: Option<String>,
    /// Heading this option sits under (checklist groups, room names).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
}

impl ChoiceOption {
    /// Creates a value/label option.
    pub fn new(value: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
            description: None,
            icon: None,
            emoji: None,
            group: None,
        }
    }

    /// Sets the emoji.
    pub fn emoji(mut self, value: impl Into<String>) -> Self {
        self.emoji = Some(value.into());
        self
    }

    /// Sets the group heading.
    pub fn group(mut self, value: impl Into<String>) -> Self {
        self.group = Some(value.into());
        self
    }

    /// Sets the description.
    pub fn description(mut self, value: impl Into<String>) -> Self {
        self.description = Some(value.into());
        self
    }

    /// Sets the icon.
    pub fn icon(mut self, value: IconName) -> Self {
        self.icon = Some(value);
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

/// How large an [`Image`](super::primitives::Image) renders.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ImageSize {
    /// As wide as its container (default).
    #[default]
    Full,
    /// A small square thumbnail; the host renderer opens the full image on click — e.g. a
    /// guest photo in a list row.
    Thumb,
}

/// Shape of a [`Chart`](super::primitives::Chart).
///
/// The data rides in `points`, not in children: a chart is **one** node, whatever its length,
/// so it costs a placement's node budget once.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ChartKind {
    /// Vertical bars, label under each; `highlight` picks the solid one (default: the last).
    #[default]
    Bars,
    /// Horizontal bars, label left and `display` right, each scaled to `max` (default: the
    /// largest value).
    HorizontalBars,
    /// Ring of parts; `value` is each part's percentage, `center` the text in the hole.
    Donut,
    /// Grid of cells, `columns` wide, each shaded by `value / max` (default max: the largest).
    Heatmap,
}

/// One datum of a [`Chart`](super::primitives::Chart).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ChartPoint {
    /// Axis or legend label (often an `i18n:` key).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// The number the mark is drawn from.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<f64>,
    /// What the shell prints for the value (« 2 signalements », « 6 h »); `value` when absent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display: Option<String>,
    /// Per-point tint (donut parts); the chart's `swatch` otherwise.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub swatch: Option<Swatch>,
}

impl ChartPoint {
    /// A labelled value.
    pub fn new(label: impl Into<String>, value: f64) -> Self {
        Self {
            label: Some(label.into()),
            value: Some(value),
            ..Self::default()
        }
    }

    /// Text shown for the value.
    pub fn display(mut self, display: impl Into<String>) -> Self {
        self.display = Some(display.into());
        self
    }

    /// Tint of this point.
    pub fn swatch(mut self, swatch: Swatch) -> Self {
        self.swatch = Some(swatch);
        self
    }
}

/// Empty state of a [`Chart`](super::primitives::Chart) with nothing to draw yet.
///
/// Written by the module, drawn compact inside the panel by the shell.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct ChartEmpty {
    /// Short headline (often an `i18n:` key).
    pub title: String,
    /// One sentence saying when data will appear.
    pub text: String,
}

impl ChartEmpty {
    /// An empty state reading `title`, then `text`.
    pub fn new(title: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            text: text.into(),
        }
    }
}

/// Whether a [`Stat`](super::primitives::Stat)'s `delta` is good news.
///
/// The sign alone lies: "−40 min" of response time is an improvement.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DeltaTone {
    /// Good news.
    Good,
    /// Bad news.
    Bad,
    /// Neither.
    Neutral,
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

/// One row of an [`EditableList`](super::primitives::EditableList).
///
/// The list submits the rows as JSON under its `name`, in the order the host left them.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct EditableListItem {
    /// Stable id of an existing row; absent on a row the host just added.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Text (French when the list is `bilingual`).
    pub label: String,
    /// English text, when the list is `bilingual`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label_en: Option<String>,
    /// "Photo" switched on, when the list has `photoToggle`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub photo: Option<bool>,
    /// Ticked, when the list has `checkbox`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checked: Option<bool>,
}

impl EditableListItem {
    /// A row showing `label`.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            ..Self::default()
        }
    }
}

/// Status pill on the right of a [`FeedItem`](super::primitives::FeedItem).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct FeedStatus {
    /// Pill text (`En cours`, `Résolu`).
    pub label: String,
    /// Pill tint.
    pub tone: Tone,
}

impl FeedStatus {
    /// A `tone` pill reading `label`.
    pub fn new(label: impl Into<String>, tone: Tone) -> Self {
        Self {
            label: label.into(),
            tone,
        }
    }
}

// ─── Livret voyageur v3 — visuels et états nommés par les primitives révisées.
//
// Le module envoie du contenu et une intention ; il ne choisit ni couleur, ni variante de
// carte, ni fond. Ces types décrivent donc ce qu'il y a à montrer, jamais comment le peindre.

/// How a [`Card`](super::primitives::Card) fills its slot.
///
/// Not a look: the booklet still picks the card's variant from its placement.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum CardPresentation {
    /// Full card with its own header.
    #[default]
    Full,
    /// One row inside a list of cards.
    Row,
}

/// Shape of a [`ListItem`](super::primitives::ListItem).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ListItemLayout {
    /// Row, full width.
    #[default]
    Row,
    /// Tile, sized by its container (carousels, grids).
    Tile,
}

/// Shape of a [`KeyValue`](super::primitives::KeyValue).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum KeyValueLayout {
    /// Label and value on one row.
    #[default]
    Row,
    /// Tile: label above, value large below.
    Tile,
}

/// What stands at the head of a [`ListItem`](super::primitives::ListItem).
///
/// A bare string stays readable on the wire — older payloads sent an icon name — and
/// deserializes as [`Leading::Icon`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum Leading {
    /// Icon name alone (legacy wire form).
    Icon(String),
    /// One of the named visuals.
    Visual(Box<LeadingVisual>),
}

impl Default for Leading {
    fn default() -> Self {
        Self::Visual(Box::default())
    }
}

/// The named visuals a row may lead with — at most one is set.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct LeadingVisual {
    /// Icon name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<IconName>,
    /// Emoji, shown as-is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub emoji: Option<String>,
    /// Rank in a numbered list of steps.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub index: Option<u32>,
    /// Color swatch (waste bins).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub swatch: Option<Swatch>,
    /// Time, in the property's local time (`08:12`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time: Option<String>,
    /// Map thumbnail centred on this point.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub map: Option<GeoPoint>,
    /// Photo URL.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    /// Provider logo — `url` plus the name it stands for.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub logo: Option<LeadingLogo>,
}

/// A provider logo at the head of a row.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct LeadingLogo {
    /// Image URL (`https` only).
    pub url: String,
    /// Provider name, for the alternative text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

/// What closes a [`ListItem`](super::primitives::ListItem): a badge, or plain text.
///
/// A bare string deserializes as [`Trailing::Text`] — the legacy wire form.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum Trailing {
    /// Plain text (legacy wire form).
    Text(String),
    /// One of the named trailing visuals.
    Visual(Box<TrailingVisual>),
}

impl Default for Trailing {
    fn default() -> Self {
        Self::Visual(Box::default())
    }
}

/// Badge or text at the end of a row — at most one is set.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct TrailingVisual {
    /// Status badge.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub badge: Option<BadgeSpec>,
    /// Plain text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// Render the text in the monospaced face (codes, times).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub mono: bool,
}

/// A badge described by what it says, not how it looks.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct BadgeSpec {
    /// Badge text.
    pub label: String,
    /// Semantic tone; the status tones carry their own glyph.
    #[serde(default)]
    pub tone: Tone,
    /// Ask for the leading dot on a tone that has no glyph of its own.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub dot: bool,
}

impl BadgeSpec {
    /// A `tone` badge reading `label`.
    pub fn new(label: impl Into<String>, tone: Tone) -> Self {
        Self {
            label: label.into(),
            tone,
            dot: false,
        }
    }
}

/// One line of a row that unfolds (opening hours, day by day).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct DetailRow {
    /// Left label (a weekday, a stop).
    pub label: String,
    /// Right value (`09:00 – 19:00`, `Fermé`).
    pub value: String,
    /// This line is the one happening now — the shell highlights it.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub current: bool,
}

impl DetailRow {
    /// A `label` / `value` line.
    pub fn new(label: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            value: value.into(),
            current: false,
        }
    }

    /// Marks this line as the current one.
    pub fn current(mut self) -> Self {
        self.current = true;
        self
    }
}

/// Whether a secret value may be shown yet.
///
/// The module masks the value itself — this only tells the shell what to say about it.
/// A secret is never sent in clear before `revealed` is `true`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct SecretState {
    /// The value carried alongside is the real one.
    pub revealed: bool,
    /// When it becomes available, formatted for the guest. `None` once revealed, or
    /// when no date can be computed (no check-in on the stay).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reveal_at: Option<String>,
}

impl SecretState {
    /// A secret still hidden, available at `reveal_at` when known.
    pub fn hidden(reveal_at: Option<String>) -> Self {
        Self {
            revealed: false,
            reveal_at,
        }
    }

    /// A secret the guest may see.
    pub fn revealed() -> Self {
        Self {
            revealed: true,
            reveal_at: None,
        }
    }
}

/// The person a quote is signed by — always the host, filled by the platform.
///
/// A module never composes it: it asks for the host by leaving it to the shell, or quotes
/// text the host wrote and the shell signs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct Author {
    /// Display name.
    pub name: String,
    /// Role line (`votre hôte`, `votre hôte depuis 2019`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    /// Initials, used when there is no photo.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub initials: Option<String>,
    /// Profile photo URL.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub photo: Option<String>,
}

/// Emphasis of a [`RichText`](super::primitives::RichText) block.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum RichTextVariant {
    /// Running text.
    #[default]
    Body,
    /// A quote, set larger — a card holding only this becomes the editorial variant.
    Lead,
}
