//! Les valeurs de `TimeRange` et `WeeklyHours` : une grammaire, partagée par tous les modules.
//!
//! Une heure est `HH:MM` sur 24 h. Une plage est `HH:MM-HH:MM` ; une fin avant le début passe
//! minuit. Une semaine est `jour=plage,plage;jour=…` avec les jours `mon` … `sun`.

/// Les jours d'une semaine, dans l'ordre de [`parse_week`].
pub const DAYS: [&str; 7] = ["mon", "tue", "wed", "thu", "fri", "sat", "sun"];

/// Une plage horaire : `("16:00", "19:00")`.
pub type Range = (String, String);

fn time(raw: &str) -> Option<String> {
    let (h, m) = raw.trim().split_once(':')?;
    let (h, m): (u8, u8) = (h.parse().ok()?, m.parse().ok()?);
    (h < 24 && m < 60).then(|| format!("{h:02}:{m:02}"))
}

/// `"16:00-19:00"` → `Some(("16:00", "19:00"))`. Une valeur illisible ou une fin égale au début :
/// `None`.
pub fn parse_range(raw: &str) -> Option<Range> {
    let (start, end) = raw.split_once('-')?;
    let (start, end) = (time(start)?, time(end)?);
    (start != end).then_some((start, end))
}

/// Les plages de chaque jour, lundi d'abord. Un jour absent, vide ou illisible n'a aucune plage
/// (fermé) ; une plage illisible est ignorée.
pub fn parse_week(raw: &str) -> [Vec<Range>; 7] {
    let mut week: [Vec<Range>; 7] = Default::default();
    for part in raw.split(';') {
        let Some((day, ranges)) = part.split_once('=') else {
            continue;
        };
        let Some(i) = DAYS.iter().position(|d| *d == day.trim()) else {
            continue;
        };
        week[i] = ranges.split(',').filter_map(parse_range).collect();
    }
    week
}

/// L'inverse de [`parse_week`] : les jours fermés sont omis.
pub fn format_week(week: &[Vec<Range>; 7]) -> String {
    DAYS.iter()
        .zip(week)
        .filter(|(_, ranges)| !ranges.is_empty())
        .map(|(day, ranges)| {
            let ranges: Vec<String> = ranges.iter().map(|(s, e)| format!("{s}-{e}")).collect();
            format!("{day}={}", ranges.join(","))
        })
        .collect::<Vec<_>>()
        .join(";")
}

/// Deux plages du même jour se chevauchent-elles ? (Pour l'erreur « Deux plages se chevauchent
/// le lundi. ») Une plage qui passe minuit court jusqu'à la fin du jour.
pub fn overlaps(ranges: &[Range]) -> bool {
    let minutes = |t: &str| -> u32 {
        let (h, m) = t.split_once(':').unwrap_or(("0", "0"));
        h.parse::<u32>().unwrap_or(0) * 60 + m.parse::<u32>().unwrap_or(0)
    };
    let mut spans: Vec<(u32, u32)> = ranges
        .iter()
        .map(|(s, e)| {
            let (s, e) = (minutes(s), minutes(e));
            (s, if e < s { 24 * 60 } else { e })
        })
        .collect();
    spans.sort();
    spans.windows(2).any(|w| w[1].0 < w[0].1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_week_round_trips() {
        let raw = "mon=07:00-12:00,14:00-22:00;sun=9:00-12:00";
        let week = parse_week(raw);
        assert_eq!(week[0].len(), 2);
        assert!(week[1].is_empty());
        assert_eq!(week[6], vec![("09:00".into(), "12:00".into())]);
        assert_eq!(
            format_week(&week),
            "mon=07:00-12:00,14:00-22:00;sun=09:00-12:00"
        );
    }

    #[test]
    fn a_bad_range_is_dropped() {
        assert_eq!(parse_range("16:00-16:00"), None);
        assert_eq!(parse_range("25:00-26:00"), None);
        assert_eq!(
            parse_range("22:00-02:00"),
            Some(("22:00".into(), "02:00".into()))
        );
        assert!(parse_week("mon=x;xyz=07:00-08:00")[0].is_empty());
    }

    #[test]
    fn overlapping_ranges_are_seen() {
        let r = |s: &str, e: &str| (s.to_string(), e.to_string());
        assert!(overlaps(&[r("07:00", "12:00"), r("11:00", "14:00")]));
        assert!(!overlaps(&[r("07:00", "12:00"), r("12:00", "14:00")]));
        assert!(overlaps(&[r("22:00", "02:00"), r("23:00", "23:30")]));
    }
}
