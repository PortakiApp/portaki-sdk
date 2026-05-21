//! Default Portaki module template.

use portaki_sdk::prelude::*;
use portaki_sdk::sdui::primitives::{Card, Stack, Text};

portaki_sdk::portaki_module!(
    id = "{{MODULE_NAME}}",
    display_name_key = "module.displayName",
    description_key = "module.description",
    author = "Syntax Labs",
);

#[portaki_sdk::capability(required)]
pub const STORAGE: &str = capability::core::STORAGE;

#[portaki_sdk::entity(schema_version = 1)]
pub struct SampleItem {
    pub id: uuid::Uuid,
    pub title: String,
}

#[portaki_sdk::surface(guest, id = "home.card")]
pub fn render_guest_home_card(ctx: GuestContext) -> Surface {
    let _ = ctx;
    Surface::new(
        Card::new()
            .title(serde_json::json!("i18n:home.card.title"))
            .child(
                Stack::new().child(Text::new().text(serde_json::json!("i18n:home.card.body"))),
            ),
    )
    .with_id("home.card")
}

#[portaki_sdk::surface(host, id = "main")]
pub fn render_host_main(ctx: HostContext) -> Surface {
    let _ = ctx;
    Surface::new(Stack::new().child(Text::new())).with_id("main")
}
