//! A `property.public` surface that is not static, or uses what the public page does not render,
//! fails `surfaces` — rendered for a visitor without a stay.

mod common;

use common::{assert_reports, failing, passing};
use portaki_sdk::prelude::*;
use portaki_sdk::sdui::primitives::{Button, ListItem, Section, Stack, Text};

#[portaki_sdk::surface(guest, id = "property.public")]
pub fn render_property_public(ctx: GuestContext) -> Surface {
    assert!(ctx.is_public_visitor(), "rendered with a guest context");
    Surface::new(
        Section::new().title("i18n:nav.fixture").child(
            Stack::new()
                .child(Section::new().child(Text::new().text("i18n:guest.empty.title")))
                .child(
                    ListItem::new()
                        .title("i18n:nav.fixture")
                        .action(Action::navigate(ids::convention::HOME_CARD, None)),
                )
                .child(Button::new().label("i18n:host.save")),
        ),
    )
}

#[test]
fn a_property_public_tree_off_the_rules_is_reported() {
    let findings = failing("surfaces", passing().check_surfaces());

    assert_reports(
        &findings,
        &[
            "guest surface `property.public`",
            "public visitor",
            "`Button`",
        ],
    );
    assert_reports(
        &findings,
        &[
            "`property.public`",
            "`Section` at $.children[0].children[0]",
        ],
    );
    assert_reports(&findings, &["`property.public`", "`action` at"]);
}
