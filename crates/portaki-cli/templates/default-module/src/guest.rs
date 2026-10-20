//! Guest booklet surfaces.

use portaki_sdk::prelude::*;
use portaki_sdk::sdui::primitives::{Card, Stack, Text};

use crate::config::ModuleConfig;

#[portaki_sdk::surface(
    guest,
    id = "home.card",
    path = "{{MODULE_NAME}}",
    label_key = "home.card.title",
    role = GuestRole::Card
)]
pub fn render_guest_home_card(ctx: GuestContext) -> Result<Surface> {
    // Written for the happy path: the SDK shows the guest its own state while the module is
    // off or incomplete, and turns an `Err` into a logged error state.
    let config = ModuleConfig::load(&ctx)?;
    // What the host typed, or the bundled wording — a card the host never configured still
    // reads as a finished card rather than an empty one.
    let body = if config.greeting.is_empty() {
        "i18n:home.card.body".to_string()
    } else {
        config.greeting
    };

    Ok(Surface::new(
        Card::new()
            .title("i18n:home.card.title")
            .child(Stack::new().child(Text::new().text(body))),
    ))
}
