//! Declared records against the stateful mock repository: what `#[record]` generates keeps a
//! status and a history, shows a guest only his stay, leaves the host's operations to the host,
//! turns late records into timeline tasks, and goes with its stay.

use chrono::Duration;
use portaki_sdk::contracts::timeline::TimelineTasksArgs;
use portaki_sdk::resource::{HistoryEvent, RecordHooks};
use portaki_test_utils::MockContext;
use serde_json::json;
use std::sync::atomic::{AtomicUsize, Ordering};

#[portaki_sdk::choice(label = "status")]
pub enum Status {
    #[initial]
    New,
    InProgress,
    #[done]
    Resolved,
}

#[portaki_sdk::choice(label = "category")]
pub enum Category {
    Appliance,
    Noise,
}

#[portaki_sdk::item(key = "summary")]
pub struct Report {
    #[field(required, label = "form.category.label")]
    pub category: Category,
    #[field(required, max_chars = 20, label = "form.summary.label")]
    pub summary: String,
    #[field(kind = "photo", label = "form.photo.label")]
    pub photo: String,
}

static CREATED: AtomicUsize = AtomicUsize::new(0);
static NOTED: AtomicUsize = AtomicUsize::new(0);

#[portaki_sdk::record(name = "reports", item = Report, status = Status, hooks)]
pub struct Reports;

impl RecordHooks for Reports {
    type Item = Report;
    type Status = Status;

    fn guard(_ctx: &portaki_sdk::Context, item: &Report) -> portaki_sdk::Result<()> {
        if item.category == Category::Noise {
            return Err(portaki_sdk::PortakiError::Host(
                "category_not_offered".into(),
            ));
        }
        Ok(())
    }

    fn on_created(
        _: &portaki_sdk::Context,
        _: &portaki_sdk::resource::Entry<Report, Status>,
    ) -> portaki_sdk::Result<()> {
        CREATED.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    fn on_note(
        _: &portaki_sdk::Context,
        _: &portaki_sdk::resource::Entry<Report, Status>,
        _: &str,
    ) -> portaki_sdk::Result<()> {
        NOTED.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

#[test]
fn a_guest_deposits_and_the_host_follows_with_a_history() {
    let (ctx, host) = MockContext::guest().build();
    portaki_sdk::host::with_host(host.clone(), ctx.clone(), || {
        __portaki_reports_submit(
            ctx.clone(),
            json!({ "category": "appliance", "summary": "Four en panne" }),
        )
        .unwrap();
        assert!(CREATED.load(Ordering::SeqCst) >= 1);
        assert!(
            __portaki_reports_submit(
                ctx.clone(),
                json!({ "category": "noise", "summary": "Bruit" })
            )
            .unwrap_err()
            .to_string()
            .contains("category_not_offered"),
            "the module's guard refuses"
        );
        assert!(
            __portaki_reports_submit(ctx.clone(), json!({ "category": "appliance" }))
                .unwrap_err()
                .to_string()
                .contains("summary_required")
        );

        let entry = Reports::list(&ctx).unwrap().remove(0);
        assert_eq!(entry.status, Status::New);
        assert_eq!(entry.stay_id, ctx.guest.as_ref().map(|g| g.session_id));
        assert!(
            __portaki_reports_set_status(
                ctx.clone(),
                serde_json::from_value(json!({ "id": entry.id, "status": "resolved" })).unwrap()
            )
            .unwrap_err()
            .to_string()
            .contains("host_only"),
            "a guest does not move a status"
        );
    });
    let mut host_ctx = ctx.clone();
    host_ctx.guest = None;
    portaki_sdk::host::with_host(host, host_ctx.clone(), || {
        let id = Reports::list(&host_ctx).unwrap()[0].id;
        __portaki_reports_set_status(
            host_ctx.clone(),
            serde_json::from_value(json!({ "id": id, "status": "in_progress" })).unwrap(),
        )
        .unwrap();
        __portaki_reports_note(
            host_ctx.clone(),
            serde_json::from_value(json!({ "id": id, "text": "Le technicien passe demain." }))
                .unwrap(),
        )
        .unwrap();
        let noted = NOTED.load(Ordering::SeqCst);
        __portaki_reports_note(
            host_ctx.clone(),
            serde_json::from_value(json!({ "id": id, "text": " Le technicien passe demain. " }))
                .unwrap(),
        )
        .unwrap();
        assert_eq!(
            NOTED.load(Ordering::SeqCst),
            noted,
            "the same reply is not sent twice"
        );
        __portaki_reports_set_status(
            host_ctx.clone(),
            serde_json::from_value(json!({ "id": id, "status": "resolved" })).unwrap(),
        )
        .unwrap();
        let entry = Reports::get(&host_ctx, id).unwrap().unwrap();
        assert!(entry.is_done());
        let kinds: Vec<&HistoryEvent> = entry.history.iter().map(|l| &l.event).collect();
        assert_eq!(kinds.len(), 4);
        assert!(matches!(kinds[0], HistoryEvent::Created));
        assert!(matches!(kinds[2], HistoryEvent::Note { .. }));
        assert_eq!(
            entry.notes().collect::<Vec<_>>(),
            vec!["Le technicien passe demain."]
        );
        assert!(NOTED.load(Ordering::SeqCst) >= 1);
    });
}

#[test]
fn a_guest_sees_only_his_stay_and_records_go_with_their_stay() {
    let (ctx, host) = MockContext::guest().build();
    let mine = ctx.guest.as_ref().unwrap().session_id;
    let other = uuid::Uuid::new_v4();
    portaki_sdk::host::with_host(host.clone(), ctx.clone(), || {
        __portaki_reports_submit(
            ctx.clone(),
            json!({ "category": "appliance", "summary": "Mien" }),
        )
        .unwrap();
        let mut host_ctx = ctx.clone();
        host_ctx.guest = None;
        __portaki_reports_add(
            host_ctx,
            json!({ "stayId": other, "category": "appliance", "summary": "Autre" }),
        )
        .unwrap();
        let seen = __portaki_reports_list_for_stay(ctx.clone()).unwrap();
        assert_eq!(seen.len(), 1);
        assert_eq!(seen[0]["item"]["summary"], "Mien");
        assert_eq!(Reports::for_stay(&ctx, other).unwrap().len(), 1);
    });
    // Purging a stay is a delete on its `stay_id` column: the record goes with it.
    let rows = host.rows("PortakiResource");
    assert!(rows.iter().any(|r| r["stay_id"] == json!(mine)));
    assert!(
        rows.iter().all(|r| r["stay_id"].is_string()),
        "every record carries its stay as a column"
    );
}

#[test]
fn a_late_open_record_is_a_task_whose_check_closes_it() {
    let (ctx, host) = MockContext::host().build();
    portaki_sdk::host::with_host(host, ctx.clone(), || {
        let stay = uuid::Uuid::new_v4();
        __portaki_reports_add(
            ctx.clone(),
            json!({ "stayId": stay, "category": "appliance", "summary": "Fuite" }),
        )
        .unwrap();
        let mut entry = Reports::list(&ctx).unwrap().remove(0);
        entry.created_at -= Duration::hours(30);
        entry.save(Reports::DECL).unwrap();
        let now = portaki_sdk::host::time::now().unwrap();
        let args = TimelineTasksArgs {
            property_id: ctx.property_id,
            from: now - Duration::days(7),
            to: now + Duration::days(7),
            stays: Vec::new(),
        };
        // `due_hours = 24` without bundles in this test crate: the runtime the macro would call.
        let decl = portaki_sdk::resource::RecordDecl {
            due_hours: Some(24),
            ..Reports::DECL
        };
        let tasks_of = |args: &TimelineTasksArgs| {
            portaki_sdk::resource::record::timeline_tasks::<Reports>(
                &ctx,
                decl,
                args,
                "t".into(),
                "d".into(),
            )
            .unwrap()
            .tasks
        };
        let tasks = tasks_of(&args);
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].stay_id, Some(stay));
        portaki_sdk::resource::record::complete_task::<Reports>(decl, &tasks[0].id, true).unwrap();
        assert!(Reports::list(&ctx).unwrap()[0].is_done());
        assert!(tasks_of(&args).is_empty());
        // Unchecked: the host ticked the wrong line, the record is open again.
        portaki_sdk::resource::record::complete_task::<Reports>(decl, &tasks[0].id, false).unwrap();
        let entry = Reports::list(&ctx).unwrap().remove(0);
        assert_eq!(entry.status, Status::New);
        assert!(entry.reached_at(Status::Resolved).is_some());
        assert_eq!(tasks_of(&args).len(), 1);
    });
}
