//! A module written for the happy path: the SDK renders its guest states.
//!
//! No `guest/empty.rs`: `#[surface(guest, …)]` shows « inactive » or « incomplete » without
//! calling the function, and turns an `Err` into a logged error state — with the icon
//! `portaki_module!` declares and texts the module may override from its bundle.

use portaki_sdk::host::module::ModuleStatus;
use portaki_sdk::prelude::*;
use portaki_sdk::sdui::primitives::{Card, EmptyState, Text};
use portaki_sdk::wasm::registry::{declarations, HandlerDeclaration};
use portaki_test_utils::{LogLine, MockContext, MockContextBuilder, SurfaceAssertions};

portaki_sdk::portaki_module!(id = "wifi-guest", icon = IconName::Wifi);

#[portaki_sdk::config]
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    #[field(required, label = "config.ssid")]
    pub ssid: String,
}

#[portaki_sdk::surface(guest, id = "home.card")]
pub fn render_home_card(ctx: GuestContext) -> Result<Surface> {
    let config = Config::load(&ctx)?;
    Ok(Surface::new(Card::new().title(config.ssid)))
}

#[portaki_sdk::surface(guest, id = "explore.detail")]
pub fn render_explore_detail(_ctx: GuestContext) -> Surface {
    Surface::new(Text::new().text("i18n:guest.detail"))
}

/// The stay's dates, which do not depend on the Wi-Fi setup: shown whatever the status.
#[portaki_sdk::surface(guest, id = "explore.stay", gate = false)]
pub fn render_explore_stay(ctx: GuestContext) -> Result<Surface> {
    let stay = ctx.stay.ok_or(PortakiError::Host("no stay".into()))?;
    Ok(Surface::new(Text::new().text(stay.stay_id.to_string())))
}

/// A module as written before the shell: its own readiness check, its own `i18n:` keys.
#[portaki_sdk::surface(guest, id = "legacy.card")]
pub fn render_legacy_card(_ctx: GuestContext) -> Surface {
    match host::module::status() {
        Ok(status) if !status.is_ready() => Surface::new(
            EmptyState::new()
                .title("i18n:module.status.inactive.title")
                .icon(IconName::Wifi),
        ),
        _ => Surface::new(Text::new().text("i18n:guest.detail")),
    }
}

fn surface(name: &str) -> &'static HandlerDeclaration {
    declarations()
        .find(|declaration| declaration.name == name)
        .expect("declared")
}

/// Renders `name` the way the runtime does — through the shim — and keeps what was logged.
fn render(name: &str, mock: MockContextBuilder) -> (Result<Surface>, Vec<LogLine>) {
    mock.run_with(|ctx, host| {
        let surface = (surface(name).dispatch)(ctx, serde_json::json!({}))
            .map(|tree| serde_json::from_value(tree).expect("a surface"));
        (surface, host.logs())
    })
}

fn state(surface: &Surface) -> EmptyState {
    SurfaceAssertions::new(surface)
        .find::<EmptyState>()
        .expect("a state")
}

fn status(active: bool, incomplete: bool) -> ModuleStatus {
    ModuleStatus {
        active,
        workspace_enabled: true,
        incomplete,
        requires_config: true,
        missing_required_keys: Vec::new(),
    }
}

fn configured() -> MockContextBuilder {
    MockContext::guest().with_config(&Config {
        ssid: "Villa-Azur".into(),
    })
}

#[test]
fn a_ready_module_renders_its_surface() {
    let (surface, logs) = render("home.card", configured());

    let json = serde_json::to_string(&surface.unwrap()).unwrap();
    assert!(json.contains("Villa-Azur"), "{json}");
    assert!(logs.is_empty());
}

#[test]
fn an_inactive_module_shows_the_sdk_state_with_the_module_icon() {
    let (surface, _) = render(
        "home.card",
        configured().with_module_status(status(false, false)),
    );

    let surface = surface.unwrap();
    let state = state(&surface);
    assert_eq!(state.title.as_deref(), Some("Indisponible"));
    assert_eq!(state.icon, Some(IconName::Wifi));
    assert_eq!(surface.id.as_deref(), Some("home.card"));
    assert!(!serde_json::to_string(&surface)
        .unwrap()
        .contains("Villa-Azur"));
}

/// An infallible surface is gated the same way: its function is not called.
#[test]
fn an_incomplete_module_asks_the_guest_to_wait() {
    let (surface, _) = render(
        "explore.detail",
        MockContext::guest().with_module_status(status(true, true)),
    );

    let surface = surface.unwrap();
    let state = state(&surface);
    assert_eq!(state.title.as_deref(), Some("Bientôt disponible"));
    assert_eq!(state.icon, Some(IconName::Sliders));
    assert!(
        !SurfaceAssertions::new(&surface).contains_primitive::<Text>(),
        "no hint unless the module writes one"
    );
}

#[test]
fn an_error_is_logged_under_a_derived_name_and_shown_as_the_error_state() {
    let unreadable = MockContext::guest().with_config(&serde_json::json!({ "ssid": 42 }));
    let (surface, logs) = render("home.card", unreadable);

    let surface = surface.unwrap();
    assert_eq!(
        state(&surface).title.as_deref(),
        Some("Momentanément indisponible")
    );
    let [line] = logs.as_slice() else {
        panic!("one log line: {logs:?}")
    };
    assert_eq!(line.level, "error");
    assert_eq!(line.message, "test_module_home_card_render_failed");
    assert_eq!(line.fields["surfaceId"], "home.card");
    assert!(line.fields["error"]
        .as_str()
        .unwrap()
        .contains("config_unreadable"));
}

#[test]
fn a_platform_that_cannot_answer_is_an_error_too() {
    let (surface, logs) = render(
        "explore.detail",
        MockContext::guest().with_module_status_error("module_status_unavailable"),
    );

    assert_eq!(
        state(&surface.unwrap()).title.as_deref(),
        Some("Momentanément indisponible")
    );
    assert_eq!(logs[0].message, "test_module_explore_detail_render_failed");
}

#[test]
fn a_key_of_the_module_bundle_wins_and_the_guest_language_is_followed() {
    let english = MockContext::guest()
        .with_property(portaki_test_utils::Property {
            locale: "en-GB".into(),
            ..Default::default()
        })
        .with_module_status(status(false, false))
        .with_translation("module.status.inactive.title", "Wi-Fi switched off");
    let (surface, _) = render("home.card", english);

    let state = state(&surface.unwrap());
    assert_eq!(state.title.as_deref(), Some("Wi-Fi switched off"));
    assert_eq!(
        state.description.as_deref(),
        Some("This section is not offered at this property.")
    );
}

#[test]
fn an_ungated_surface_is_called_whatever_the_status_and_its_error_goes_up() {
    let inactive = MockContext::guest().with_module_status(status(false, false));
    let (surface, _) = render(
        "explore.stay",
        inactive
            .clone()
            .with_stay(portaki_test_utils::Booking::default()),
    );
    assert!(SurfaceAssertions::new(&surface.unwrap()).contains_primitive::<Text>());

    let (surface, logs) = render("explore.stay", inactive);
    assert!(surface.unwrap_err().to_string().contains("no stay"));
    assert!(logs.is_empty());
}

/// Called directly, as in a unit test, the function is the one written: no shell.
#[test]
fn the_function_itself_is_left_as_written() {
    MockContext::guest()
        .with_module_status(status(false, false))
        .run(|ctx| assert!(render_home_card(ctx).is_ok()));
}

#[test]
fn a_property_not_geocoded_has_no_coordinates() {
    MockContext::guest().with_coordinates(None).run(|ctx| {
        assert_eq!(ctx.property.coordinates, None);
    });
    MockContext::guest().run(|ctx| {
        assert_eq!(
            ctx.property.coordinates,
            Some(GeoPoint::new(43.5513, 7.0128))
        );
    });
}

/// Its keys are the SDK's: the words its bundle declares are the ones the guest reads.
#[test]
fn a_module_keeping_its_empty_rs_shows_its_own_words() {
    let (surface, _) = render(
        "legacy.card",
        MockContext::guest()
            .with_module_status(status(false, false))
            .with_translation("module.status.inactive.title", "Module Wi-Fi désactivé"),
    );

    let state = state(&surface.unwrap());
    assert_eq!(state.title.as_deref(), Some("Module Wi-Fi désactivé"));
    assert_eq!(state.icon, Some(IconName::Wifi));

    let (surface, _) = render("legacy.card", MockContext::guest());
    assert!(serde_json::to_string(&surface.unwrap())
        .unwrap()
        .contains("i18n:guest.detail"));
}
