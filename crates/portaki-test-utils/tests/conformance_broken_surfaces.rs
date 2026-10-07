//! Surfaces that panic, fail on a first install, or are routed but not served.

mod common;

use common::{assert_reports, broken, failing, passing};
use portaki_sdk::prelude::*;
use portaki_sdk::sdui::primitives::{RichText, Select, Stack, Text};

#[portaki_sdk::surface(guest, id = "home.card")]
pub fn render_home_card(ctx: GuestContext) -> Surface {
    // What an empty install looks like: no stay on a home card.
    let stay = ctx.stay.expect("a stay");
    Surface::new(Text::new().text(stay.stay_id.to_string()))
}

#[portaki_sdk::surface(host, id = "main")]
pub fn render_host_main(_ctx: HostContext) -> Result<Surface> {
    let raw = host::kv::get("config")?.ok_or(PortakiError::Host("no config yet".into()))?;
    Ok(Surface::new(
        Text::new().text(String::from_utf8_lossy(&raw)),
    ))
}

#[portaki_sdk::surface(guest, id = "explore.detail")]
pub fn render_explore_detail(_ctx: GuestContext) -> Surface {
    Surface::new(Text::new().text("i18n:guest.empty.title"))
}

/// The SDK turns the `Err` into its error state — which a first install should not show.
#[portaki_sdk::surface(guest, id = "explore.config")]
pub fn render_explore_config(_ctx: GuestContext) -> Result<Surface> {
    let raw = host::kv::get("config")?.ok_or(PortakiError::Host("nothing saved".into()))?;
    Ok(Surface::new(
        Text::new().text(String::from_utf8_lossy(&raw)),
    ))
}

/// Out of the SDK's shell, and nothing to say when the module is not ready.
#[portaki_sdk::surface(guest, id = "explore.ungated", gate = false)]
pub fn render_explore_ungated(_ctx: GuestContext) -> Result<Surface> {
    let status = host::module::status()?;
    if !status.is_ready() {
        return Ok(Surface::new(Stack::new()));
    }
    Ok(Surface::new(Text::new().text("i18n:guest.empty.title")))
}

#[portaki_sdk::surface(guest, id = "explore.picker")]
pub fn render_explore_picker(_ctx: GuestContext) -> Surface {
    Surface::new(
        Stack::new()
            .child(Select::new().name("empty"))
            .child(
                Select::new()
                    .name("size")
                    .options(vec![
                        ChoiceOption::new("s", "S"),
                        ChoiceOption::new("m", "M"),
                    ])
                    .value("xl"),
            )
            .child(
                Select::new()
                    .name("unset")
                    .options(vec![ChoiceOption::new("s", "S")])
                    .value(""),
            ),
    )
}

/// `content` is a TipTap field: the booklet renders anything else as literal text, tags included.
#[portaki_sdk::surface(guest, id = "explore.howto")]
pub fn render_explore_howto(_ctx: GuestContext) -> Surface {
    Surface::new(
        Stack::new()
            .child(RichText::new().content("<p>Appuyez 2 secondes sur la touche marche.</p>"))
            // A host's tip, shown as written — what `local-guide` sends.
            .child(RichText::new().content("Venez avant 9 h : les croissants partent vite."))
            // Translated before the booklet looks for a document.
            .child(RichText::new().content("i18n:guest.empty.title"))
            .child(RichText::new().content(
                r#"{"type":"doc","content":[{"type":"paragraph","content":[{"type":"text","text":"Appuyez."}]}]}"#,
            )),
    )
}

#[test]
fn pre_rendered_markup_in_a_rich_text_is_reported() {
    let findings = failing("surfaces", passing().check_surfaces());

    assert_reports(
        &findings,
        &["guest surface `explore.howto`", "`<p>`", "TipTap"],
    );
    // The plain tip, the `i18n:` reference and the TipTap document are not reported.
    assert_eq!(
        findings
            .problems()
            .iter()
            .filter(|problem| problem.contains("explore.howto"))
            .count(),
        1,
        "{findings}"
    );
}

/// The committed rendering is read too: a `content` built from data is not in the empty-mock tree.
#[test]
fn pre_rendered_markup_in_a_committed_rendering_is_reported() {
    let findings = failing("surfaces", broken().check_surfaces());

    assert_reports(&findings, &["previews.json", "`<p>`", "TipTap"]);
    // The plain tip beside it, and the module that commits no rendering at all, are not reported.
    assert_eq!(
        findings
            .problems()
            .iter()
            .filter(|problem| problem.contains("previews.json"))
            .count(),
        1,
        "{findings}"
    );
}

#[test]
fn a_select_without_options_or_with_a_stray_value_is_reported() {
    let findings = failing("surfaces", passing().check_surfaces());

    assert_reports(
        &findings,
        &["explore.picker", "Select `empty` without options"],
    );
    assert_reports(&findings, &["explore.picker", "Select `size`", "`xl`"]);
    assert!(
        !findings.problems().iter().any(|p| p.contains("`unset`")),
        "{findings}"
    );
}

#[test]
fn a_panicking_surface_is_named_with_its_message() {
    let findings = failing("surfaces", passing().check_surfaces());

    assert_reports(
        &findings,
        &[
            "guest surface `home.card` (render_home_card)",
            "panicked",
            "a stay",
        ],
    );
}

#[test]
fn a_surface_that_errors_on_an_empty_install_is_reported() {
    let findings = failing("surfaces", passing().check_surfaces());

    assert_reports(
        &findings,
        &[
            "host surface `main` (render_host_main)",
            "failed with an empty mock",
            "no config yet",
        ],
    );
}

#[test]
fn a_surface_that_renders_is_not_reported() {
    let findings = failing("surfaces", passing().check_surfaces());

    assert!(
        !findings
            .problems()
            .iter()
            .any(|p| p.contains("explore.detail")),
        "{findings}"
    );
}

#[test]
fn a_manifest_route_to_an_undeclared_surface_is_reported() {
    let findings = failing("surfaces", broken().check_surfaces());

    assert_reports(
        &findings,
        &["guestSurfaces `explore.missing`", "no #[surface(guest"],
    );
}

#[test]
fn an_error_the_sdk_shows_as_its_error_state_is_still_reported() {
    let findings = failing("surfaces", passing().check_surfaces());

    assert_reports(
        &findings,
        &[
            "guest surface `explore.config` (render_explore_config)",
            "failed with an empty mock",
            "nothing saved",
        ],
    );
}

#[test]
fn an_ungated_surface_is_rendered_in_every_guest_state() {
    let findings = failing("surfaces", passing().check_surfaces());

    assert_reports(
        &findings,
        &[
            "guest surface `explore.ungated`",
            "rendered nothing to read with the module inactive",
        ],
    );
    assert_reports(
        &findings,
        &[
            "`explore.ungated`",
            "nothing to read with the module incomplete",
        ],
    );
    assert_reports(
        &findings,
        &[
            "`explore.ungated`",
            "failed with the module error",
            "module_status_unavailable",
        ],
    );
    // Through the SDK's shell, every other guest surface shows a state.
    assert!(
        !findings
            .problems()
            .iter()
            .any(|p| p.contains("explore.detail")
                || p.contains("`home.card` (render_home_card) rendered")),
        "{findings}"
    );
}
