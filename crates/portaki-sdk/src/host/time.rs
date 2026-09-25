//! `host::time` — the sandbox clock, the property's timezone, and dates written for a reader.
//!
//! The Wasm sandbox has no wall clock: [`now`] asks the host. Everything else here is pure.
//!
//! # Dates in the reader's language
//!
//! [`short_date`], [`long_date`], [`date_time`], [`hm`], [`weekday_name`], [`month_abbr`],
//! [`elapsed`] and [`ago`] write a date or a duration in the languages the SDK ships — `en`, `fr`,
//! `es`, `de`, `it`, `nl`; any other language reads English. They take a short language code:
//! `ctx.lang()` for the reader, `"fr"` / `"en"` when a text is built in every language at once.
//!
//! ```
//! use chrono::{TimeZone, Utc};
//! use portaki_sdk::host::time;
//!
//! let at = Utc.with_ymd_and_hms(2026, 9, 12, 14, 5, 0).unwrap();
//! assert_eq!(time::short_date(at, "fr"), "12 sept.");
//! assert_eq!(time::short_date(at, "en"), "Sep 12");
//! assert_eq!(time::long_date(at, "en"), "Sep 12, 2026");
//! assert_eq!(time::date_time(at, "fr"), "12 sept. 2026 · 14:05");
//! let now = at + chrono::Duration::hours(3);
//! assert_eq!(time::ago(at, now, "fr"), "il y a 3 h");
//! assert_eq!(time::elapsed(at, now, "en"), "3 h");
//! ```
//!
//! # The property's timezone
//!
//! A date shown to a guest is in the property's time, not UTC: [`PropertyTz`], from
//! [`Context::property_tz`](crate::context::Context::property_tz).
//!
//! ```
//! use chrono::{TimeZone, Utc};
//! use portaki_sdk::host::time::{self, PropertyTz};
//!
//! let paris = PropertyTz::parse("Europe/Paris").unwrap();
//! let checkout = Utc.with_ymd_and_hms(2026, 7, 20, 8, 0, 0).unwrap();
//! assert_eq!(time::hm(paris.to_local(checkout)), "10:00"); // CEST
//! assert_eq!(PropertyTz::parse("Mars/Olympus_Mons"), None);
//! ```

use chrono::{
    DateTime, Datelike, Days, Duration, FixedOffset, NaiveDate, NaiveDateTime, Timelike, Utc,
    Weekday,
};

use crate::error::{PortakiError, Result};
use crate::host::runtime::backend;

/// Returns the current UTC time from the gateway clock.
pub fn now() -> Result<DateTime<Utc>> {
    let iso = backend()?.time_now_iso()?;
    DateTime::parse_from_rfc3339(&iso)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|e| PortakiError::Host(format!("time_now_parse_failed: {e}")))
}

// ── Timezone ────────────────────────────────────────────────────────────────

/// The property's timezone: a standard offset and its daylight-saving rule.
///
/// Built from an IANA name by [`PropertyTz::parse`], which knows Europe (every zone on the EU
/// rule), North America (US / Canada rule), the French overseas territories and the common zones
/// without daylight saving. Another name is `None` — say so, or fall back to UTC calendar math
/// knowingly; never to Paris.
// ponytail: a table, not the tz database (chrono-tz more than doubles a module's wasm). A zone
// with another rule (southern hemisphere, Morocco, Israel…) is None; add a row or a rule then.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PropertyTz {
    /// Standard (winter) offset east of UTC, in seconds.
    standard: i32,
    rule: Dst,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Dst {
    None,
    /// Last Sunday of March 01:00 UTC → last Sunday of October 01:00 UTC.
    Eu,
    /// Second Sunday of March 02:00 local → first Sunday of November 02:00 local.
    NorthAmerica,
}

const H: i32 = 3600;

/// IANA name → standard offset and rule.
const ZONES: &[(&str, i32, Dst)] = &[
    ("UTC", 0, Dst::None),
    ("Etc/UTC", 0, Dst::None),
    ("GMT", 0, Dst::None),
    ("Etc/GMT", 0, Dst::None),
    // Western Europe
    ("Europe/London", 0, Dst::Eu),
    ("Europe/Dublin", 0, Dst::Eu),
    ("Europe/Lisbon", 0, Dst::Eu),
    ("Atlantic/Canary", 0, Dst::Eu),
    ("Atlantic/Madeira", 0, Dst::Eu),
    ("Atlantic/Faroe", 0, Dst::Eu),
    ("Europe/Guernsey", 0, Dst::Eu),
    ("Europe/Jersey", 0, Dst::Eu),
    ("Europe/Isle_of_Man", 0, Dst::Eu),
    ("Atlantic/Azores", -H, Dst::Eu),
    // Central Europe
    ("Europe/Paris", H, Dst::Eu),
    ("Europe/Berlin", H, Dst::Eu),
    ("Europe/Madrid", H, Dst::Eu),
    ("Europe/Rome", H, Dst::Eu),
    ("Europe/Brussels", H, Dst::Eu),
    ("Europe/Amsterdam", H, Dst::Eu),
    ("Europe/Vienna", H, Dst::Eu),
    ("Europe/Zurich", H, Dst::Eu),
    ("Europe/Luxembourg", H, Dst::Eu),
    ("Europe/Monaco", H, Dst::Eu),
    ("Europe/Oslo", H, Dst::Eu),
    ("Europe/Stockholm", H, Dst::Eu),
    ("Europe/Copenhagen", H, Dst::Eu),
    ("Europe/Prague", H, Dst::Eu),
    ("Europe/Warsaw", H, Dst::Eu),
    ("Europe/Budapest", H, Dst::Eu),
    ("Europe/Zagreb", H, Dst::Eu),
    ("Europe/Ljubljana", H, Dst::Eu),
    ("Europe/Bratislava", H, Dst::Eu),
    ("Europe/Belgrade", H, Dst::Eu),
    ("Europe/Sarajevo", H, Dst::Eu),
    ("Europe/Skopje", H, Dst::Eu),
    ("Europe/Podgorica", H, Dst::Eu),
    ("Europe/Tirane", H, Dst::Eu),
    ("Europe/Andorra", H, Dst::Eu),
    ("Europe/Malta", H, Dst::Eu),
    ("Europe/Vatican", H, Dst::Eu),
    ("Europe/San_Marino", H, Dst::Eu),
    ("Europe/Gibraltar", H, Dst::Eu),
    ("Europe/Vaduz", H, Dst::Eu),
    ("Europe/Busingen", H, Dst::Eu),
    ("Africa/Ceuta", H, Dst::Eu),
    ("Arctic/Longyearbyen", H, Dst::Eu),
    // Eastern Europe
    ("Europe/Athens", 2 * H, Dst::Eu),
    ("Europe/Helsinki", 2 * H, Dst::Eu),
    ("Europe/Mariehamn", 2 * H, Dst::Eu),
    ("Europe/Bucharest", 2 * H, Dst::Eu),
    ("Europe/Sofia", 2 * H, Dst::Eu),
    ("Europe/Riga", 2 * H, Dst::Eu),
    ("Europe/Tallinn", 2 * H, Dst::Eu),
    ("Europe/Vilnius", 2 * H, Dst::Eu),
    ("Europe/Kyiv", 2 * H, Dst::Eu),
    ("Europe/Kiev", 2 * H, Dst::Eu),
    ("Asia/Nicosia", 2 * H, Dst::Eu),
    ("Europe/Nicosia", 2 * H, Dst::Eu),
    // No daylight saving
    ("Europe/Istanbul", 3 * H, Dst::None),
    ("Europe/Moscow", 3 * H, Dst::None),
    ("Europe/Minsk", 3 * H, Dst::None),
    ("Africa/Johannesburg", 2 * H, Dst::None),
    ("Africa/Nairobi", 3 * H, Dst::None),
    ("Africa/Abidjan", 0, Dst::None),
    ("Africa/Dakar", 0, Dst::None),
    ("Africa/Tunis", H, Dst::None),
    ("Africa/Algiers", H, Dst::None),
    ("Asia/Dubai", 4 * H, Dst::None),
    ("Asia/Kolkata", 5 * H + 1800, Dst::None),
    ("Asia/Bangkok", 7 * H, Dst::None),
    ("Asia/Singapore", 8 * H, Dst::None),
    ("Asia/Hong_Kong", 8 * H, Dst::None),
    ("Asia/Shanghai", 8 * H, Dst::None),
    ("Asia/Tokyo", 9 * H, Dst::None),
    ("Asia/Seoul", 9 * H, Dst::None),
    ("America/Phoenix", -7 * H, Dst::None),
    ("Pacific/Honolulu", -10 * H, Dst::None),
    ("America/Sao_Paulo", -3 * H, Dst::None),
    ("America/Argentina/Buenos_Aires", -3 * H, Dst::None),
    ("America/Mexico_City", -6 * H, Dst::None),
    ("America/Bogota", -5 * H, Dst::None),
    ("America/Lima", -5 * H, Dst::None),
    ("America/Panama", -5 * H, Dst::None),
    ("America/Cancun", -5 * H, Dst::None),
    ("America/Puerto_Rico", -4 * H, Dst::None),
    ("America/Santo_Domingo", -4 * H, Dst::None),
    // French overseas territories
    ("America/Guadeloupe", -4 * H, Dst::None),
    ("America/Martinique", -4 * H, Dst::None),
    ("America/St_Barthelemy", -4 * H, Dst::None),
    ("America/Marigot", -4 * H, Dst::None),
    ("America/Cayenne", -3 * H, Dst::None),
    ("Indian/Reunion", 4 * H, Dst::None),
    ("Indian/Mayotte", 3 * H, Dst::None),
    ("Indian/Mauritius", 4 * H, Dst::None),
    ("Pacific/Tahiti", -10 * H, Dst::None),
    ("Pacific/Marquesas", -9 * H - 1800, Dst::None),
    ("Pacific/Gambier", -9 * H, Dst::None),
    ("Pacific/Noumea", 11 * H, Dst::None),
    ("Pacific/Wallis", 12 * H, Dst::None),
    // North America
    ("America/Miquelon", -3 * H, Dst::NorthAmerica),
    ("America/St_Johns", -3 * H - 1800, Dst::NorthAmerica),
    ("America/Halifax", -4 * H, Dst::NorthAmerica),
    ("America/New_York", -5 * H, Dst::NorthAmerica),
    ("America/Toronto", -5 * H, Dst::NorthAmerica),
    ("America/Montreal", -5 * H, Dst::NorthAmerica),
    ("America/Detroit", -5 * H, Dst::NorthAmerica),
    ("America/Chicago", -6 * H, Dst::NorthAmerica),
    ("America/Winnipeg", -6 * H, Dst::NorthAmerica),
    ("America/Denver", -7 * H, Dst::NorthAmerica),
    ("America/Edmonton", -7 * H, Dst::NorthAmerica),
    ("America/Los_Angeles", -8 * H, Dst::NorthAmerica),
    ("America/Vancouver", -8 * H, Dst::NorthAmerica),
    ("America/Anchorage", -9 * H, Dst::NorthAmerica),
];

impl PropertyTz {
    /// The zone named `name` (IANA, e.g. `Europe/Paris`, any case), `None` when the SDK does
    /// not know its rule.
    pub fn parse(name: &str) -> Option<Self> {
        let name = name.trim();
        ZONES
            .iter()
            .find(|(known, _, _)| known.eq_ignore_ascii_case(name))
            .map(|&(_, standard, rule)| Self { standard, rule })
    }

    /// The offset in force at `at`.
    pub fn offset_at(&self, at: DateTime<Utc>) -> FixedOffset {
        let daylight = match self.rule {
            Dst::None => false,
            Dst::Eu => {
                let start = sunday(at.year(), 3, Nth::Last).and_hms_opt(1, 0, 0);
                let end = sunday(at.year(), 10, Nth::Last).and_hms_opt(1, 0, 0);
                between(at, start, end)
            }
            Dst::NorthAmerica => {
                // 02:00 local: standard time at the start, daylight time at the end.
                let utc = |day: NaiveDate, offset: i32| {
                    day.and_hms_opt(2, 0, 0)
                        .map(|local| local - Duration::seconds(i64::from(offset)))
                };
                let start = utc(sunday(at.year(), 3, Nth::Second), self.standard);
                let end = utc(sunday(at.year(), 11, Nth::First), self.standard + H);
                between(at, start, end)
            }
        };
        let seconds = self.standard + if daylight { H } else { 0 };
        FixedOffset::east_opt(seconds).expect("offsets of the table are within a day")
    }

    /// `at` in the property's local time.
    pub fn to_local(&self, at: DateTime<Utc>) -> DateTime<FixedOffset> {
        at.with_timezone(&self.offset_at(at))
    }

    /// The instant of a local wall-clock time. A time that happens twice (the autumn change)
    /// is the first; a time that never happens (the spring gap) is read with the winter offset,
    /// which lands just after the change.
    pub fn from_local(&self, local: NaiveDateTime) -> DateTime<Utc> {
        let at = |offset: i32| local.and_utc() - Duration::seconds(i64::from(offset));
        [self.standard + H, self.standard]
            .into_iter()
            .find(|&offset| self.offset_at(at(offset)).local_minus_utc() == offset)
            .map_or_else(|| at(self.standard), at)
    }
}

enum Nth {
    First,
    Second,
    Last,
}

/// The first, second or last Sunday of `month`.
fn sunday(year: i32, month: u32, nth: Nth) -> NaiveDate {
    let first = NaiveDate::from_ymd_opt(year, month, 1).expect("a month of the rules");
    let first_sunday = first
        + Days::new(u64::from(
            (7 - first.weekday().days_since(Weekday::Sun)) % 7,
        ));
    match nth {
        Nth::First => first_sunday,
        Nth::Second => first_sunday + Days::new(7),
        Nth::Last => {
            let mut last = first_sunday;
            while (last + Days::new(7)).month() == month {
                last = last + Days::new(7);
            }
            last
        }
    }
}

fn between(at: DateTime<Utc>, start: Option<NaiveDateTime>, end: Option<NaiveDateTime>) -> bool {
    match (start, end) {
        (Some(start), Some(end)) => at.naive_utc() >= start && at.naive_utc() < end,
        _ => false,
    }
}

// ── Dates for a reader ──────────────────────────────────────────────────────

/// The languages dates are written in; any other reads English.
pub const LANGUAGES: [&str; 6] = ["en", "fr", "es", "de", "it", "nl"];

pub(crate) fn column(lang: &str) -> usize {
    let lang = lang.trim().to_ascii_lowercase();
    let lang = lang.split(['-', '_']).next().unwrap_or_default();
    LANGUAGES
        .iter()
        .position(|known| *known == lang)
        .unwrap_or(0)
}

const MONTHS: [[&str; 12]; 6] = [
    [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ],
    [
        "janv.", "févr.", "mars", "avr.", "mai", "juin", "juil.", "août", "sept.", "oct.", "nov.",
        "déc.",
    ],
    [
        "ene", "feb", "mar", "abr", "may", "jun", "jul", "ago", "sept", "oct", "nov", "dic",
    ],
    [
        "Jan.", "Feb.", "März", "Apr.", "Mai", "Juni", "Juli", "Aug.", "Sept.", "Okt.", "Nov.",
        "Dez.",
    ],
    [
        "gen", "feb", "mar", "apr", "mag", "giu", "lug", "ago", "set", "ott", "nov", "dic",
    ],
    [
        "jan", "feb", "mrt", "apr", "mei", "jun", "jul", "aug", "sep", "okt", "nov", "dec",
    ],
];

const WEEKDAYS: [[&str; 7]; 6] = [
    [
        "Monday",
        "Tuesday",
        "Wednesday",
        "Thursday",
        "Friday",
        "Saturday",
        "Sunday",
    ],
    [
        "lundi", "mardi", "mercredi", "jeudi", "vendredi", "samedi", "dimanche",
    ],
    [
        "lunes",
        "martes",
        "miércoles",
        "jueves",
        "viernes",
        "sábado",
        "domingo",
    ],
    [
        "Montag",
        "Dienstag",
        "Mittwoch",
        "Donnerstag",
        "Freitag",
        "Samstag",
        "Sonntag",
    ],
    [
        "lunedì",
        "martedì",
        "mercoledì",
        "giovedì",
        "venerdì",
        "sabato",
        "domenica",
    ],
    [
        "maandag",
        "dinsdag",
        "woensdag",
        "donderdag",
        "vrijdag",
        "zaterdag",
        "zondag",
    ],
];

/// `just now`, `{} ago`; then the units of [`elapsed`]: minute, hour, day, week, month.
const RELATIVE: [[&str; 7]; 6] = [
    ["just now", "{} ago", "min", "h", "d", "wk", "mo"],
    ["à l’instant", "il y a {}", "min", "h", "j", "sem.", "mois"],
    ["ahora mismo", "hace {}", "min", "h", "d", "sem.", "mes."],
    ["gerade eben", "vor {}", "Min.", "Std.", "T.", "Wo.", "Mon."],
    ["proprio ora", "{} fa", "min", "h", "g", "sett.", "mesi"],
    ["zojuist", "{} geleden", "min", "u", "d", "wk", "mnd"],
];

/// Abbreviated month name, `month` 1–12 (`sept.`, `Sep`); `""` out of range.
pub fn month_abbr(month: u32, lang: &str) -> &'static str {
    let index = month.wrapping_sub(1) as usize;
    MONTHS[column(lang)].get(index).copied().unwrap_or("")
}

/// Full weekday name (`lundi`, `Monday`).
pub fn weekday_name(weekday: Weekday, lang: &str) -> &'static str {
    WEEKDAYS[column(lang)][weekday.num_days_from_monday() as usize]
}

/// Day and month: `12 sept.`, `Sep 12`, `12. Sept.`.
pub fn short_date(date: impl Datelike, lang: &str) -> String {
    day_month(date.day(), date.month(), lang)
}

/// Day, month and year: `12 sept. 2026`, `Sep 12, 2026`.
pub fn long_date(date: impl Datelike, lang: &str) -> String {
    let day_month = day_month(date.day(), date.month(), lang);
    match LANGUAGES[column(lang)] {
        "en" => format!("{day_month}, {}", date.year()),
        _ => format!("{day_month} {}", date.year()),
    }
}

fn day_month(day: u32, month: u32, lang: &str) -> String {
    let month = month_abbr(month, lang);
    match LANGUAGES[column(lang)] {
        "en" => format!("{month} {day}"),
        "de" => format!("{day}. {month}"),
        _ => format!("{day} {month}"),
    }
}

/// `14:05`.
pub fn hm(time: impl Timelike) -> String {
    format!("{:02}:{:02}", time.hour(), time.minute())
}

/// [`long_date`] and [`hm`]: `12 sept. 2026 · 14:05`. Convert to the property's time first
/// ([`PropertyTz::to_local`]) — a UTC instant reads in UTC.
pub fn date_time<T: Datelike + Timelike + Copy>(at: T, lang: &str) -> String {
    format!("{} · {}", long_date(at, lang), hm(at))
}

/// Time between `at` and `now`, compact for a tile: `12 min`, `3 h` (under 48 hours), `2 j`,
/// `3 sem.` (from 14 days), `2 mois` (from 60 days). A future `at` reads as `0 min`.
pub fn elapsed(at: DateTime<Utc>, now: DateTime<Utc>, lang: &str) -> String {
    let units = RELATIVE[column(lang)];
    let minutes = (now - at).num_minutes().max(0);
    let days = minutes / (24 * 60);
    let (n, unit) = match minutes {
        m if m < 60 => (m, units[2]),
        m if m < 48 * 60 => (m / 60, units[3]),
        _ if days < 14 => (days, units[4]),
        _ if days < 60 => (days / 7, units[5]),
        _ => (days / 30, units[6]),
    };
    format!("{n} {unit}")
}

/// [`elapsed`] as a phrase: `il y a 3 h`, `3 h ago`; `à l’instant` under a minute.
pub fn ago(at: DateTime<Utc>, now: DateTime<Utc>, lang: &str) -> String {
    let texts = RELATIVE[column(lang)];
    if (now - at).num_minutes() < 1 {
        return texts[0].to_string();
    }
    texts[1].replace("{}", &elapsed(at, now, lang))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn utc(s: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc)
    }

    #[test]
    fn europe_switches_on_the_last_sundays_at_one_utc() {
        let paris = PropertyTz::parse("europe/paris").unwrap();
        let offset = |s: &str| paris.offset_at(utc(s)).local_minus_utc() / H;
        assert_eq!(offset("2026-03-29T00:59:59Z"), 1);
        assert_eq!(offset("2026-03-29T01:00:00Z"), 2);
        assert_eq!(offset("2026-10-25T00:59:59Z"), 2);
        assert_eq!(offset("2026-10-25T01:00:00Z"), 1);
        let london = PropertyTz::parse("Europe/London").unwrap();
        assert_eq!(
            london
                .offset_at(utc("2026-07-01T12:00:00Z"))
                .local_minus_utc(),
            H
        );
    }

    #[test]
    fn north_america_switches_at_two_local() {
        let ny = PropertyTz::parse("America/New_York").unwrap();
        let offset = |s: &str| ny.offset_at(utc(s)).local_minus_utc() / H;
        // 8 March 2026 02:00 EST = 07:00 UTC; 1 November 2026 02:00 EDT = 06:00 UTC.
        assert_eq!(offset("2026-03-08T06:59:59Z"), -5);
        assert_eq!(offset("2026-03-08T07:00:00Z"), -4);
        assert_eq!(offset("2026-11-01T05:59:59Z"), -4);
        assert_eq!(offset("2026-11-01T06:00:00Z"), -5);
    }

    #[test]
    fn sundays_of_the_rules() {
        let day = |y, m, nth| sunday(y, m, nth).to_string();
        assert_eq!(day(2026, 3, Nth::Last), "2026-03-29");
        assert_eq!(day(2026, 3, Nth::Second), "2026-03-08");
        assert_eq!(day(2026, 11, Nth::First), "2026-11-01");
        assert_eq!(day(2026, 10, Nth::Last), "2026-10-25");
    }

    #[test]
    fn fixed_zones_and_unknown_ones() {
        let reunion = PropertyTz::parse("Indian/Reunion").unwrap();
        assert_eq!(
            reunion
                .offset_at(utc("2026-07-01T00:00:00Z"))
                .local_minus_utc(),
            4 * H
        );
        assert_eq!(PropertyTz::parse("Australia/Sydney"), None);
        assert_eq!(PropertyTz::parse(""), None);
    }

    #[test]
    fn local_times_resolve_through_the_changes() {
        let paris = PropertyTz::parse("Europe/Paris").unwrap();
        let local = |s: &str| NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M").unwrap();
        assert_eq!(
            paris.from_local(local("2026-07-19 16:00")),
            utc("2026-07-19T14:00:00Z")
        );
        assert_eq!(
            paris.from_local(local("2026-01-18 16:00")),
            utc("2026-01-18T15:00:00Z")
        );
        // 02:30 does not exist on 29 March; 02:30 happens twice on 25 October.
        assert_eq!(
            paris.from_local(local("2026-03-29 02:30")),
            utc("2026-03-29T01:30:00Z")
        );
        assert_eq!(
            paris.from_local(local("2026-10-25 02:30")),
            utc("2026-10-25T00:30:00Z")
        );
    }

    #[test]
    fn dates_in_each_language() {
        let at = utc("2026-09-12T14:05:00Z");
        let shown: Vec<String> = ["fr", "en", "es", "de", "it", "nl", "ja"]
            .iter()
            .map(|lang| long_date(at, lang))
            .collect();
        assert_eq!(
            shown,
            [
                "12 sept. 2026",
                "Sep 12, 2026",
                "12 sept 2026",
                "12. Sept. 2026",
                "12 set 2026",
                "12 sep 2026",
                "Sep 12, 2026",
            ]
        );
        assert_eq!(weekday_name(at.weekday(), "fr-FR"), "samedi");
        assert_eq!(month_abbr(13, "fr"), "");
        assert_eq!(month_abbr(0, "fr"), "");
    }

    #[test]
    fn durations_grow_their_unit() {
        let now = utc("2026-09-12T12:00:00Z");
        let before = |minutes: i64| now - Duration::minutes(minutes);
        let shown: Vec<String> = [0, 12, 3 * 60, 47 * 60, 2 * 1440, 20 * 1440, 90 * 1440]
            .iter()
            .map(|&m| elapsed(before(m), now, "fr"))
            .collect();
        assert_eq!(
            shown,
            ["0 min", "12 min", "3 h", "47 h", "2 j", "2 sem.", "3 mois"]
        );
        assert_eq!(ago(now, now, "fr"), "à l’instant");
        assert_eq!(ago(before(12), now, "en"), "12 min ago");
        assert_eq!(ago(before(2 * 1440), now, "de"), "vor 2 T.");
        assert_eq!(elapsed(now + Duration::hours(1), now, "en"), "0 min");
    }
}
