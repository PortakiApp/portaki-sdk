//! Guest booklet surfaces.

use portaki_sdk::prelude::*;
use portaki_sdk::sdui::primitives::{Card, Stack, Text};

use crate::config::load_config;
use crate::ids::HOME_CARD;

#[portaki_sdk::surface(guest, id = "home.card")]
pub fn render_guest_home_card(_ctx: GuestContext) -> Surface {
    let config = load_config().unwrap_or_default();
    // What the host typed, or the bundled wording — a card the host never configured still
    // reads as a finished card rather than an empty one.
    let body = if config.greeting.is_empty() {
        "i18n:home.card.body".to_string()
    } else {
        config.greeting
    };

    Surface::new(
        Card::new()
            .title("i18n:home.card.title")
            .child(Stack::new().child(Text::new().text(body))),
    )
    .with_id(HOME_CARD)
}
