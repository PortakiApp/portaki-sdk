//! The common rules of the host settings (règles communes de la config hôte, §6), with their
//! messages. Each check answers `None` when the value is fine, or the message to show — under the
//! field (`Field::error`) and in the module's `publishReadiness`, so both say the same thing.
//!
//! The checks are on what is stored: a time is `HH:MM`, a phone number E.164. An empty value is
//! fine here — whether a field may be empty is `required`, not these rules.
//!
//! ```
//! use portaki_sdk::config::check;
//!
//! assert!(check::max_chars("Boîte grise à gauche du portail", 60).is_none());
//! let error = check::max_chars(&"x".repeat(61), 60).unwrap();
//! assert_eq!(error.get("fr"), "60 caractères au maximum.");
//! ```

use crate::contracts::i18n::I18nText;

/// `texte`, `texte long`, `riche`: at most `max` characters, spaces at both ends not counted.
pub fn max_chars(value: &str, max: usize) -> Option<I18nText> {
    (value.trim().chars().count() > max).then(|| {
        I18nText::new(
            format!("{max} caractères au maximum."),
            format!("{max} characters at most."),
        )
    })
}

/// `int` / `décimal`: within `min..=max`.
pub fn between(value: f64, min: f64, max: f64) -> Option<I18nText> {
    (value < min || value > max).then(|| {
        I18nText::new(
            format!("Entre {} et {}.", number(min), number(max)),
            format!("Between {} and {}.", number(min), number(max)),
        )
    })
}

/// `heure`: `HH:MM`, 24 h.
pub fn time(value: &str) -> Option<I18nText> {
    (!value.is_empty() && parse_time(value).is_none())
        .then(|| I18nText::new("Heure au format 16:00.", "Time as 16:00."))
}

/// `plage`: two valid times that differ. An end before the start runs past midnight — say so in
/// the field's help, it is not an error.
pub fn time_range(start: &str, end: &str) -> Option<I18nText> {
    if let Some(error) = time(start).or_else(|| time(end)) {
        return Some(error);
    }
    (!start.is_empty() && start == end).then(|| {
        I18nText::new(
            "L'heure de fin doit être différente de l'heure de début.",
            "The end time must differ from the start time.",
        )
    })
}

/// `téléphone`: E.164, a `+`, a country code and up to 15 digits.
// ponytail: shape only, not each country's numbering plan — a libphonenumber check if hosts get
// numbers past it that cannot be called.
pub fn phone(value: &str) -> Option<I18nText> {
    let digits = value.strip_prefix('+').unwrap_or("");
    let valid = (7..=15).contains(&digits.len())
        && digits.bytes().all(|b| b.is_ascii_digit())
        && !digits.starts_with('0');
    (!value.is_empty() && !valid).then(|| {
        I18nText::new(
            "Ce numéro n'est pas valide. Vérifiez l'indicatif.",
            "This number is not valid. Check the country code.",
        )
    })
}

/// `url`: `https://` and a host. Whether the host answers is a warning the platform checks at
/// publication, not this rule.
pub fn https_url(value: &str) -> Option<I18nText> {
    let host = value
        .strip_prefix("https://")
        .map(|rest| rest.split(['/', '?', '#']).next().unwrap_or(""));
    (!value.is_empty() && !host.is_some_and(|h| h.contains('.') && !h.contains(' '))).then(|| {
        I18nText::new(
            "L'adresse doit commencer par https://",
            "The address must start with https://",
        )
    })
}

/// `enum`: one of the options.
pub fn one_of(value: &str, options: &[&str]) -> Option<I18nText> {
    (!options.contains(&value))
        .then(|| I18nText::new("Choisissez une option.", "Choose an option."))
}

/// `set<enum>` required: at least one option chosen.
pub fn at_least_one<T>(values: &[T]) -> Option<I18nText> {
    values.is_empty().then(|| {
        I18nText::new(
            "Choisissez au moins une option.",
            "Choose at least one option.",
        )
    })
}

/// `géo`: a placed pin. Its distance to the property is [`within_km`], a warning.
pub fn pin(lat: Option<f64>, lng: Option<f64>) -> Option<I18nText> {
    let placed = matches!((lat, lng), (Some(lat), Some(lng))
        if (-90.0..=90.0).contains(&lat) && (-180.0..=180.0).contains(&lng));
    (!placed).then(|| {
        I18nText::new(
            "Placez l'épingle sur la carte.",
            "Place the pin on the map.",
        )
    })
}

/// `géo`: `true` when the pin is within `km` of the property (200 km in the rules) — farther is a
/// warning, never an error.
pub fn within_km(lat: f64, lng: f64, home_lat: f64, home_lng: f64, km: f64) -> bool {
    let (p1, p2) = (lat.to_radians(), home_lat.to_radians());
    let dp = (home_lat - lat).to_radians();
    let dl = (home_lng - lng).to_radians();
    let a = (dp / 2.0).sin().powi(2) + p1.cos() * p2.cos() * (dl / 2.0).sin().powi(2);
    6371.0 * 2.0 * a.sqrt().asin() <= km
}

fn parse_time(value: &str) -> Option<(u8, u8)> {
    let (h, m) = value.split_once(':')?;
    if h.len() != 2 || m.len() != 2 {
        return None;
    }
    let (h, m): (u8, u8) = (h.parse().ok()?, m.parse().ok()?);
    (h < 24 && m < 60).then_some((h, m))
}

/// `1`, `2.5` — no trailing `.0` in a message.
fn number(value: f64) -> String {
    if value.fract() == 0.0 {
        format!("{}", value as i64)
    } else {
        format!("{value}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fr(text: Option<I18nText>) -> Option<String> {
        text.map(|t| t.get("fr").to_string())
    }

    #[test]
    fn each_rule_says_the_message_of_the_spec() {
        assert_eq!(
            fr(max_chars(&"é".repeat(61), 60)).as_deref(),
            Some("60 caractères au maximum.")
        );
        assert_eq!(fr(max_chars(&format!("  {}  ", "a".repeat(60)), 60)), None);
        assert_eq!(
            fr(between(61.0, 1.0, 60.0)).as_deref(),
            Some("Entre 1 et 60.")
        );
        assert_eq!(fr(between(0.5, 0.5, 2.5)), None);
        assert_eq!(fr(time("24:00")).as_deref(), Some("Heure au format 16:00."));
        assert_eq!(fr(time("9:00")).as_deref(), Some("Heure au format 16:00."));
        assert_eq!(fr(time("23:59")), None);
        assert_eq!(
            fr(time_range("16:00", "16:00")).as_deref(),
            Some("L'heure de fin doit être différente de l'heure de début.")
        );
        assert_eq!(
            fr(time_range("22:00", "02:00")),
            None,
            "passer minuit est permis"
        );
        assert_eq!(
            fr(phone("0612345678")).as_deref(),
            Some("Ce numéro n'est pas valide. Vérifiez l'indicatif.")
        );
        assert_eq!(fr(phone("+33612345678")), None);
        assert_eq!(
            fr(https_url("http://a.fr")).as_deref(),
            Some("L'adresse doit commencer par https://")
        );
        assert_eq!(
            fr(https_url("https://")).as_deref(),
            Some("L'adresse doit commencer par https://")
        );
        assert_eq!(fr(https_url("https://portaki.app/livret?x=1")), None);
        assert_eq!(
            fr(one_of("wep", &["wpa", "open"])).as_deref(),
            Some("Choisissez une option.")
        );
        assert_eq!(
            fr(at_least_one::<&str>(&[])).as_deref(),
            Some("Choisissez au moins une option.")
        );
        assert_eq!(
            fr(pin(Some(45.0), None)).as_deref(),
            Some("Placez l'épingle sur la carte.")
        );
        assert_eq!(fr(pin(Some(45.0), Some(6.0))), None);
    }

    #[test]
    fn an_empty_value_is_left_to_required() {
        assert_eq!(time(""), None);
        assert_eq!(phone(""), None);
        assert_eq!(https_url(""), None);
    }

    #[test]
    fn two_hundred_km_around_the_property() {
        // L'Islette (Indre-et-Loire) → Tours, ~25 km ; → Marseille, ~600 km.
        assert!(within_km(47.39, 0.69, 47.25, 0.43, 200.0));
        assert!(!within_km(43.30, 5.37, 47.25, 0.43, 200.0));
    }
}
