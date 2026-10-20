//! What a module adds to a record: several deposits from one form, which records are tasks, the
//! task's texts.

use chrono::Duration;
use portaki_sdk::contracts::i18n::I18nText;
use portaki_sdk::contracts::timeline::TimelineTasksArgs;
use portaki_sdk::resource::{Entry, RecordHooks};
use portaki_sdk::{Context, PortakiError, Result};
use portaki_test_utils::MockContext;
use serde_json::{json, Value};

#[portaki_sdk::choice(label = "status")]
pub enum Status {
    #[initial]
    Open,
    Planned,
    #[done]
    Restocked,
}

#[portaki_sdk::item(key = "product")]
pub struct Request {
    #[field(required, max_chars = 40, label = "form.product.label")]
    pub product: String,
}

#[portaki_sdk::record(name = "requests", item = Request, status = Status, hooks)]
pub struct Requests;

/// `{"products": [...]}` → one request per product, at most three per stay.
impl RecordHooks for Requests {
    type Item = Request;
    type Status = Status;

    fn deposits(ctx: &Context, args: &Value) -> Result<Vec<Request>> {
        let stay = ctx.guest.as_ref().map(|g| g.session_id).unwrap_or_default();
        if Requests::for_stay(ctx, stay)?.len() >= 3 {
            return Err(PortakiError::Host("requests_limit".into()));
        }
        args["products"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|p| portaki_sdk::resource::record::item_from(&json!({ "product": p }), "fr"))
            .collect()
    }

    fn needs_action(_: &Context, entry: &Entry<Request, Status>) -> bool {
        entry.status == Status::Open
    }

    fn task_context(entry: &Entry<Request, Status>) -> I18nText {
        I18nText::from(entry.guest_name.as_str())
    }
}

#[test]
fn one_form_deposits_several_records_and_the_module_caps_them() {
    let (ctx, host) = MockContext::guest().build();
    portaki_sdk::host::with_host(host, ctx.clone(), || {
        __portaki_requests_submit(ctx.clone(), json!({ "products": ["Café", "Thé"] })).unwrap();
        assert_eq!(Requests::list(&ctx).unwrap().len(), 2);
        // The cap is read once per form: 2 stored, 2 more asked, both pass.
        __portaki_requests_submit(ctx.clone(), json!({ "products": ["Sel", "Huile"] })).unwrap();
        assert_eq!(Requests::list(&ctx).unwrap().len(), 4);
        let refused = __portaki_requests_submit(ctx.clone(), json!({ "products": ["Sucre"] }));
        assert!(refused.unwrap_err().to_string().contains("requests_limit"));
        let invalid = __portaki_requests_submit(ctx.clone(), json!({ "products": [""] }));
        assert!(invalid.is_err());
    });
}

#[test]
fn only_what_needs_action_is_a_task_titled_from_the_item() {
    let (ctx, host) = MockContext::host().build();
    portaki_sdk::host::with_host(host, ctx.clone(), || {
        let stay = uuid::Uuid::new_v4();
        for product in ["Café", "Thé"] {
            __portaki_requests_add(ctx.clone(), json!({ "stayId": stay, "product": product }))
                .unwrap();
        }
        let planned = Requests::list(&ctx)
            .unwrap()
            .into_iter()
            .find(|e| e.item.product == "Thé")
            .unwrap()
            .id;
        portaki_sdk::resource::record::set_status::<Requests>(
            Requests::DECL,
            planned,
            Status::Planned,
            portaki_sdk::resource::Actor::Host,
        )
        .unwrap();
        let now = portaki_sdk::host::time::now().unwrap();
        let args = TimelineTasksArgs {
            property_id: ctx.property_id,
            from: now - Duration::days(1),
            to: now + Duration::days(1),
            stays: Vec::new(),
        };
        let decl = portaki_sdk::resource::RecordDecl {
            due_hours: Some(0),
            ..Requests::DECL
        };
        let tasks = portaki_sdk::resource::record::timeline_tasks::<Requests>(
            &ctx,
            decl,
            &args,
            I18nText::new("{product} demandé", "{product} requested"),
            "d".into(),
        )
        .unwrap()
        .tasks;
        assert_eq!(tasks.len(), 1, "a planned request is no task");
        assert_eq!(tasks[0].title.fr, "Café demandé");
        assert_eq!(tasks[0].title.en, "Café requested");
    });
}

#[portaki_sdk::choice(label = "tick")]
pub enum Tick {
    #[initial]
    Open,
    #[done]
    Done,
}

#[portaki_sdk::item(key = "step")]
pub struct Ticked {
    #[field(required, label = "step.label")]
    pub step: String,
    #[field(kind = "photo", label = "step.photo")]
    pub photo: String,
}

/// State per stay and step, no operation of its own.
#[portaki_sdk::record(name = "ticks", item = Ticked, status = Tick, commands = false)]
pub struct Ticks;

#[test]
fn marking_keeps_one_record_per_stay_and_key() {
    use portaki_sdk::resource::record::mark;
    use portaki_sdk::resource::Actor;
    let (ctx, host) = MockContext::host().build();
    portaki_sdk::host::with_host(host, ctx.clone(), || {
        let (stay, other) = (uuid::Uuid::new_v4(), uuid::Uuid::new_v4());
        let step = |photo: &str| Ticked {
            step: "floors".into(),
            photo: photo.into(),
            ..Default::default()
        };
        mark::<Ticks>(
            Ticks::DECL,
            stay,
            step("portaki-file:a"),
            Tick::Done,
            Actor::Host,
        )
        .unwrap();
        mark::<Ticks>(Ticks::DECL, stay, step(""), Tick::Open, Actor::Host).unwrap();
        mark::<Ticks>(Ticks::DECL, other, step(""), Tick::Done, Actor::Guest).unwrap();

        let mine = Ticks::for_stay(&ctx, stay).unwrap();
        assert_eq!(mine.len(), 1, "one record per stay and key");
        assert_eq!(mine[0].status, Tick::Open);
        assert_eq!(mine[0].item.photo, "", "the item is replaced");
        assert!(mine[0].reached_at(Tick::Done).is_some());
        assert_eq!(Ticks::list(&ctx).unwrap().len(), 2);
        // Marking the status it has writes no history line.
        let before = Ticks::for_stay(&ctx, other).unwrap()[0].history.len();
        mark::<Ticks>(Ticks::DECL, other, step(""), Tick::Done, Actor::Guest).unwrap();
        assert_eq!(
            Ticks::for_stay(&ctx, other).unwrap()[0].history.len(),
            before
        );
    });
}
