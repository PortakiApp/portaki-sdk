//! A module that passes the conformance battery — through the macro modules call.
//!
//! The handlers live in this test binary, so the battery sees them without `extern crate`:
//! `dir =` points it at the fixture's manifest and bundles instead of this crate's.

use portaki_sdk::email::EmailContextArgs;
use portaki_sdk::prelude::*;
use portaki_sdk::sdui::primitives::{Button, Card, EmptyState, Form, Stack, Text};

#[portaki_sdk::surface(guest, id = "home.card")]
pub fn render_home_card(_ctx: GuestContext) -> Surface {
    let title = portaki_sdk::t!("guest.empty.title").unwrap_or_default();
    Surface::new(
        Card::new()
            .title("i18n:nav.fixture")
            .child(EmptyState::new().title(title)),
    )
}

#[portaki_sdk::surface(guest, id = "explore.detail")]
pub fn render_explore_detail(_ctx: GuestContext) -> Surface {
    Surface::new(Stack::new().child(Text::new().text("i18n:guest.empty.title")))
}

#[portaki_sdk::surface(host, id = "main")]
pub fn render_host_main(_ctx: HostContext) -> Result<Surface> {
    let saved = host::kv::get("config")?;
    Ok(Surface::new(
        Card::new().title("i18n:host.title").child(
            Form::new()
                .child(Text::new().text(if saved.is_some() { "saved" } else { "new" }))
                .child(Button::new().label("i18n:host.save")),
        ),
    ))
}

#[derive(Debug, Deserialize)]
pub struct UpdateConfigArgs {
    pub title: String,
}

/// Refuses `{}` with an error — which is what empty input deserves.
#[portaki_sdk::command(name = "updateConfig")]
pub fn update_config(_ctx: Context, args: UpdateConfigArgs) -> Result<()> {
    host::kv::set("config", args.title.as_bytes(), None)
}

#[portaki_sdk::command(name = "sendReminder")]
pub fn send_reminder(ctx: Context) -> Result<()> {
    let Some(stay) = ctx.stay else {
        return Err(PortakiError::Host("no stay".into()));
    };
    let _subject = portaki_sdk::t!("email.reminder.subject")?;
    let _ = stay.checkin_at;
    Ok(())
}

#[portaki_sdk::query(name = "emailContext")]
pub fn email_context(_ctx: Context, args: EmailContextArgs) -> Result<serde_json::Value> {
    Ok(serde_json::json!({ "template": args.template_key }))
}

portaki_test_utils::conformance!(
    dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/passing")
);

#[test]
fn every_check_passes_together() {
    let module = portaki_test_utils::conformance::Module::at(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/passing"
    ));
    if let Err(findings) = module.check_all() {
        panic!("{findings}");
    }
}

/// The battery is not passing on an empty list: every handler above is visible to it.
#[test]
fn the_battery_sees_every_declared_handler() {
    use portaki_sdk::wasm::registry::{declarations, HandlerKind};

    let mut seen: Vec<(HandlerKind, &str, &str)> = declarations()
        .map(|declaration| (declaration.kind, declaration.context, declaration.name))
        .collect();
    seen.sort_by_key(|(_, context, name)| (*context, *name));

    assert_eq!(
        seen,
        vec![
            (HandlerKind::Query, "", "emailContext"),
            (HandlerKind::Command, "", "sendReminder"),
            (HandlerKind::Command, "", "updateConfig"),
            (HandlerKind::Surface, "guest", "explore.detail"),
            (HandlerKind::Surface, "guest", "home.card"),
            (HandlerKind::Surface, "host", "main"),
        ]
    );
}
