//! Guest booklet surfaces.

use portaki_sdk::prelude::*;
use portaki_sdk::sdui::primitives::{Card, Stack, Text};

use crate::ids::HOME_CARD;

#[portaki_sdk::surface(guest, id = "home.card")]
pub fn render_guest_home_card(ctx: GuestContext) -> Surface {
    let _ = ctx;
    Surface::new(
        Card::new()
            .title("i18n:home.card.title")
            .child(Stack::new().child(Text::new().text("i18n:home.card.body"))),
    )
    .with_id(HOME_CARD)
}
