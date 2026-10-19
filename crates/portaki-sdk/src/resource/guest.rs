//! Booklet templates: a home card listing the first rows, a page of rows grouped by a value.
//!
//! The module says what a row reads as ([`Row`]); the template owns the layout, the count line,
//! « see all » and the empty case, so every module that lists things lists them the same way.

use crate::sdui::action::{Action, OverlayArgs, OverlayPresentation};
use crate::sdui::common::{
    BadgeSpec, Leading, SurfaceLevel, TextVariant, Trailing, TrailingVisual,
};
use crate::sdui::component::Component;
use crate::sdui::primitives::{Button, Card, EmptyState, ListItem, Stack, Text};
use crate::sdui::surface::Surface;
use crate::vocab::IconName;
use crate::{ids::SurfaceId, sdui::common::ButtonVariant};

/// What one row reads as in the booklet.
#[derive(Debug, Clone, Default)]
pub struct Row {
    title: String,
    subtitle: Vec<String>,
    icon: Option<String>,
    badge: Option<BadgeSpec>,
}

impl Row {
    /// A row titled `title`.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            ..Self::default()
        }
    }

    /// A part of the line under the title; blank parts are left out, the others joined by « · ».
    pub fn detail(mut self, part: impl Into<String>) -> Self {
        self.subtitle.push(part.into());
        self
    }

    /// The icon at the head of the row (an icon name).
    pub fn icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// A badge at the end of the row.
    pub fn badge(mut self, badge: Option<BadgeSpec>) -> Self {
        self.badge = badge;
        self
    }

    fn component(&self) -> Component {
        let mut item = ListItem::new().title(self.title.clone());
        if let Some(icon) = &self.icon {
            item = item.leading(Leading::Icon(icon.clone()));
        }
        let subtitle = self
            .subtitle
            .iter()
            .map(|part| part.trim())
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join(" · ");
        if !subtitle.is_empty() {
            item = item.subtitle(subtitle);
        }
        if let Some(badge) = &self.badge {
            item = item.trailing(Trailing::Visual(Box::new(TrailingVisual {
                badge: Some(badge.clone()),
                ..TrailingVisual::default()
            })));
        }
        item.into()
    }
}

/// `key` translated with `{count}`, from `key.one` when there is exactly one.
pub fn count_line(key: &str, count: usize) -> String {
    let key = if count == 1 {
        format!("{key}.one")
    } else {
        key.to_string()
    };
    crate::t!(&key, count = count).unwrap_or_else(|_| format!("i18n:{key}"))
}

/// The home card: an icon, a title, a count line, the first rows, « see all » when there are
/// more; it opens the detail. No row: the empty state.
pub struct ListCard {
    rows: Vec<Row>,
    icon: IconName,
    title: String,
    count_key: String,
    see_all_key: String,
    limit: usize,
    detail: SurfaceId,
}

impl ListCard {
    /// The card of `rows`, opening `detail` full screen.
    pub fn new(rows: Vec<Row>, detail: SurfaceId) -> Self {
        Self {
            rows,
            icon: IconName::Info,
            title: String::new(),
            count_key: "home.card.subtitle".into(),
            see_all_key: "home.card.seeAll".into(),
            limit: 4,
            detail,
        }
    }

    /// Icon and title (bundle key) of the card and of the detail it opens.
    pub fn titled(mut self, icon: IconName, title: &str) -> Self {
        self.icon = icon;
        self.title = format!("i18n:{title}");
        self
    }

    /// How many rows the card shows.
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = limit.max(1);
        self
    }

    /// The surface.
    pub fn build(self, id: SurfaceId) -> Surface {
        let total = self.rows.len();
        if total == 0 {
            return empty_state(id, self.icon);
        }
        let open = Action::open_overlay(
            OverlayPresentation::Fullscreen,
            self.detail,
            OverlayArgs::new().icon(self.icon).title(self.title.clone()),
        );
        let mut children: Vec<Component> = self
            .rows
            .iter()
            .take(self.limit)
            .map(Row::component)
            .collect();
        if total > self.limit {
            children.push(
                Button::new()
                    .label(
                        crate::t!(&self.see_all_key, count = total)
                            .unwrap_or_else(|_| format!("i18n:{}", self.see_all_key)),
                    )
                    .variant(ButtonVariant::Outline)
                    .action(open.clone())
                    .into(),
            );
        }
        Surface::new(
            Card::new()
                .icon(self.icon)
                .title(self.title)
                .subtitle(count_line(&self.count_key, total))
                .action(open)
                .children(children),
        )
        .with_id(id)
    }
}

/// A page of rows, one block per group in the host's order; one block without a title when
/// there is a single group (a heading that tells nothing apart is one more line to read).
pub struct GroupedList {
    groups: Vec<(String, Vec<Row>)>,
    intro_key: String,
    icon: IconName,
}

impl GroupedList {
    /// Rows grouped by `group` (its label, empty for none), groups in order of first appearance.
    pub fn new<T>(items: &[T], group: impl Fn(&T) -> String, row: impl Fn(&T) -> Row) -> Self {
        let mut groups: Vec<(String, Vec<Row>)> = Vec::new();
        for item in items {
            let label = group(item);
            match groups.iter_mut().find(|(g, _)| *g == label) {
                Some((_, rows)) => rows.push(row(item)),
                None => groups.push((label, vec![row(item)])),
            }
        }
        Self {
            groups,
            intro_key: "guest.detail.intro".into(),
            icon: IconName::Info,
        }
    }

    /// The icon of each group's block.
    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = icon;
        self
    }

    /// The surface.
    pub fn build(self, id: SurfaceId) -> Surface {
        let count = self.groups.iter().map(|(_, rows)| rows.len()).sum();
        if count == 0 {
            return empty_state(id, self.icon);
        }
        let mut children: Vec<Component> = vec![Text::new()
            .text(count_line(&self.intro_key, count))
            .variant(TextVariant::Caption)
            .into()];
        let grouped = self.groups.len() > 1;
        for (label, rows) in &self.groups {
            let mut card = Card::new().surface(SurfaceLevel::Elevated);
            if grouped && !label.is_empty() {
                card = card.icon(self.icon).title(label.clone());
            }
            let body = Stack::new()
                .gap(0.0)
                .children(rows.iter().map(Row::component).collect());
            children.push(card.child(body).into());
        }
        Surface::new(Stack::new().gap(0.0).children(children)).with_id(id)
    }
}

/// Nothing to list yet: `home.card.empty.title` / `.description` of the module's bundles.
pub fn empty_state(id: SurfaceId, icon: IconName) -> Surface {
    Surface::new(
        EmptyState::new()
            .title("i18n:home.card.empty.title")
            .description("i18n:home.card.empty.description")
            .icon(icon),
    )
    .with_id(id)
}
