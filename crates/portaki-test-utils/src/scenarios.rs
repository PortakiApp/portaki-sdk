//! The seven pathological cases the sandbox replays after every deploy, for `cargo test`.
//!
//! The sandbox's « Scénarios » grid renders every surface of a module against seven stays that
//! break modules in production: no email, an arrival already past, one night, three months, no
//! guest name, no property photo, and the normal case. The fixtures are the SDK's
//! `contracts/scenarios/<case>.json`, carried in this crate byte for byte — the platform vendors
//! the same files, so a case that passes here is the case the sandbox runs.
//!
//! # Adopting it
//!
//! `tests/scenarios.rs`, run with `cargo test --test scenarios`:
//!
//! ```ignore
//! use portaki_test_utils::scenarios;
//!
//! #[test]
//! fn the_home_card_holds_on_every_case() {
//!     scenarios::check_each(|scenario| {
//!         let surface = scenario.guest().run(my_module::render_home_card);
//!         if scenario.stay.guest_name.is_none() {
//!             // what this module promises without a name
//!         }
//!         surface.map(|_| ()).map_err(|error| error.to_string())
//!     });
//! }
//! ```
//!
//! # What the fixtures do not carry
//!
//! - **The guest's contact.** As in production, the stay reaches a module without email or
//!   phone unless its manifest declares `stay:guest_contact:read`; a module that does chains
//!   `.with_guest_contact(scenario.stay.guest_email.as_deref(), None)`.
//! - **Today.** The sandbox counts the offsets from the day it runs; here they count from
//!   [`TODAY`], so a test never depends on the day it runs.

use std::panic::{catch_unwind, AssertUnwindSafe};

use chrono::{DateTime, Duration, NaiveDate, NaiveTime, Utc};
use portaki_sdk::context::StayContext;
use serde::Deserialize;
use uuid::Uuid;

use crate::{MockContextBuilder, Property};

/// The case ids, in the order of the sandbox grid.
pub const CASES: [&str; 7] = [
    "normal",
    "no_email",
    "past_arrival",
    "one_night",
    "three_months",
    "no_name",
    "no_photo",
];

/// The fixtures, as the SDK publishes them under `contracts/scenarios/`.
const FIXTURES: [&str; 7] = [
    include_str!("../scenarios/normal.json"),
    include_str!("../scenarios/no_email.json"),
    include_str!("../scenarios/past_arrival.json"),
    include_str!("../scenarios/one_night.json"),
    include_str!("../scenarios/three_months.json"),
    include_str!("../scenarios/no_name.json"),
    include_str!("../scenarios/no_photo.json"),
];

/// The day the offsets count from — [`Scenario::guest`] freezes the clock on it, at 10:00.
pub const TODAY: &str = "2026-06-15";

/// Arrival and departure hours, local time — the sandbox's.
const CHECK_IN_HOUR: u32 = 16;
const CHECK_OUT_HOUR: u32 = 10;

/// Europe/Paris in summer.
// ponytail: fixed UTC+2 — every case falls between TODAY-10 and TODAY+92, all inside summer
// time. A case outside it needs a real timezone database (chrono-tz).
const PARIS_SUMMER_OFFSET_HOURS: i64 = 2;

/// One pathological case.
#[derive(Debug, Clone, Deserialize)]
pub struct Scenario {
    /// `normal`, `no_email`… — one of [`CASES`].
    pub case: String,
    /// What the case exists to make a module meet.
    pub pathology: String,
    /// The property the stay is on.
    pub property: ScenarioProperty,
    /// The stay.
    pub stay: ScenarioStay,
}

/// The property of a case.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScenarioProperty {
    /// Display name.
    pub name: String,
    /// City — what a template composes an address from.
    pub city: String,
    /// Whether the property has a cover photo.
    pub has_photo: bool,
}

/// The stay of a case.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScenarioStay {
    /// Guest name, `None` for an import that did not carry one.
    pub guest_name: Option<String>,
    /// Guest email, `None` when the guest left none.
    pub guest_email: Option<String>,
    /// Check-in day, in days from today.
    pub check_in_offset: i64,
    /// Check-out day, in days from today.
    pub check_out_offset: i64,
    /// Booking channel as imported — may be empty.
    pub source: String,
}

/// The seven cases, in the order of [`CASES`].
pub fn all() -> Vec<Scenario> {
    FIXTURES
        .iter()
        .map(|raw| serde_json::from_str(raw).expect("a scenario fixture is valid JSON"))
        .collect()
}

/// The case named `case`.
///
/// # Panics
///
/// When `case` is not one of [`CASES`].
pub fn get(case: &str) -> Scenario {
    all()
        .into_iter()
        .find(|scenario| scenario.case == case)
        .unwrap_or_else(|| panic!("unknown scenario {case} — one of {}", CASES.join(", ")))
}

/// Runs `test` on every case, and fails once with every case that failed or panicked.
///
/// One case failing does not hide the next: the report reads like the sandbox grid.
pub fn check_each(mut test: impl FnMut(&Scenario) -> Result<(), String>) {
    let failures: Vec<String> = all()
        .iter()
        .filter_map(
            |scenario| match catch_unwind(AssertUnwindSafe(|| test(scenario))) {
                Ok(Ok(())) => None,
                Ok(Err(message)) => Some(format!("{}: {message}", scenario.case)),
                Err(panic) => Some(format!(
                    "{}: panicked — {}",
                    scenario.case,
                    panic_text(&panic)
                )),
            },
        )
        .collect();
    assert!(
        failures.is_empty(),
        "{} scenario(s) failed:\n  {}",
        failures.len(),
        failures.join("\n  ")
    );
}

fn panic_text(panic: &Box<dyn std::any::Any + Send>) -> String {
    panic
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| panic.downcast_ref::<&str>().map(|text| text.to_string()))
        .unwrap_or_default()
}

impl Scenario {
    /// A guest context on this case: property, stay, guest name, clock frozen on [`TODAY`].
    pub fn guest(&self) -> MockContextBuilder {
        let mut builder = self.apply(MockContextBuilder::guest());
        if let Some(guest) = builder.context.guest.as_mut() {
            guest.display_name = self.stay.guest_name.clone();
        }
        builder
    }

    /// A host context on this case — the host looking at the same stay.
    pub fn host(&self) -> MockContextBuilder {
        self.apply(MockContextBuilder::host())
    }

    /// Check-in instant.
    pub fn check_in(&self) -> DateTime<Utc> {
        local(self.stay.check_in_offset, CHECK_IN_HOUR)
    }

    /// Check-out instant.
    pub fn check_out(&self) -> DateTime<Utc> {
        local(self.stay.check_out_offset, CHECK_OUT_HOUR)
    }

    /// Stable per case: a module keying anything on the stay finds it again next run.
    fn stay_id(&self) -> Uuid {
        let mut bytes = [0u8; 16];
        for (slot, byte) in bytes.iter_mut().zip(self.case.bytes()) {
            *slot = byte;
        }
        Uuid::from_bytes(bytes)
    }

    fn apply(&self, builder: MockContextBuilder) -> MockContextBuilder {
        let mut builder = builder
            .with_property(Property {
                name: self.property.name.clone(),
                ..Property::default()
            })
            .with_stay(StayContext {
                stay_id: self.stay_id(),
                checkin_at: Some(self.check_in()),
                checkout_at: Some(self.check_out()),
                booking_channel: Some(self.stay.source.clone()).filter(|s| !s.is_empty()),
                ..StayContext::default()
            })
            .with_now(local(0, 10));
        builder.context.property.address =
            Some(self.property.city.clone()).filter(|city| !city.is_empty());
        builder
    }
}

/// `offset` days from [`TODAY`], at `hour` Paris time.
fn local(offset: i64, hour: u32) -> DateTime<Utc> {
    let day = NaiveDate::parse_from_str(TODAY, "%Y-%m-%d").expect("TODAY is a date")
        + Duration::days(offset);
    let at = day.and_time(NaiveTime::from_hms_opt(hour, 0, 0).expect("an hour"));
    (at - Duration::hours(PARIS_SUMMER_OFFSET_HOURS)).and_utc()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_seven_cases_load_in_grid_order() {
        let ids: Vec<String> = all().into_iter().map(|scenario| scenario.case).collect();
        assert_eq!(ids, CASES);
    }

    #[test]
    fn each_case_carries_its_pathology() {
        assert!(get("no_email").stay.guest_email.is_none());
        assert!(get("no_name").stay.guest_name.is_none());
        assert!(!get("no_photo").property.has_photo);
        let past = get("past_arrival");
        assert!(past.check_in() < local(0, 10));
        let one = get("one_night");
        assert_eq!((one.check_out() - one.check_in()).num_hours(), 18);
        let long = get("three_months");
        assert!((long.check_out() - long.check_in()).num_days() >= 89);
    }

    /// 16:00 in Paris in June is 14:00 UTC.
    #[test]
    fn check_in_is_four_pm_paris_time() {
        assert_eq!(
            get("normal").check_in().to_rfc3339(),
            "2026-06-22T14:00:00+00:00"
        );
    }

    #[test]
    fn the_guest_context_carries_the_case() {
        let context = get("no_name").guest().context();
        assert!(context.guest.expect("a guest").display_name.is_none());
        assert_eq!(context.property.name, "Le Petit Balcon");
        assert!(context.stay.expect("a stay").guest_email.is_none());

        let normal = get("normal").guest().context();
        assert_eq!(
            normal.guest.expect("a guest").display_name.as_deref(),
            Some("Camille Roux")
        );
    }

    #[test]
    fn every_failing_case_is_named_once() {
        let outcome = catch_unwind(|| {
            check_each(|scenario| match scenario.case.as_str() {
                "no_email" => Err("no fallback text".to_string()),
                "no_photo" => panic!("image unwrap"),
                _ => Ok(()),
            })
        });
        let panic = outcome.expect_err("two cases failed");
        let text = panic_text(&panic);
        assert!(text.contains("2 scenario(s) failed"), "{text}");
        assert!(text.contains("no_email: no fallback text"), "{text}");
        assert!(text.contains("no_photo: panicked — image unwrap"), "{text}");
    }
}
