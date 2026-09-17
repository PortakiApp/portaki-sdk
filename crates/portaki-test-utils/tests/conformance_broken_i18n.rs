//! Keys used by a surface, a handler or the manifest must exist in `fr` and `en`.

mod common;

use common::{assert_reports, broken, failing, passing};
use portaki_sdk::prelude::*;
use portaki_sdk::sdui::primitives::{Card, Text};

#[portaki_sdk::surface(guest, id = "home.card")]
pub fn render_home_card(_ctx: GuestContext) -> Surface {
    Surface::new(
        Card::new()
            .title("i18n:guest.nowhere")
            .child(Text::new().text("i18n:nav.fixture")),
    )
}

#[portaki_sdk::surface(host, id = "main")]
pub fn render_host_main(_ctx: HostContext) -> Result<Surface> {
    let label = portaki_sdk::t!("fixture.only_fr")?;
    Ok(Surface::new(Text::new().text(label)))
}

#[test]
fn a_rendered_key_missing_everywhere_is_reported_for_both_languages() {
    let findings = failing("i18n", passing().check_i18n());

    assert_reports(
        &findings,
        &["`guest.nowhere`", "fr and en", "guest surface `home.card`"],
    );
}

#[test]
fn a_translated_key_missing_in_one_language_names_it() {
    let findings = failing("i18n", passing().check_i18n());

    assert_reports(
        &findings,
        &["`fixture.only_fr`", "the en bundle", "host surface `main`"],
    );
    assert!(
        !findings
            .problems()
            .iter()
            .any(|p| p.contains("nav.fixture")),
        "a key present in both bundles is not a finding:\n{findings}"
    );
}

#[test]
fn a_manifest_label_key_is_checked_too() {
    let findings = failing("i18n", broken().check_i18n());

    assert_reports(
        &findings,
        &[
            "`nav.untranslated`",
            "the en bundle",
            "guestSurfaces `explore.missing`",
        ],
    );
}

#[test]
fn a_module_without_bundles_is_reported() {
    let bare = portaki_test_utils::conformance::Module::at(env!("CARGO_MANIFEST_DIR"));
    let findings = failing("i18n", bare.check_i18n());

    assert_reports(&findings, &["cannot read", "i18n"]);
}
