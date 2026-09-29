//! Warn that a newer version exists, without ever getting in the way.
//!
//! # What would make this notice harmful
//!
//! A network call on every command. `portaki build` would become slower because one day someone
//! might want to know that a version has shipped — the opposite of the service rendered. The
//! answer is therefore cached for a whole day, and the one command that refreshes it gives up
//! after a second and a half.
//!
//! # Where it does not show
//!
//! Under `--plain`: that output is meant to be read by a program, and one more line there is one
//! more field to filter out. Not outside a terminal either — a CI log has nobody to act on it,
//! and `portaki ci check` already says there what is ageing. And never if
//! `PORTAKI_NO_UPDATE_CHECK` is set.
//!
//! The notice comes **after** the command, once its work is delivered: it delays nothing that was
//! being waited for, and does not push out of sight the line one came to read.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::ui;

/// The published version stays good for a day: beyond that we ask again, within it we keep quiet.
const FRESH_FOR: Duration = Duration::from_secs(24 * 60 * 60);

/// Beyond that, the question is no longer worth the time it takes.
const GIVE_UP_AFTER: Duration = Duration::from_millis(1_500);

/// The variable that turns it all off, for whoever would rather not hear about it.
const OPT_OUT: &str = "PORTAKI_NO_UPDATE_CHECK";

/// Says that a newer version exists, if it does and if someone is there to read it.
pub async fn notify() {
    if !wanted() {
        return;
    }
    let Some(latest) = latest().await else {
        return;
    };
    let running = env!("CARGO_PKG_VERSION");
    if !outdated(running, &latest) {
        return;
    }

    ui::blank();
    ui::warn(format!("portaki {running} → {latest} is available"));
    ui::detail("cargo install portaki-cli --locked --force");
    ui::detail(format!("{OPT_OUT}=1 silences this"));
}

/// Is there anyone to read it, and do they want it?
fn wanted() -> bool {
    !ui::plain() && console::user_attended() && std::env::var_os(OPT_OUT).is_none()
}

/// The latest published version, from the cache while it is fresh.
async fn latest() -> Option<String> {
    if let Some(cached) = read_cache() {
        return Some(cached);
    }
    let fetched = tokio::time::timeout(GIVE_UP_AFTER, latest_published("portaki-cli"))
        .await
        .ok()?
        .ok()?;
    write_cache(&fetched);
    Some(fetched)
}

/// The latest stable version of a crate published on crates.io.
///
/// The only question asked of crates.io: the update notice and `portaki ci check` both ask it,
/// for the CLI and for the SDK.
pub async fn latest_published(krate: &str) -> anyhow::Result<String> {
    use anyhow::Context;
    let body: serde_json::Value = crate::http::client()
        .get(format!("https://crates.io/api/v1/crates/{krate}"))
        // crates.io refuses a request without an identifiable agent, and says so with a 403.
        .header(
            "User-Agent",
            concat!("portaki-cli/", env!("CARGO_PKG_VERSION")),
        )
        .send()
        .await?
        .json()
        .await?;
    body.pointer("/crate/max_stable_version")
        .or_else(|| body.pointer("/crate/newest_version"))
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
        .context("crates.io did not say which version is newest")
}

fn read_cache() -> Option<String> {
    parse_cache(&std::fs::read_to_string(cache_path()?).ok()?, now())
}

/// `<timestamp> <version>` — two fields, one line, no format to keep evolving.
///
/// Kept apart from reading the file so that it can be checked: a damaged cache must read as no
/// answer, never as a version, otherwise a truncated file would have anything at all announced
/// as the latest published version.
fn parse_cache(raw: &str, now: Duration) -> Option<String> {
    let (stamped, version) = raw.trim().split_once(' ')?;
    let stamped = Duration::from_secs(stamped.parse().ok()?);
    if version.is_empty() || now.checked_sub(stamped)? > FRESH_FOR {
        return None;
    }
    Some(version.to_string())
}

fn write_cache(version: &str) {
    let Some(path) = cache_path() else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    // A cache that fails to be written breaks nothing: we will ask again, that is all.
    let _ = std::fs::write(&path, format!("{} {version}", now().as_secs()));
}

fn cache_path() -> Option<std::path::PathBuf> {
    let base = std::env::var_os("XDG_CACHE_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|home| std::path::PathBuf::from(home).join(".cache"))
        })?;
    Some(base.join("portaki").join("latest-version"))
}

fn now() -> Duration {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
}

/// Is `running` behind `latest`?
///
/// Compared component by component, as numbers: `2.10.0` comes after `2.9.0`, which a
/// lexicographic ordering would get backwards.
pub fn outdated(running: &str, latest: &str) -> bool {
    parts(latest) > parts(running)
}

fn parts(version: &str) -> Vec<u64> {
    version
        .split('-')
        .next()
        .unwrap_or(version)
        .split('.')
        .map(|part| part.parse().unwrap_or(0))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_compare_as_numbers_not_as_text() {
        assert!(outdated("2.9.0", "2.10.0"));
        assert!(!outdated("2.10.0", "2.9.0"));
        assert!(!outdated("2.4.0", "2.4.0"));
    }

    /// A prerelease is not a newer version: `2.5.0-rc.1` must not push someone running `2.5.0`
    /// into "updating" to something they are already ahead of.
    #[test]
    fn a_prerelease_suffix_is_ignored() {
        assert!(!outdated("2.5.0", "2.5.0-rc.1"));
    }

    const NOW: Duration = Duration::from_secs(1_000_000);

    #[test]
    fn a_fresh_cache_answers() {
        let written = NOW.as_secs() - 60;

        assert_eq!(
            parse_cache(&format!("{written} 2.4.0"), NOW).as_deref(),
            Some("2.4.0")
        );
    }

    /// Past a day, we ask again — without which a published version would stay invisible.
    #[test]
    fn a_stale_cache_asks_again() {
        let written = NOW.as_secs() - FRESH_FOR.as_secs() - 1;

        assert!(parse_cache(&format!("{written} 2.4.0"), NOW).is_none());
    }

    /// A damaged cache reads as no answer, never as a version: a truncated file would otherwise
    /// have anything at all announced as the latest published version.
    #[test]
    fn a_damaged_cache_reads_as_no_answer() {
        for raw in ["", "   ", "n importe quoi", "pas-un-nombre 2.4.0", "1000 "] {
            assert!(parse_cache(raw, NOW).is_none(), "accepté : {raw:?}");
        }
    }

    /// A clock that goes backwards — an NTP correction, a machine waking up — writes a
    /// timestamp in the future. We ask again then, rather than trusting a freshness we have no
    /// way of computing: asking again costs one request, while relying on a cache we do not
    /// understand could silence the notice for a very long time.
    #[test]
    fn a_cache_written_in_the_future_is_asked_again() {
        let written = NOW.as_secs() + 10;

        assert!(parse_cache(&format!("{written} 2.4.0"), NOW).is_none());
    }
}
