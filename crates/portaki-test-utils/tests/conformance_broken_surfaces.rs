//! Surfaces that panic, fail on a first install, or are routed but not served.

mod common;

use common::{assert_reports, broken, failing, passing};
use portaki_sdk::prelude::*;
use portaki_sdk::sdui::primitives::Text;

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
