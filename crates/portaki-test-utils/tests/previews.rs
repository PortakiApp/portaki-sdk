//! `previews`: every guest route rendered, compared to the committed `previews.json`.

use portaki_sdk::prelude::*;
use portaki_sdk::sdui::primitives::{Card, RichText, Text};
use portaki_test_utils::previews;

/// A route: rendered in the previews, with its title.
#[portaki_sdk::surface(guest, id = "explore.detail", path = "wifi", label_key = "nav.wifi")]
pub fn render_detail(ctx: GuestContext) -> Surface {
    let ssid = ctx
        .module_config
        .as_ref()
        .and_then(|config| config["ssid"].as_str())
        .unwrap_or_default()
        .to_string();
    Surface::new(
        Card::new()
            .title("i18n:guest.detail.title")
            .child(Text::new().text(format!("{ssid} · {}", Uuid::from_u128(7)))),
    )
}

/// Not a route (no path), not a guest surface: not previewed.
#[portaki_sdk::surface(guest, id = "home.card")]
pub fn render_home(_ctx: GuestContext) -> Surface {
    Surface::new(Text::new().text("home"))
}

#[portaki_sdk::surface(host, id = "main", label_key = "nav.wifi")]
pub fn render_main(_ctx: HostContext) -> Surface {
    Surface::new(Text::new().text("host"))
}

const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/previews");

fn context() -> portaki_test_utils::MockContextBuilder {
    previews::guest(ROOT).with_config(&serde_json::json!({ "ssid": "Maison-Invites" }))
}

#[test]
fn every_route_is_rendered_and_matches_the_file() {
    previews::check_all(ROOT, context());
}

#[test]
fn surfaces_rendered_by_hand_compare_the_same() {
    let detail = context().run(render_detail);
    previews::check(ROOT, vec![("explore.detail", detail)]);
}

#[test]
#[should_panic(expected = "one preview per guest surface")]
fn a_missing_route_fails() {
    previews::check(ROOT, Vec::new());
}

/// A `content` built from data is where pre-rendered HTML lands — the conformance battery's empty
/// mock renders the empty state instead, and would not see it.
#[test]
#[should_panic(expected = "carries a RichText")]
fn pre_rendered_markup_in_a_preview_fails() {
    let detail = Surface::new(RichText::new().content("<p>Appuyez 2 secondes.</p>"));
    previews::check(ROOT, vec![("explore.detail", detail)]);
}
