//! A module that passes the conformance battery — through the macro modules call.
//!
//! The handlers live in this test binary, so the battery sees them without `extern crate`:
//! `dir =` points it at the fixture's manifest and bundles instead of this crate's.

use portaki_sdk::contracts::i18n::I18nText;
use portaki_sdk::contracts::publish::{PublishCheck, PublishLevel, PublishReadiness};
use portaki_sdk::contracts::stats::{self, StatsSummary, StatsSummaryArgs};
use portaki_sdk::contracts::timeline::{
    self, TaskToggleArgs, TimelineTaskItem, TimelineTasks, TimelineTasksArgs,
};
use portaki_sdk::email::EmailContextArgs;
use portaki_sdk::prelude::*;
use portaki_sdk::sdui::primitives::{Button, Card, EmptyState, Form, Select, Stack, Text};

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

#[portaki_sdk::surface(host, id = "reports")]
pub fn render_reports(ctx: HostContext) -> Surface {
    let days = ctx.input_u64("periodDays").unwrap_or(30);
    Surface::new(
        Stack::new()
            .child(Text::new().text(days.to_string()))
            .child(
                Select::new()
                    .name("period")
                    .options(vec![
                        ChoiceOption::new("30", "30"),
                        ChoiceOption::new("90", "90"),
                    ])
                    .value("30"),
            ),
    )
}

#[portaki_sdk::query(name = "statsSummary")]
pub fn stats_summary(_ctx: Context, args: StatsSummaryArgs) -> Result<StatsSummary> {
    Ok(stats::summary(
        args.period.to_string(),
        I18nText::new("signalements", "reports"),
    ))
}

#[portaki_sdk::query(name = "timelineTasks")]
pub fn timeline_tasks(_ctx: Context, args: TimelineTasksArgs) -> Result<TimelineTasks> {
    let tasks = args
        .stays
        .iter()
        .map(|stay| {
            timeline::task(
                format!("cleaning:{}", stay.id),
                stay.check_out,
                args.property_id,
                I18nText::new("Ménage", "Cleaning"),
                I18nText::new("Après le départ", "After check-out"),
            )
            .stay(stay.id)
            .items(cleaning_items())
        })
        .collect();
    Ok(TimelineTasks { tasks })
}

fn cleaning_items() -> Vec<TimelineTaskItem> {
    vec![
        TimelineTaskItem::new("floors", I18nText::new("Sols", "Floors")),
        TimelineTaskItem::new("living-room", I18nText::new("Salon", "Living room"))
            .photo_required(),
    ]
}

#[portaki_sdk::command(name = "taskToggle")]
pub fn task_toggle(_ctx: Context, args: TaskToggleArgs) -> Result<()> {
    let item = cleaning_items()
        .into_iter()
        .find(|item| item.id == args.item_id)
        .ok_or(PortakiError::Host("unknown item".into()))?;
    item.check_toggle(args.done, args.photo.as_deref())?;
    host::kv::set(&format!("{}/{}", args.task_id, args.item_id), b"1", None)
}

#[portaki_sdk::query(name = "publishReadiness")]
pub fn publish_readiness(_ctx: Context) -> Result<PublishReadiness> {
    let saved = host::kv::get("config")?;
    Ok(PublishReadiness {
        items: vec![PublishCheck {
            id: "title".into(),
            level: PublishLevel::Required,
            ok: saved.is_some(),
            label: I18nText::new("Titre", "Title"),
            hint: I18nText::new("Il manque le titre", "The title is missing"),
        }],
    })
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
            (HandlerKind::Query, "", "publishReadiness"),
            (HandlerKind::Command, "", "sendReminder"),
            (HandlerKind::Query, "", "statsSummary"),
            (HandlerKind::Command, "", "taskToggle"),
            (HandlerKind::Query, "", "timelineTasks"),
            (HandlerKind::Command, "", "updateConfig"),
            (HandlerKind::Surface, "guest", "explore.detail"),
            (HandlerKind::Surface, "guest", "home.card"),
            (HandlerKind::Surface, "host", "main"),
            (HandlerKind::Surface, "host", "reports"),
        ]
    );
}
