//! Rows inside rows, links, files and labels, and a module's own rule — against the stateful
//! mock repository.

use portaki_sdk::contracts::i18n::I18nText;
use portaki_sdk::resource::{Collection, CollectionEditor, Problem};
use portaki_test_utils::MockContext;
use serde_json::json;

#[portaki_sdk::item]
pub struct Step {
    #[field(max_chars = 20, label = "host.step.text")]
    pub text: I18nText,
    #[field(kind = "photo", label = "host.step.image")]
    pub image: String,
}

#[portaki_sdk::item(key = "name")]
pub struct Device {
    #[field(required, label = "host.device.name")]
    pub name: I18nText,
    #[field(label = "host.device.featured")]
    pub featured: bool,
    #[field(kind = "url", label = "host.device.manualUrl")]
    pub manual_url: String,
    #[field(
        kind = "file",
        accept = "application/pdf",
        max_bytes = 1024,
        label = "host.device.manualFile"
    )]
    pub manual_file: String,
    #[field(max_rows = 2, label = "host.device.steps")]
    pub steps: Vec<Step>,
}

#[portaki_sdk::choice(label = "host.notify")]
pub enum Channel {
    #[default]
    Email,
    App,
}

#[portaki_sdk::item(meta)]
pub struct Display {
    #[field(label = "host.roomOrder.label")]
    pub room_order: Vec<String>,
    #[field(default = true, label = "host.requests.enabled")]
    pub requests_enabled: Option<bool>,
    #[field(
        kind = "choices",
        visible_when = "requests_enabled=true",
        label = "host.notify"
    )]
    pub notify: Option<Vec<Channel>>,
}

/// One device on the home card at most.
fn one_featured(devices: &Collection<Device, Display>) -> Vec<Problem> {
    let featured = devices.rows().iter().filter(|d| d.featured).count();
    (featured > 1)
        .then(|| Problem {
            row: None,
            field: "featured",
            message: I18nText::new("Un seul.", "Only one."),
        })
        .into_iter()
        .collect()
}

#[portaki_sdk::collection(name = "devices", item = Device, meta = Display, check = one_featured, import)]
pub struct Devices;

#[test]
fn nested_rows_round_trip_with_their_ids() {
    MockContext::host().run(|ctx| {
        __portaki_devices_update_config(
            ctx.clone(),
            json!({ "devices": [ { "name": "Four", "steps": [ { "text": "Tourner" }, { "text": "Attendre", "image": "portaki-file:1" } ] } ],
                    "room_order": [ { "label": "Cuisine" }, { "label": "Salon" } ] }),
        )
        .unwrap();
        let devices = Devices::load(&ctx).unwrap();
        let steps = &devices.rows()[0].steps;
        assert_eq!(steps.len(), 2);
        assert_eq!(steps[1].image, "portaki-file:1");
        assert_eq!(devices.meta().room_order, ["Cuisine", "Salon"]);

        // The first step removed, by id: the second keeps its id and its photo.
        let device = &devices.rows()[0];
        __portaki_devices_update_config(
            ctx.clone(),
            json!({ "devices": [ { "id": device.id, "steps": [ { "id": steps[1].id, "text": "Patienter" } ] } ] }),
        )
        .unwrap();
        let after = Devices::load(&ctx).unwrap();
        assert_eq!(after.rows()[0].steps.len(), 1);
        assert_eq!(after.rows()[0].steps[0].id, steps[1].id);
        assert_eq!(after.rows()[0].steps[0].image, "portaki-file:1");
    });
}

#[test]
fn the_modules_rule_and_the_fields_block_publication() {
    MockContext::host().run(|ctx| {
        __portaki_devices_update_config(
            ctx.clone(),
            json!({ "devices": [
                { "name": "Four", "featured": true, "manual_url": "http://notice", "steps": ["a", "b", "c"] },
                { "name": "Télé", "featured": true }
            ] }),
        )
        .unwrap();
        let readiness = __portaki_devices_publish_readiness(ctx.clone()).unwrap();
        let ids: Vec<&str> = readiness.items.iter().map(|i| i.id.as_str()).collect();
        assert!(ids.contains(&"resources.devices.featured"), "{ids:?}");
        assert!(ids.iter().any(|id| id.ends_with(".manual_url")), "{ids:?}");
        assert!(ids.iter().any(|id| id.ends_with(".steps")), "{ids:?}");
    });
}

#[test]
fn the_editor_draws_nested_rows_with_their_ids_and_the_other_controls() {
    let rows = json!([ { "id": "d1", "name": { "fr": "Four" }, "featured": true,
                         "steps": [ { "id": "s1", "text": { "fr": "Tourner" } } ] } ]);
    MockContext::host()
        .with_resource("devices", rows, json!({ "room_order": ["Cuisine"] }))
        .run(|ctx| {
            let devices = Devices::load(&ctx).unwrap();
            let editor = CollectionEditor::new(&ctx, &devices)
                .column("name")
                .computed("last", "host.column.last")
                .cell("last", |_: &Device| "hier".into())
                .check_at_most("featured", 1, "Un seul.")
                .search("host.search")
                .filter_by("name")
                .settings("host.display.title", Vec::new());
            let tree = serde_json::to_string(&editor.build()).unwrap();
            for wanted in [
                r#""type":"StepList""#,
                r#""name":"devices.0.steps.0.id""#,
                r#""value":"s1""#,
                r#""name":"devices.0.steps.0.text""#,
                r#""type":"FileUpload""#,
                r#""accept":"application/pdf""#,
                r#""type":"EditableList""#,
                r#""kind":"check""#,
                r#""label":"i18n:host.column.last""#,
                r#""last":"hier""#,
                r#""featured":true"#,
                r#""searchPlaceholder":"i18n:host.search""#,
            ] {
                assert!(tree.contains(wanted), "{wanted} in {tree}");
            }
        });
}

#[test]
fn several_choices_keep_only_known_values_once() {
    MockContext::host().run(|ctx| {
        __portaki_devices_update_config(ctx.clone(), json!({ "notify": "app,email,app,sms" }))
            .unwrap();
        let devices = Devices::load(&ctx).unwrap();
        assert_eq!(
            devices.meta().notify,
            Some(vec![Channel::App, Channel::Email])
        );
        assert_eq!(
            devices.meta().requests_enabled,
            None,
            "never sent, never stored"
        );
        __portaki_devices_update_config(ctx.clone(), json!({ "notify": r#"["email"]"# })).unwrap();
        assert_eq!(
            Devices::load(&ctx).unwrap().meta().notify,
            Some(vec![Channel::Email])
        );
    });
}

#[test]
fn a_default_shows_until_the_host_chooses_and_a_condition_travels() {
    MockContext::host().run(|ctx| {
        let devices = Devices::load(&ctx).unwrap();
        let tree = serde_json::to_string(
            &CollectionEditor::new(&ctx, &devices)
                .settings("host.display.title", Vec::new())
                .settings_last()
                .build(),
        )
        .unwrap();
        assert!(
            tree.contains(r#""checked":true"#),
            "requests open by default: {tree}"
        );
        assert!(
            tree.contains(r#""visibleWhen":"requests_enabled=true""#),
            "{tree}"
        );
        assert!(tree.contains(r#""multi":true"#), "{tree}");
        assert!(
            tree.find("DataTable").unwrap() < tree.find("host.display.title").unwrap(),
            "rows first"
        );
    });
}
