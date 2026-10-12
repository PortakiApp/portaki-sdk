//! Server-Driven UI (SDUI) catalog for module-authored interfaces.
//!
//! Modules build a tree of [`Component`] values, wrap them in [`Surface`], and
//! return that tree from `#[surface(...)]` handlers. Shells (host dashboard, guest
//! booklet) deserialize the JSON payload and render native widgets — modules never
//! ship JSX, Flutter, or CSS.
//!
//! ## Host vs guest
//!
//! Primitives, tokens, and [`Action`] are **shared** — there is no `sdui::host` /
//! `sdui::guest` type split. Shells differ at **surface registration**
//! (`#[surface(guest, …)]` vs `#[surface(host, …)]`) and at the Wasm crate layout
//! (`guest/` vs `host/` folders). See `docs/module-layout.md` in the SDK repo.
//!
//! ## Contract
//!
//! - Primitives and enums serialize to the schema version pinned in
//!   [`crate::manifest::UiSchemaVersions`].
//! - User interactions attach [`Action`] values; shells route them back to the
//!   gateway (commands, navigation, external URLs).
//! - Styling uses semantic tokens ([`common::Tone`], [`common::Emphasis`], …) —
//!   shells map tokens to platform theme — do not embed hex colors in Wasm.
//!
//! ## Un champ qui n'apparaît qu'au bon moment — `Field::visibleWhen`
//!
//! `Field::visibleWhen("autreChamp=valeur")` ne montre le champ que tant que la commande nommée
//! porte exactement cette valeur. L'égalité est stricte : une liste à choix multiples ne
//! correspond que si sa valeur sérialisée est égale, pas si elle contient la valeur.
//!
//! Un champ masqué **n'est pas rendu** : il n'est donc ni validé — un `required` masqué n'empêche
//! rien — ni envoyé. C'est ce qui permet de demander une adresse de renvoi obligatoire *sous*
//! « Me le renvoyer » sans l'imposer à qui choisit de repasser la prendre.
//!
//! Le livret et le tableau de bord l'honorent. Côté hôte, un champ masqué n'est pas envoyé :
//! `updateConfig` fusionne, sa valeur enregistrée reste. Un nom inconnu laisse le champ visible —
//! se tromper de nom montre un champ de trop, jamais un champ qui manque.
//!
//! `Section::visibleWhen` suit la même règle pour un bloc entier : la section et tous ses champs
//! disparaissent, et la colonne de navigation de la page de réglages ne la liste plus.
//!
//! ## Trois saisies de formulaire hôte
//!
//! Leur valeur est une chaîne, comme celle de tout champ : le formulaire reste plat.
//!
//! - `TimeRange` : un créneau `HH:MM-HH:MM` (« 16:00-19:00 »), `min` / `max` bornent les deux
//!   heures. Une fin avant le début passe minuit. Lire avec [`hours::parse_range`].
//! - `WeeklyHours` : des plages par jour, `mon=07:00-12:00,14:00-22:00;tue=…` ; un jour absent
//!   ou vide est fermé, `maxRanges` borne les plages d'un jour (3 par défaut). Lire et écrire avec
//!   [`hours::parse_week`] / [`hours::format_week`].
//! - `PhoneInput` : un numéro E.164 (« +33612345678 »), l'indicatif choisi à côté du numéro ;
//!   `defaultCountry` (ISO 3166, « FR ») est l'indicatif proposé d'abord.
//!
//! ## Un fichier joint — `FileUpload`
//!
//! Comme `ImageUpload`, mais pour un document : le tableau de bord envoie le fichier à la
//! plateforme et le champ prend sa référence `portaki-file:<id>` — le module ne voit jamais les
//! octets. `accept` est la liste des types MIME admis, séparés par des virgules
//! (`"application/pdf"`) ; `maxBytes` la taille au-delà de laquelle le tableau de bord refuse
//! avant tout envoi. Ce contrôle est un confort : la plateforme relit les octets à la porte.
//!
//! ```
//! use portaki_sdk::sdui::{primitives::FileUpload, Component};
//!
//! let pdf: Component = FileUpload::new().name("devices.0.manualFile")
//!     .accept("application/pdf").maxBytes(20 * 1024 * 1024).into();
//! let wire = serde_json::to_value(&pdf).unwrap();
//! assert_eq!(wire["type"], "FileUpload");
//! assert_eq!(wire["maxBytes"], 20_971_520);
//! ```
//!
//! ## Une longue liste en tableau — `DataTable` éditable
//!
//! Avec `rowName`, un `DataTable` édite la liste `rowName` du formulaire au lieu de seulement
//! l'afficher (règles communes §2.4 bis : tableau + panneau latéral). Chaque objet de `rows` est
//! une ligne : `id` obligatoire, puis les valeurs des colonnes.
//!
//! - **Le panneau d'une ligne est l'enfant de même rang** : `children[i]` est le formulaire de
//!   `rows[i]`, du SDUI ordinaire dont les champs se nomment `{rowName}.{i}.{clé}` et portent
//!   leur valeur et leur `Field::error`. Le tableau de bord l'ouvre dans un panneau latéral
//!   (~480 px) au clic sur la ligne, avec Précédent / Suivant / Supprimer.
//! - **Chaque ligne envoie au moins son id** (`{rowName}.{i}.id`) : une ligne jamais ouverte
//!   n'envoie que lui, et le module (ou la plateforme, pour un champ `structured` à `item.id`)
//!   garde ce qu'elle avait. Supprimer retire la ligne du formulaire.
//! - **Colonnes** (`columns`, une liste d'objets) : `key`, `label`, et `kind` — `text` par
//!   défaut ; `check`, une case liée à `{rowName}.{i}.{key}` (valeur booléenne de la ligne), que
//!   `max` borne en nombre de cases cochées (au-delà, les autres se désactivent et `maxMessage`
//!   s'affiche) ; `pill`, une pastille quand la valeur n'est pas vide. Une colonne `text` peut
//!   porter `sub` (la clé d'une autre valeur de la ligne, en seconde ligne discrète : la précision
//!   sous la règle) et `mono: true` (chiffres à chasse fixe : des heures).
//! - `searchPlaceholder` ajoute une recherche sur le texte des cellules ; `filterKey` des
//!   pastilles de filtre, une par valeur distincte de cette colonne ; `addLabel` + `addAction`,
//!   un bouton « Ajouter » : le module rend une ligne de plus (avec son id), et le tableau de bord
//!   ouvre son panneau.
//!
//! Sans `rowName`, la table reste en lecture seule, comme avant.
//!
//! ```
//! use portaki_sdk::sdui::primitives::{DataTable, Field, TextInput};
//! use serde_json::json;
//!
//! let table = DataTable::new()
//!     .rowName("devices")
//!     .columns(json!([{ "key": "name", "label": "Appareil" },
//!                     { "key": "featured", "label": "Mis en avant", "kind": "check", "max": 4 }]))
//!     .rows(json!([{ "id": "d1", "name": "Lave-linge", "featured": true }]))
//!     .child(Field::new().name("devices.0.name")
//!         .child(TextInput::new().name("devices.0.name").value("Lave-linge")));
//! let wire = serde_json::to_value(&table).unwrap();
//! assert_eq!(wire["rowName"], "devices");
//! assert_eq!(wire["children"].as_array().unwrap().len(), 1);
//! ```
//!
//! ### Une liste ordonnée — `sortable`, `dividerAfter`, `panel`
//!
//! - `sortable` : l'hôte réordonne les lignes (poignée glissée, ou flèches ↑ ↓ sur la poignée au
//!   clavier). Le formulaire suit l'ordre affiché : la ligne montrée en position `i` part en
//!   `{rowName}.{i}`, avec son `id` — le module enregistre la liste dans l'ordre reçu. Le panneau
//!   d'une ligne déplacée écrit à son nouveau rang : le module nomme toujours ses champs au rang
//!   où il rend la ligne, le tableau de bord les renumérote.
//! - `dividerAfter` + `dividerLabel` : un filet titré après les `dividerAfter` premières lignes
//!   (« Sur la carte d'accueil » au-dessus, le reste en dessous). Pris tel que rendu : il suit le
//!   réglage au rendu suivant du module, pas pendant la saisie. Masqué pendant une recherche ou un
//!   filtre.
//! - `panel` : `"sheet"` (défaut, le panneau latéral) ou `"dialog"`, une fenêtre centrée pour un
//!   formulaire court — sans Précédent / Suivant, avec Supprimer, Annuler (rend à la ligne ses
//!   valeurs d'avant l'ouverture ; retire une ligne tout juste ajoutée) et Enregistrer.
//!
//! ```
//! use portaki_sdk::sdui::primitives::DataTable;
//! use serde_json::json;
//!
//! let rules = DataTable::new()
//!     .rowName("rules")
//!     .columns(json!([{ "key": "text", "label": "Règle" }]))
//!     .rows(json!([{ "id": "r1", "text": "Pas de fête" }, { "id": "r2", "text": "Non-fumeur" }]))
//!     .sortable(true)
//!     .dividerAfter(4)
//!     .dividerLabel("Sur la carte d'accueil")
//!     .panel("dialog");
//! let wire = serde_json::to_value(&rules).unwrap();
//! assert_eq!(wire["sortable"], true);
//! assert_eq!(wire["dividerAfter"], 4);
//! assert_eq!(wire["panel"], "dialog");
//! ```
//!
//! ## Un mois en grille — `MonthCalendar`
//!
//! Un mois, sept colonnes du lundi au dimanche, des pastilles dans les jours : le calendrier des
//! événements, des séjours, du ménage. Une feuille, comme `Chart` : ses données voyagent dans ses
//! champs, jamais dans `children`.
//!
//! - `month` : `AAAA-MM`. Le tableau de bord écrit lui-même le titre (« Août 2026 »), les jours
//!   de la semaine et les jours hors du mois (grisés) ; sur un téléphone, la grille devient une
//!   liste des jours qui ont quelque chose.
//! - `days` : **seulement les jours qui ont quelque chose**, chacun avec sa date `AAAA-MM-JJ` et
//!   ses pastilles ([`CalendarItem`]) dans l'ordre d'affichage. Une date hors de `month` est
//!   ignorée.
//! - Une pastille : `label`, `tone` (d'où elle vient : `neutral` par défaut, `info`, `warning`…),
//!   `strike` (barrée : annulée), `muted` (grisée : passée) et `action` — sans action, elle
//!   n'est pas un bouton.
//! - `prevAction` / `nextAction` : les boutons ‹ › ; le module rend le mois demandé (typiquement
//!   `emit` de `contracts::shell::SURFACE_INPUT` avec le mois voisin). Sans eux, pas de bouton.
//! - `legend` : des [`CalendarItem`] sous la grille, dessinés comme les pastilles (`id` et
//!   `action` y sont ignorés).
//!
//! La semaine commence le lundi.
//!
//! ```
//! use portaki_sdk::sdui::primitives::MonthCalendar;
//! use portaki_sdk::sdui::{CalendarDay, CalendarItem, Component, Tone};
//!
//! let august: Component = MonthCalendar::new()
//!     .month("2026-08")
//!     .days(vec![CalendarDay::new("2026-08-15", vec![
//!         CalendarItem::new("oa-1", "Feu d'artifice").tone(Tone::Info),
//!         CalendarItem::new("e1", "Fête du jardin").strike(true),
//!     ])])
//!     .legend(vec![CalendarItem::new("", "Importé").tone(Tone::Info)])
//!     .into();
//! let wire = serde_json::to_value(&august).unwrap();
//! assert_eq!(wire["type"], "MonthCalendar");
//! assert_eq!(wire["days"][0]["items"][1]["strike"], true);
//! ```
//!
//! ## Une erreur sous le champ — `Field::error`
//!
//! `Field::error` est le message affiché sous le champ, à la place de son aide (`FieldHint`) :
//! ce qui ne va pas et quoi faire (« 60 caractères au maximum. »). Le module rend son formulaire
//! depuis le brouillon de config : il calcule l'erreur au rendu, avec les validateurs de
//! [`crate::config::check`], et la même règle bloque « Publier » via sa query `publishReadiness`.
//! Vide ou absent, pas d'erreur.
//!
//! ## What modules must not assume
//!
//! - Not every shell implements every primitive — stick to the documented catalog.
//! - `Action::Command` crosses the Wasm boundary again; keep args small and JSON-serializable.
//! - Overlay surfaces (`Action::OpenOverlay`) are rendered in a second invocation —
//!   do not rely on in-memory state from the parent surface.
//!
//! # Examples
//!
//! ```ignore
//! use portaki_sdk::prelude::*;
//! use portaki_sdk::sdui::primitives::{Button, Card, Stack, Text};
//! use portaki_sdk::sdui::action::Action;
//!
//! fn render(ctx: HostContext) -> Surface {
//!     // Call sites: module `ids::*` or `contracts::*` OperationName consts.
//!     let refresh = Action::command(&ctx.module_id, ids::UPDATE_CONFIG, EmptyArgs {});
//!     Surface::new(
//!         Card::new()
//!             .child(Stack::new().child(Text::new()))
//!             .child(Button::new().action(refresh)),
//!     )
//! }
//! ```

pub mod action;
pub mod common;
pub mod component;
pub mod hours;
pub mod primitives;
pub mod surface;

pub use action::{json_value, Action, EmptyArgs, NavigateTarget, OverlayArgs, OverlayPresentation};
pub use common::{
    AccordionItem, ActionRowItem, Animation, AnimationKind, ButtonVariant, CalendarDay,
    CalendarItem, ChartEmpty, ChartKind, ChartPoint, ChoiceListLayout, ChoiceOption, DeltaTone,
    EditableListItem, Emphasis, FeedStatus, FilterBarChip, GeoPoint, ImageSize, MapClustering,
    MapInteractionMode, MapMarker, MapMarkerKind, MapViewport, RichTextDoc, StackDirection,
    SurfaceLevel, Swatch, TabBarItem, TabItem, TempVariant, TemperatureUnit, TextVariant, Tone,
    Visibility, VisibilityExpr,
};
pub use component::Component;
pub use surface::Surface;

/// The hidden field carrying a list row's id in a host form: `<list>.<index>.id`.
///
/// A form sends rows by position; with this field, each row also sends its id, and the platform
/// merges a save into the stored row with the same id (`item.id` of a `structured` config field)
/// — a removal or a reorder no longer shifts the other rows' translations and kept sub-keys. The
/// dashboard renders a `TextInput` named `….id` hidden and sends it back untouched.
///
/// `id` is the stored row's; a new row (`None` or blank) gets a fresh UUID. Put it in every row
/// of a `StepList` (or any repeated group), next to the row's other fields.
///
/// ```
/// use portaki_sdk::sdui::{primitives::TextInput, row_id, primitives::SduiPrimitive};
///
/// let kept = row_id("steps", 0, Some("gate"));
/// let input = TextInput::from_component(&kept).unwrap();
/// assert_eq!(input.name.as_deref(), Some("steps.0.id"));
/// assert_eq!(input.value.as_deref(), Some("gate"));
///
/// let added = row_id("steps", 1, None);
/// assert_eq!(TextInput::from_component(&added).unwrap().value.as_ref().unwrap().len(), 36);
/// ```
pub fn row_id(list: &str, index: usize, id: Option<&str>) -> Component {
    let id = match id.map(str::trim) {
        Some(id) if !id.is_empty() => id.to_string(),
        _ => uuid::Uuid::new_v4().to_string(),
    };
    primitives::TextInput::new()
        .name(format!("{list}.{index}.id"))
        .value(id)
        .into()
}
