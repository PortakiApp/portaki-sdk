//! When a secret (door code, Wi-Fi password, parking code) may be shown to a guest.
//!
//! The host picks a [`RevealPolicy`] in the module's settings; a guest surface asks
//! [`RevealPolicy::evaluate_for`] and shows the secret, or [`SECRET_MASK`] and a
//! [`RevealDecision::locked_message`]. Everything runs in the module: the plaintext never enters
//! the SDUI tree while it is locked.
//!
//! Fail-safe: [`RevealPolicy::Always`] reveals without a stay; any other policy stays locked
//! until the stay has a check-in and its moment has come.
//!
//! ```
//! use chrono::{TimeZone, Utc};
//! use portaki_sdk::context::{Context, StayContext};
//! use portaki_sdk::reveal::{RevealPolicy, SECRET_MASK};
//!
//! let checkin = Utc.with_ymd_and_hms(2026, 7, 20, 14, 0, 0).unwrap(); // 16:00 in Paris
//! let ctx = Context {
//!     stay: Some(StayContext { checkin_at: Some(checkin), ..StayContext::default() }),
//!     ..Context::default() // Europe/Paris
//! };
//!
//! let early = Utc.with_ymd_and_hms(2026, 7, 19, 13, 0, 0).unwrap();
//! let decision = RevealPolicy::DayBefore16h.evaluate_for(&ctx, early);
//! assert_eq!(decision.show("4821"), SECRET_MASK);
//! assert_eq!(
//!     decision.locked_message(&ctx).as_deref(),
//!     Some("Disponible à partir du 19 juil. 2026 · 16:00")
//! );
//!
//! let later = Utc.with_ymd_and_hms(2026, 7, 19, 14, 0, 0).unwrap();
//! assert_eq!(RevealPolicy::DayBefore16h.evaluate_for(&ctx, later).show("4821"), "4821");
//! ```
//!
//! In the settings, a `select` field holds the policy — its options are
//! [`RevealPolicy::WIRE_VALUES`] — and [`RevealPolicy::choice_list`] draws the choice with the
//! SDK's labels:
//!
//! ```text
//! #[field(kind = "select", options = ["always", "hours_before_24", "day_before_16h", "at_checkin"],
//!         label = "config.revealPolicy")]
//! pub reveal_policy: RevealPolicy,
//! ```

use chrono::{DateTime, Days, Duration, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::context::Context;
use crate::host::time::{self, PropertyTz};
use crate::sdui::common::{ChoiceListLayout, ChoiceOption};
use crate::sdui::primitives::ChoiceList;
use crate::vocab::IconName;

/// What a locked secret shows instead of itself.
pub const SECRET_MASK: &str = "••••••";

/// When a secret becomes visible to the guest.
///
/// Wire values: `always`, `hours_before_24`, `day_before_16h` (the default), `at_checkin`; the
/// older `hours_before24` / `day_before16h` still read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RevealPolicy {
    /// As soon as the guest opens the booklet, stay or not.
    Always,
    /// 24 hours before the check-in instant.
    #[serde(rename = "hours_before_24", alias = "hours_before24")]
    HoursBefore24,
    /// From 16:00, property time, the day before check-in.
    #[default]
    #[serde(rename = "day_before_16h", alias = "day_before16h")]
    DayBefore16h,
    /// At the check-in instant.
    AtCheckin,
}

/// Whether the secret shows now, and from when it will.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RevealDecision {
    /// The secret may be shown.
    pub revealed: bool,
    /// When it may (UTC); `None` for [`RevealPolicy::Always`] or without a check-in.
    pub available_from: Option<DateTime<Utc>>,
    /// The property's timezone, to write `available_from` in.
    tz: Option<PropertyTz>,
}

impl RevealPolicy {
    /// Every policy, in the order the settings offer them.
    pub const ALL: [RevealPolicy; 4] = [
        Self::Always,
        Self::HoursBefore24,
        Self::DayBefore16h,
        Self::AtCheckin,
    ];

    /// The wire value of each of [`Self::ALL`] — the `options` of the `select` field.
    pub const WIRE_VALUES: [&'static str; 4] =
        ["always", "hours_before_24", "day_before_16h", "at_checkin"];

    /// The wire value (`day_before_16h`).
    pub const fn as_wire(self) -> &'static str {
        match self {
            Self::Always => "always",
            Self::HoursBefore24 => "hours_before_24",
            Self::DayBefore16h => "day_before_16h",
            Self::AtCheckin => "at_checkin",
        }
    }

    /// When the secret becomes visible for a check-in at `checkin_at`; `None` for
    /// [`Self::Always`]. `tz` places « 16:00 the day before »; without one, that is 16:00 UTC.
    pub fn reveal_at(
        self,
        checkin_at: DateTime<Utc>,
        tz: Option<PropertyTz>,
    ) -> Option<DateTime<Utc>> {
        match self {
            Self::Always => None,
            Self::HoursBefore24 => Some(checkin_at - Duration::hours(24)),
            Self::AtCheckin => Some(checkin_at),
            Self::DayBefore16h => {
                let local_day = match tz {
                    Some(tz) => tz.to_local(checkin_at).date_naive(),
                    None => checkin_at.date_naive(),
                };
                let eve: NaiveDateTime = local_day
                    .checked_sub_days(Days::new(1))?
                    .and_hms_opt(16, 0, 0)?;
                Some(match tz {
                    Some(tz) => tz.from_local(eve),
                    None => eve.and_utc(),
                })
            }
        }
    }

    /// Whether the secret shows at `now` for a check-in at `checkin_at` (`None`: no stay, or a
    /// stay without one — locked, unless [`Self::Always`]).
    pub fn evaluate(
        self,
        now: DateTime<Utc>,
        checkin_at: Option<DateTime<Utc>>,
        tz: Option<PropertyTz>,
    ) -> RevealDecision {
        let available_from = checkin_at.and_then(|checkin| self.reveal_at(checkin, tz));
        RevealDecision {
            revealed: self == Self::Always || available_from.is_some_and(|from| now >= from),
            available_from,
            tz,
        }
    }

    /// [`Self::evaluate`] for this invocation: the stay's check-in, the property's timezone.
    pub fn evaluate_for(self, ctx: &Context, now: DateTime<Utc>) -> RevealDecision {
        let checkin_at = ctx.stay.as_ref().and_then(|stay| stay.checkin_at);
        self.evaluate(now, checkin_at, ctx.property_tz())
    }

    /// The policy's name in the settings, in `lang` (`fr`, `en`…; English otherwise).
    pub fn label(self, lang: &str) -> &'static str {
        TEXTS[self as usize].0[time::column(lang)]
    }

    /// What the policy does, one sentence, in `lang`.
    pub fn description(self, lang: &str) -> &'static str {
        TEXTS[self as usize].1[time::column(lang)]
    }

    /// The host's choice among [`Self::ALL`], `current` selected, labelled in `lang` — a
    /// `ChoiceList` posting `name` with the wire value.
    pub fn choice_list(name: &str, current: Self, lang: &str) -> ChoiceList {
        ChoiceList::new()
            .name(name)
            .value(current.as_wire())
            .layout(ChoiceListLayout::Compact)
            .choices(
                Self::ALL
                    .iter()
                    .map(|policy| {
                        ChoiceOption::new(policy.as_wire(), policy.label(lang))
                            .description(policy.description(lang))
                            .icon(IconName::ClockCircle)
                    })
                    .collect(),
            )
    }
}

impl RevealDecision {
    /// `secret` when revealed, [`SECRET_MASK`] otherwise.
    pub fn show<'a>(&self, secret: &'a str) -> &'a str {
        if self.revealed {
            secret
        } else {
            SECRET_MASK
        }
    }

    /// Why the secret is hidden, for the guest, in their language — « Disponible à partir du
    /// 19 juil. 2026 · 16:00 » in the property's time, or « bientôt » without a check-in. `None`
    /// once revealed.
    pub fn locked_message(&self, ctx: &Context) -> Option<String> {
        if self.revealed {
            return None;
        }
        let lang = ctx.lang();
        let column = time::column(&lang);
        Some(match self.available_from {
            Some(from) => {
                let when = match self.tz {
                    Some(tz) => time::date_time(tz.to_local(from), &lang),
                    None => time::date_time(from, &lang),
                };
                LOCKED_UNTIL[column].replace("{when}", &when)
            }
            None => LOCKED_SOON[column].to_string(),
        })
    }
}

/// Label and description of each policy, in the order of [`RevealPolicy::ALL`], one column per
/// language of [`time::LANGUAGES`].
const TEXTS: [([&str; 6], [&str; 6]); 4] = [
    (
        ["Always", "Toujours", "Siempre", "Immer", "Sempre", "Altijd"],
        [
            "Shown as soon as the guest opens the booklet.",
            "Visible dès que le voyageur ouvre le livret.",
            "Visible en cuanto el viajero abre la guía.",
            "Sichtbar, sobald der Gast die Gästemappe öffnet.",
            "Visibile appena l’ospite apre la guida.",
            "Zichtbaar zodra de gast het gastenboekje opent.",
        ],
    ),
    (
        [
            "24 hours before check-in",
            "24 h avant l’arrivée",
            "24 h antes de la llegada",
            "24 Std. vor der Anreise",
            "24 ore prima dell’arrivo",
            "24 uur vóór aankomst",
        ],
        [
            "Shown 24 hours before the check-in time.",
            "Visible 24 h avant l’heure d’arrivée.",
            "Visible 24 h antes de la hora de llegada.",
            "Sichtbar 24 Stunden vor der Anreisezeit.",
            "Visibile 24 ore prima dell’orario di arrivo.",
            "Zichtbaar 24 uur vóór het aankomsttijdstip.",
        ],
    ),
    (
        [
            "The day before, from 4 pm",
            "La veille à partir de 16 h",
            "El día antes, desde las 16:00",
            "Am Vortag ab 16 Uhr",
            "Il giorno prima, dalle 16",
            "De dag ervoor vanaf 16.00 uur",
        ],
        [
            "Shown from 4 pm the day before arrival, property time.",
            "Visible dès 16 h la veille de l’arrivée, heure du logement.",
            "Visible desde las 16:00 del día anterior a la llegada, hora del alojamiento.",
            "Sichtbar ab 16 Uhr am Vortag der Anreise, Ortszeit der Unterkunft.",
            "Visibile dalle 16 del giorno prima dell’arrivo, ora dell’alloggio.",
            "Zichtbaar vanaf 16.00 uur de dag vóór aankomst, lokale tijd van de accommodatie.",
        ],
    ),
    (
        [
            "At check-in time",
            "À l’heure d’arrivée",
            "A la hora de llegada",
            "Zur Anreisezeit",
            "All’orario di arrivo",
            "Op het aankomsttijdstip",
        ],
        [
            "Shown at the check-in time.",
            "Visible à l’heure d’arrivée.",
            "Visible a la hora de llegada.",
            "Sichtbar zur Anreisezeit.",
            "Visibile all’orario di arrivo.",
            "Zichtbaar op het aankomsttijdstip.",
        ],
    ),
];

const LOCKED_UNTIL: [&str; 6] = [
    "Available from {when}",
    "Disponible à partir du {when}",
    "Disponible a partir del {when}",
    "Verfügbar ab {when}",
    "Disponibile dal {when}",
    "Beschikbaar vanaf {when}",
];

const LOCKED_SOON: [&str; 6] = [
    "Available closer to your arrival.",
    "Disponible à l’approche de votre arrivée.",
    "Disponible cuando se acerque tu llegada.",
    "Verfügbar kurz vor Ihrer Anreise.",
    "Disponibile all’avvicinarsi del tuo arrivo.",
    "Beschikbaar kort voor je aankomst.",
];

#[cfg(test)]
mod tests {
    use super::*;

    fn utc(s: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc)
    }

    fn paris() -> Option<PropertyTz> {
        PropertyTz::parse("Europe/Paris")
    }

    #[test]
    fn always_reveals_without_a_stay_the_others_lock() {
        let now = utc("2026-07-19T12:00:00Z");
        assert!(RevealPolicy::Always.evaluate(now, None, None).revealed);
        for policy in &RevealPolicy::ALL[1..] {
            let decision = policy.evaluate(now, None, paris());
            assert!(!decision.revealed, "{policy:?}");
            assert_eq!(decision.available_from, None);
        }
    }

    #[test]
    fn each_policy_has_its_moment() {
        let checkin = utc("2026-07-20T14:00:00Z");
        let at = |policy: RevealPolicy| policy.reveal_at(checkin, paris());
        assert_eq!(at(RevealPolicy::Always), None);
        assert_eq!(
            at(RevealPolicy::HoursBefore24),
            Some(utc("2026-07-19T14:00:00Z"))
        );
        assert_eq!(at(RevealPolicy::AtCheckin), Some(checkin));
        // 16:00 CEST the day before; 16:00 CET in winter.
        assert_eq!(
            at(RevealPolicy::DayBefore16h),
            Some(utc("2026-07-19T14:00:00Z"))
        );
        assert_eq!(
            RevealPolicy::DayBefore16h.reveal_at(utc("2026-01-19T14:00:00Z"), paris()),
            Some(utc("2026-01-18T15:00:00Z"))
        );
        // New York, not Paris: 16:00 EDT.
        assert_eq!(
            RevealPolicy::DayBefore16h.reveal_at(checkin, PropertyTz::parse("America/New_York")),
            Some(utc("2026-07-19T20:00:00Z"))
        );
        // An unknown zone: 16:00 UTC, said so in the docs.
        assert_eq!(
            RevealPolicy::DayBefore16h.reveal_at(checkin, None),
            Some(utc("2026-07-19T16:00:00Z"))
        );
    }

    #[test]
    fn the_boundary_is_inclusive() {
        let checkin = Some(utc("2026-07-20T14:00:00Z"));
        let policy = RevealPolicy::HoursBefore24;
        assert!(
            policy
                .evaluate(utc("2026-07-19T14:00:00Z"), checkin, None)
                .revealed
        );
        assert!(
            !policy
                .evaluate(utc("2026-07-19T13:59:59Z"), checkin, None)
                .revealed
        );
    }

    #[test]
    fn wire_values_round_trip_and_old_spellings_read() {
        for (policy, wire) in RevealPolicy::ALL.iter().zip(RevealPolicy::WIRE_VALUES) {
            assert_eq!(policy.as_wire(), wire);
            assert_eq!(serde_json::to_value(policy).unwrap(), wire);
            let read: RevealPolicy = serde_json::from_value(wire.into()).unwrap();
            assert_eq!(&read, policy);
        }
        let old: RevealPolicy = serde_json::from_value("day_before16h".into()).unwrap();
        assert_eq!(old, RevealPolicy::DayBefore16h);
        assert_eq!(RevealPolicy::default(), RevealPolicy::DayBefore16h);
    }

    #[test]
    fn the_locked_message_speaks_the_guest_language_in_property_time() {
        let ctx = Context {
            locale: "en-GB".into(),
            ..Context::default()
        };
        let checkin = Some(utc("2026-07-20T14:00:00Z"));
        let early = utc("2026-07-19T10:00:00Z");
        let decision = RevealPolicy::DayBefore16h.evaluate(early, checkin, paris());
        assert_eq!(
            decision.locked_message(&ctx).as_deref(),
            Some("Available from Jul 19, 2026 · 16:00")
        );
        let unknown = RevealPolicy::AtCheckin.evaluate(early, None, paris());
        assert_eq!(
            unknown.locked_message(&ctx).as_deref(),
            Some("Available closer to your arrival.")
        );
        let open = RevealPolicy::Always.evaluate(early, None, None);
        assert_eq!(open.locked_message(&ctx), None);
        assert_eq!(open.show("x"), "x");
    }

    #[test]
    fn the_choice_list_offers_every_policy() {
        let list = serde_json::to_value(RevealPolicy::choice_list(
            "reveal_policy",
            RevealPolicy::AtCheckin,
            "fr",
        ))
        .unwrap();
        assert_eq!(list["value"], "at_checkin");
        let values: Vec<&str> = list["choices"]
            .as_array()
            .unwrap()
            .iter()
            .map(|choice| choice["value"].as_str().unwrap())
            .collect();
        assert_eq!(values, RevealPolicy::WIRE_VALUES);
        assert_eq!(list["choices"][2]["label"], "La veille à partir de 16 h");
    }
}
