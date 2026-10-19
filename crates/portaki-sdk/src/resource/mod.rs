//! Declared resources — the SDK stores, merges, checks and draws what a module declares.
//!
//! **Unstable** (feature `unstable`): the SDK 11 proof, see `PREUVE-SDK11.md`. Its API may change
//! in any release while the feature exists.
//!
//! A module declares the shape of its data and the SDK does the rest:
//!
//! - [`#[item]`](crate::item) on a row type: its `#[field(…)]`s are the schema — kind from the
//!   Rust type, label, constraints (`required`, `max_chars`, `between`);
//! - [`#[choice]`](crate::choice) on a unit enum: the values a field picks from, their labels and
//!   icons;
//! - [`#[collection]`](crate::collection) on a marker struct: an ordered list of items, stored
//!   by the SDK, with the host's `updateConfig`, the import's `importRows` and
//!   `publishReadiness` generated.
//!
//! The views follow from the declaration: [`CollectionEditor`] for the host, [`ListCard`] and
//! [`GroupedList`] for the booklet.
//!
//! ```ignore
//! #[portaki_sdk::choice(label = "rule.status")]
//! pub enum RuleStatus { #[default] Neutral, Important, Allowed, Forbidden }
//!
//! #[portaki_sdk::item(key = "title")]
//! pub struct Rule {
//!     #[field(required, max_chars = 60, label = "host.rule.title")]
//!     pub title: I18nText,
//!     #[field(label = "host.rule.status")]
//!     pub status: RuleStatus,
//! }
//!
//! #[portaki_sdk::collection(name = "rules", item = Rule, max = 30)]
//! pub struct Rules;
//!
//! let rules = Rules::load(&ctx)?;
//! ```

mod editor;
mod guest;
pub mod merge;
pub mod store;

use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;

use crate::contracts::i18n::I18nText;
use crate::sdui::common::ChoiceOption;
use crate::vocab::IconName;

pub use editor::CollectionEditor;
pub use guest::{count_line, empty_state, GroupedList, ListCard, Row};

/// A unit enum a field picks from — `#[portaki_sdk::choice]` writes it.
pub trait Choice: Copy + Default + PartialEq + Serialize + DeserializeOwned + 'static {
    /// Every value, in declaration order.
    const ALL: &'static [Self];
    /// The bundle prefix of the labels: a value's label is `<prefix>.<wire>`.
    const LABEL_PREFIX: &'static str;

    /// The stored value.
    fn wire(self) -> &'static str;

    /// The icon shown next to the value, if any.
    fn icon(self) -> Option<IconName> {
        None
    }

    /// The bundle key of the label, `i18n:`-prefixed for SDUI text.
    fn label_key(self) -> String {
        format!("i18n:{}.{}", Self::LABEL_PREFIX, self.wire())
    }

    /// The value stored as `wire`, if any.
    fn from_wire(wire: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .copied()
            .find(|value| value.wire() == wire.trim())
    }

    /// The options of a `Select` / `ChoiceList`.
    fn options() -> Vec<ChoiceOption> {
        Self::ALL
            .iter()
            .map(|value| {
                let mut option = ChoiceOption::new(value.wire(), value.label_key());
                option.icon = value.icon();
                option
            })
            .collect()
    }
}

/// How a declared field is stored, merged and edited.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldKind {
    /// [`I18nText`]: the host writes his language, the others are kept.
    Localized,
    /// A plain string.
    Text,
    /// A boolean.
    Toggle,
    /// A number.
    Number,
    /// One value of a [`Choice`].
    Choice,
    /// `HH:MM-HH:MM`, empty for none.
    TimeRange,
}

impl FieldKind {
    /// Whether the field carries text a row is "about" — a row whose texts are all blank is empty.
    pub const fn is_text(self) -> bool {
        matches!(self, Self::Localized | Self::Text)
    }
}

/// One declared field, as `#[item]` writes it.
#[derive(Debug, Clone, Copy)]
pub struct FieldSpec {
    /// Stored key.
    pub key: &'static str,
    /// Kind.
    pub kind: FieldKind,
    /// Bundle key of the label.
    pub label: &'static str,
    /// Bundle key of the help text, if any.
    pub description: Option<&'static str>,
    /// Publication is blocked while it is empty.
    pub required: bool,
    /// At most this many characters.
    pub max_chars: Option<usize>,
    /// A number at least this.
    pub min: Option<f64>,
    /// A number at most this.
    pub max: Option<f64>,
    /// The options of a [`FieldKind::Choice`].
    pub options: Option<fn() -> Vec<ChoiceOption>>,
}

impl FieldSpec {
    /// The field's problem in `row`, if any — `None` when it is fine.
    pub fn problem(&self, row: &Value) -> Option<I18nText> {
        let value = &row[self.key];
        let text = text_of(value);
        if self.required && text.trim().is_empty() {
            return Some(I18nText::new("Champ obligatoire.", "Required."));
        }
        if let Some(max) = self.max_chars {
            for (_, text) in texts_of(value) {
                if let Some(error) = crate::config::check::max_chars(&text, max) {
                    return Some(error);
                }
            }
        }
        if let (Some(number), Some(min), Some(max)) = (value.as_f64(), self.min, self.max) {
            return crate::config::check::between(number, min, max);
        }
        None
    }
}

/// The text a value holds, any language — what "empty" is judged on.
fn text_of(value: &Value) -> String {
    texts_of(value)
        .into_iter()
        .map(|(_, text)| text)
        .find(|text| !text.trim().is_empty())
        .unwrap_or_default()
}

/// Every `(language, text)` of a value: a localized object's languages, or `("", text)`.
fn texts_of(value: &Value) -> Vec<(String, String)> {
    match value {
        Value::String(text) => vec![(String::new(), text.clone())],
        Value::Object(map) => map
            .iter()
            .filter_map(|(lang, text)| Some((lang.clone(), text.as_str()?.to_string())))
            .collect(),
        Value::Number(number) => vec![(String::new(), number.to_string())],
        _ => Vec::new(),
    }
}

/// A row type — `#[portaki_sdk::item]` writes it.
pub trait Item: Serialize + DeserializeOwned + Default + Clone + 'static {
    /// The natural key: two rows with the same text there are the same row to an import.
    const KEY: Option<&'static str>;

    /// The declared fields, in source order.
    fn fields() -> &'static [FieldSpec];

    /// The row's id.
    fn id(&self) -> &str;
}

/// The settings stored with a collection — `#[portaki_sdk::item(meta)]` writes it.
pub trait Meta: Serialize + DeserializeOwned + Default + Clone + 'static {
    /// The declared fields, in source order.
    fn fields() -> &'static [FieldSpec];
}

impl Meta for () {
    fn fields() -> &'static [FieldSpec] {
        &[]
    }
}

/// What `#[collection]` declares.
#[derive(Debug, Clone, Copy)]
pub struct CollectionDecl {
    /// The resource name: wire key of the rows in `updateConfig`, `resource` of `importRows`.
    pub name: &'static str,
    /// At most this many rows before publication is blocked (and an import stops adding).
    pub max: Option<usize>,
    /// Publication waits for at least one row.
    pub required: bool,
}

/// A problem that blocks publication.
#[derive(Debug, Clone, PartialEq)]
pub struct Problem {
    /// The row, `None` for the collection or its settings.
    pub row: Option<String>,
    /// The field key, or `"rows"` for the collection itself.
    pub field: &'static str,
    /// What to show.
    pub message: I18nText,
}

/// An ordered list of rows and its settings, as stored.
#[derive(Debug, Clone)]
pub struct Collection<T: Item, M: Meta = ()> {
    decl: CollectionDecl,
    rows: Vec<T>,
    meta: M,
}

impl<T: Item, M: Meta> Collection<T, M> {
    /// A collection of `rows`.
    pub fn new(decl: CollectionDecl, rows: Vec<T>, meta: M) -> Self {
        Self { decl, rows, meta }
    }

    /// The declaration.
    pub fn decl(&self) -> CollectionDecl {
        self.decl
    }

    /// The rows, in the host's order.
    pub fn rows(&self) -> &[T] {
        &self.rows
    }

    /// The rows, to change them.
    pub fn rows_mut(&mut self) -> &mut Vec<T> {
        &mut self.rows
    }

    /// The settings.
    pub fn meta(&self) -> &M {
        &self.meta
    }

    /// The settings, to change them.
    pub fn meta_mut(&mut self) -> &mut M {
        &mut self.meta
    }

    /// The number of rows.
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    /// No row.
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// The row with this id.
    pub fn find(&self, id: &str) -> Option<&T> {
        self.rows.iter().find(|row| row.id() == id)
    }

    /// What blocks publication: each row's fields, the settings, and more rows than `max`.
    pub fn problems(&self) -> Vec<Problem> {
        let mut problems = Vec::new();
        if self.decl.required && self.rows.is_empty() {
            let key = format!("publish.{}.hint", self.decl.name);
            let text = crate::t!(&key).ok().filter(|text| *text != key);
            problems.push(Problem {
                row: None,
                field: "rows",
                message: text.map_or_else(
                    || I18nText::new("Ajoutez au moins une ligne.", "Add at least one row."),
                    |text| I18nText::from(text.as_str()),
                ),
            });
        }
        if let Some(max) = self.decl.max.filter(|max| self.rows.len() > *max) {
            problems.push(Problem {
                row: None,
                field: "rows",
                message: I18nText::new(
                    format!("{max} lignes au maximum."),
                    format!("{max} rows at most."),
                ),
            });
        }
        let meta = serde_json::to_value(&self.meta).unwrap_or(Value::Null);
        for field in M::fields() {
            if let Some(message) = field.problem(&meta) {
                problems.push(Problem {
                    row: None,
                    field: field.key,
                    message,
                });
            }
        }
        for row in &self.rows {
            let value = serde_json::to_value(row).unwrap_or(Value::Null);
            for field in T::fields() {
                if let Some(message) = field.problem(&value) {
                    problems.push(Problem {
                        row: Some(row.id().to_string()),
                        field: field.key,
                        message,
                    });
                }
            }
        }
        problems
    }

    /// The problem of one field of one row, for the editor.
    pub fn problem_of(&self, row_id: &str, field: &str) -> Option<I18nText> {
        self.problems()
            .into_iter()
            .find(|p| p.row.as_deref() == Some(row_id) && p.field == field)
            .map(|p| p.message)
    }
}

/// The arguments of `importRows`.
#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct ImportArgs {
    /// The resource the rows go to.
    pub resource: String,
    /// The rows, each an object of declared field keys.
    #[serde(default)]
    pub rows: Vec<Value>,
}

/// The generated `updateConfig`: the host form, merged by [`merge::host_save`].
pub fn host_command<T: Item, M: Meta>(
    ctx: &crate::Context,
    decl: CollectionDecl,
    args: &Value,
) -> crate::Result<()> {
    let (rows, meta) = store::load_raw(decl.name)?;
    let (rows, meta) = merge::host_save(
        &rows,
        &meta,
        args,
        decl.name,
        T::fields(),
        M::fields(),
        &ctx.lang(),
    );
    store::save(&typed::<T, M>(decl, rows, meta)?)
}

/// The generated `importRows`: rows added by [`merge::import`], never over the host's.
pub fn import_command<T: Item, M: Meta>(
    ctx: &crate::Context,
    decl: CollectionDecl,
    args: ImportArgs,
) -> crate::Result<merge::ImportReport> {
    if args.resource != decl.name {
        return Err(crate::PortakiError::Host(format!(
            "resource_unknown: {}",
            args.resource
        )));
    }
    let lang = ctx
        .property_lang()
        .map_or_else(|| ctx.lang(), str::to_string);
    let (rows, meta) = store::load_raw(decl.name)?;
    let (rows, report) = merge::import(&rows, &args.rows, T::fields(), T::KEY, decl.max, &lang);
    if report.added > 0 {
        store::save(&typed::<T, M>(decl, rows, meta)?)?;
    }
    Ok(report)
}

/// The generated `publishReadiness`: every [`Collection::problems`], blocking.
pub fn readiness<T: Item, M: Meta>(
    ctx: &crate::Context,
    decl: CollectionDecl,
) -> crate::Result<crate::contracts::publish::PublishReadiness> {
    use crate::contracts::publish::{PublishCheck, PublishLevel, PublishReadiness};
    let collection = store::load::<T, M>(decl)?;
    let label_of = |field: &str| {
        let key = T::fields()
            .iter()
            .chain(M::fields())
            .find(|f| f.key == field)
            .map_or_else(
                || format!("host.{}.title", decl.name),
                |f| f.label.to_string(),
            );
        let text = crate::t!(&key).unwrap_or(key);
        I18nText::from(text.as_str())
    };
    let _ = ctx;
    Ok(PublishReadiness {
        items: collection
            .problems()
            .into_iter()
            .map(|problem| PublishCheck {
                id: match &problem.row {
                    Some(row) => format!("resources.{}.{row}.{}", decl.name, problem.field),
                    None => format!("resources.{}.{}", decl.name, problem.field),
                },
                level: PublishLevel::Required,
                ok: false,
                label: label_of(problem.field),
                hint: problem.message,
            })
            .collect(),
    })
}

/// Rows and settings as JSON, read back as the declared types — a value that does not read is
/// an error, never stored.
fn typed<T: Item, M: Meta>(
    decl: CollectionDecl,
    rows: Vec<Value>,
    meta: Value,
) -> crate::Result<Collection<T, M>> {
    let rows = rows
        .into_iter()
        .map(serde_json::from_value)
        .collect::<std::result::Result<Vec<T>, _>>()?;
    Ok(Collection::new(decl, rows, serde_json::from_value(meta)?))
}
