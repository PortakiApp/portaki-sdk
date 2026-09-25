//! What a guest sees when a surface cannot show its content.
//!
//! `#[surface(guest, …)]` wraps the render function in [`render`]: before calling it, the SDK asks
//! the platform whether the module is ready ([`crate::host::module::status`]) and, when it is
//! not, renders one of its own states instead — the render function is not called. When the
//! function returns an `Err`, the SDK logs it and renders the error state. No `guest/empty.rs`,
//! no `match … { Err => … }` around every surface, no status keys to copy into each bundle.
//!
//! | State | When | Icon | Keys |
//! |-------|------|------|------|
//! | inactive | the module is off on the property or the workspace | the module's (`portaki_module!(icon = …)`) | `module.status.inactive.title`, `.description` |
//! | incomplete | a required config field is empty | `IconName::Sliders` | `module.status.incomplete.title`, `.description`, `.hint` |
//! | error | the render function (or the status read) failed | the module's | `guest.error.title`, `.description`, `.hint` |
//!
//! The SDK ships every text of these states in the languages of [`LANGUAGES`] — see [`TEXTS`]. A
//! key of the same name in the module's own bundle wins: a module that wants its own wording
//! declares it, the others declare nothing. A text the SDK leaves empty (the incomplete `hint`)
//! is shown only when the module declares it.
//!
//! The failure is logged at `error` level as `<module id>_<surface id>_render_failed`
//! (non-alphanumerics as `_`, e.g. `wifi_guest_home_card_render_failed`) with the fields
//! `surfaceId` and `error`.
//!
//! Reading the status is the `module.status` host operation, which the `platform` permission
//! (cargo feature) opens. Without that feature the SDK cannot know, and treats the module as
//! ready: only the error state applies.
//!
//! A surface opts out with `#[surface(guest, id = "…", gate = false)]` — see
//! [`macro@crate::surface`].

use crate::context::Context;
use crate::error::{PortakiError, Result};
use crate::host;
use crate::sdui::common::TextVariant;
use crate::sdui::primitives::{EmptyState, Text};
use crate::sdui::surface::Surface;
use crate::vocab::IconName;

/// The icon `portaki_module!(icon = …)` declares, registered for the guest states.
#[doc(hidden)]
pub struct ModuleIcon(pub IconName);

inventory::collect!(ModuleIcon);

/// The icon when `portaki_module!` names none.
const FALLBACK_ICON: IconName = IconName::InfoCircle;

/// The languages the SDK ships the state texts in, in the column order of [`TEXTS`]. Any other
/// language reads the English text.
pub const LANGUAGES: [&str; 6] = ["en", "fr", "es", "de", "it", "nl"];

/// Every key of the guest states, with its text in each of [`LANGUAGES`]. Empty: no text unless
/// the module declares one.
pub const TEXTS: [(&str, [&str; 6]); 8] = [
    (
        "module.status.inactive.title",
        [
            "Not available",
            "Indisponible",
            "No disponible",
            "Nicht verfügbar",
            "Non disponibile",
            "Niet beschikbaar",
        ],
    ),
    (
        "module.status.inactive.description",
        [
            "This section is not offered at this property.",
            "Cette rubrique n’est pas proposée dans ce logement.",
            "Esta sección no está disponible en este alojamiento.",
            "Dieser Bereich wird in dieser Unterkunft nicht angeboten.",
            "Questa sezione non è disponibile in questo alloggio.",
            "Dit onderdeel wordt in deze accommodatie niet aangeboden.",
        ],
    ),
    (
        "module.status.incomplete.title",
        [
            "Coming soon",
            "Bientôt disponible",
            "Próximamente",
            "Bald verfügbar",
            "Presto disponibile",
            "Binnenkort beschikbaar",
        ],
    ),
    (
        "module.status.incomplete.description",
        [
            "Your host has not finished setting this up yet.",
            "Votre hôte n’a pas encore fini de préparer cette rubrique.",
            "Tu anfitrión aún no ha terminado de preparar esta sección.",
            "Ihr Gastgeber hat diesen Bereich noch nicht fertig eingerichtet.",
            "Il tuo host non ha ancora finito di preparare questa sezione.",
            "Je host heeft dit onderdeel nog niet helemaal ingericht.",
        ],
    ),
    ("module.status.incomplete.hint", ["", "", "", "", "", ""]),
    (
        "guest.error.title",
        [
            "Temporarily unavailable",
            "Momentanément indisponible",
            "No disponible temporalmente",
            "Vorübergehend nicht verfügbar",
            "Temporaneamente non disponibile",
            "Tijdelijk niet beschikbaar",
        ],
    ),
    (
        "guest.error.description",
        [
            "This information could not be loaded.",
            "Ces informations n’ont pas pu être chargées.",
            "No se ha podido cargar esta información.",
            "Diese Informationen konnten nicht geladen werden.",
            "Non è stato possibile caricare queste informazioni.",
            "Deze informatie kon niet worden geladen.",
        ],
    ),
    (
        "guest.error.hint",
        [
            "Please try again in a moment.",
            "Réessayez dans un instant.",
            "Vuelve a intentarlo en un momento.",
            "Bitte versuchen Sie es gleich noch einmal.",
            "Riprova tra un momento.",
            "Probeer het zo meteen opnieuw.",
        ],
    ),
];

/// The SDK's text for `key` in `locale` (`fr-FR`, `de`…), English for another language; `None`
/// for a key that is not a state key, or a text the SDK leaves to the module.
pub fn default_text(key: &str, locale: &str) -> Option<&'static str> {
    let language = locale
        .split(['-', '_'])
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    let column = LANGUAGES
        .iter()
        .position(|known| *known == language)
        .unwrap_or(0);
    TEXTS
        .iter()
        .find(|(known, _)| *known == key)
        .map(|(_, texts)| texts[column])
        .filter(|text| !text.is_empty())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(not(feature = "platform"), allow(dead_code))]
enum State {
    Inactive,
    Incomplete,
    Error,
}

/// Renders a guest surface behind the module's readiness, with the error state as a floor.
///
/// Called by the code `#[surface(guest, …)]` generates — a module does not call it.
#[doc(hidden)]
pub fn render(
    ctx: Context,
    surface_id: &str,
    render: impl FnOnce(Context) -> Result<Surface>,
) -> Surface {
    let locale = ctx.locale.clone();
    let module_id = ctx.module_id.to_string();
    let error = match not_ready() {
        Ok(Some(state)) => return state_surface(state, surface_id, &locale),
        Ok(None) => match render(ctx) {
            Ok(surface) => return surface,
            Err(error) => error,
        },
        Err(error) => error,
    };
    log_failure(&module_id, surface_id, &error);
    state_surface(State::Error, surface_id, &locale)
}

#[cfg(feature = "platform")]
fn not_ready() -> Result<Option<State>> {
    let status = host::module::status()?;
    Ok(if !status.active || !status.workspace_enabled {
        Some(State::Inactive)
    } else if status.incomplete {
        Some(State::Incomplete)
    } else {
        None
    })
}

/// Without the `platform` permission the status cannot be read: the module is taken as ready.
#[cfg(not(feature = "platform"))]
fn not_ready() -> Result<Option<State>> {
    Ok(None)
}

/// `<module id>_<surface id>_render_failed`, non-alphanumerics as `_`.
pub(crate) fn failure_event(module_id: &str, surface_id: &str) -> String {
    format!("{module_id}_{surface_id}_render_failed")
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect()
}

fn log_failure(module_id: &str, surface_id: &str, error: &PortakiError) {
    let mut fields = host::log::Fields::new();
    fields.insert("surfaceId", &surface_id);
    fields.insert("error", &error.to_string());
    // Un journal qui échoue ne doit pas priver le voyageur de l'état d'erreur.
    let _ = host::log::error(&failure_event(module_id, surface_id), &fields);
}

fn state_surface(state: State, surface_id: &str, locale: &str) -> Surface {
    let (prefix, icon) = match state {
        State::Inactive => ("module.status.inactive", module_icon()),
        State::Incomplete => ("module.status.incomplete", IconName::Sliders),
        State::Error => ("guest.error", module_icon()),
    };
    let text = |part: &str| text(&format!("{prefix}.{part}"), locale);
    let mut empty = EmptyState::new().icon(icon);
    if let Some(title) = text("title") {
        empty = empty.title(title);
    }
    if let Some(description) = text("description") {
        empty = empty.description(description);
    }
    if let Some(hint) = text("hint") {
        empty = empty.child(Text::new().text(hint).variant(TextVariant::Body));
    }
    Surface {
        id: Some(surface_id.to_string()),
        root: empty.into(),
    }
}

/// The module's own text when its bundle has `key`, the SDK's otherwise.
fn text(key: &str, locale: &str) -> Option<String> {
    // Le runtime renvoie la clé elle-même quand le bundle ne l'a pas.
    let own = crate::t!(key)
        .ok()
        .filter(|text| text != key && !text.trim().is_empty());
    own.or_else(|| default_text(key, locale).map(str::to_string))
}

fn module_icon() -> IconName {
    inventory::iter::<ModuleIcon>
        .into_iter()
        .next()
        .map_or(FALLBACK_ICON, |icon| icon.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_state_text_exists_in_every_language_but_the_optional_hint() {
        for (key, texts) in TEXTS {
            for (language, text) in LANGUAGES.iter().zip(texts) {
                assert_eq!(
                    text.is_empty(),
                    key == "module.status.incomplete.hint",
                    "{key} in {language}"
                );
            }
        }
    }

    #[test]
    fn a_locale_reads_its_language_and_an_unknown_one_english() {
        assert_eq!(
            default_text("guest.error.title", "fr-FR"),
            Some("Momentanément indisponible")
        );
        assert_eq!(
            default_text("guest.error.title", "de"),
            Some("Vorübergehend nicht verfügbar")
        );
        assert_eq!(
            default_text("guest.error.title", "pt-BR"),
            Some("Temporarily unavailable")
        );
        assert_eq!(default_text("module.status.incomplete.hint", "fr"), None);
        assert_eq!(default_text("home.card.title", "fr"), None);
    }

    #[test]
    fn the_failure_event_is_named_after_the_module_and_the_surface() {
        assert_eq!(
            failure_event("wifi-guest", "home.card"),
            "wifi_guest_home_card_render_failed"
        );
    }
}
