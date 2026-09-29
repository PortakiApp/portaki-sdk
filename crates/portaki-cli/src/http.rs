//! The CLI's HTTP client — never without a deadline.
//!
//! `reqwest::Client::new()` sets none: an unreachable platform never gave control back, and
//! `portaki login` spun forever on its "asking the platform for a code" spinner without ever
//! failing. Every client built here therefore carries two deadlines — one to open the connection,
//! one for the whole request — and transport errors read as a single sentence that names the URL
//! actually called, because a wrong `PORTAKI_API_URL` shows up nowhere else.
//!
//! Two clients only, so the choice stays readable: [`client`] for ordinary JSON calls,
//! [`patient_client`] for the ones that carry an artifact or wait on work happening on the
//! platform side. A call with its own rhythm keeps the right to set its deadline on the request
//! (`RequestBuilder::timeout`), which replaces the client's without touching the connection's.

use std::time::Duration;

/// The time allowed for opening the connection (DNS, TCP, TLS).
///
/// Short on purpose: past this delay, the host does not exist, the port is closed, or the network
/// is swallowing the packets. None of those three answers improves with waiting.
pub const CONNECT: Duration = Duration::from_secs(5);

/// The time allowed for an ordinary request, opening the connection included.
pub const REQUEST: Duration = Duration::from_secs(15);

/// The time allowed for an artifact transfer or for work run on the platform side.
///
/// A `dev-deploy` pushes a `.wasm` of several megabytes over whatever connection is at hand, and
/// a `dispatch` runs an operation before answering: cutting them off at fifteen seconds would
/// break normal work. The deadline exists all the same — without it, a connection that dies in
/// silence blocks the CLI forever.
pub const TRANSFER: Duration = Duration::from_secs(300);

/// A client with the default deadlines: to be used for every JSON call.
pub fn client() -> reqwest::Client {
    client_with(CONNECT, REQUEST)
}

/// A client for transfers and long calls.
pub fn patient_client() -> reqwest::Client {
    client_with(CONNECT, TRANSFER)
}

/// A client whose two deadlines are chosen by the caller.
pub fn client_with(connect: Duration, request: Duration) -> reqwest::Client {
    reqwest::Client::builder()
        .connect_timeout(connect)
        .timeout(request)
        .build()
        // `build` only fails if the TLS backend does not initialise, in which case
        // `Client::new()` would fail the same way — but panicking with the message reqwest gives
        // everywhere else, rather than with an error invented here.
        .unwrap_or_else(|_| reqwest::Client::new())
}

/// Why a request did not go through — before there is even a status to read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reach {
    /// The connection never opened: unknown host, closed port, lost packets.
    Connect,
    /// It did open, but the answer never came in time.
    Timeout,
    /// Everything else: TLS refused, redirect loop, interrupted body.
    Transport,
}

/// Reads a reqwest error. Deliberately thin: it is the resulting sentence that gets tested.
///
/// `is_connect` first, because a connection deadline carries both marks at once and "the
/// connection never opened" stays true in that case, whereas "the platform did not answer" would
/// suggest we had talked to it.
pub fn reach(failure: &reqwest::Error) -> Reach {
    if failure.is_connect() {
        Reach::Connect
    } else if failure.is_timeout() {
        Reach::Timeout
    } else {
        Reach::Transport
    }
}

/// The sentence shown to the user when the request did not go through.
///
/// The URL appears in full: it is the only place where a badly set `PORTAKI_API_URL` shows up.
pub fn describe(url: &str, reach: Reach) -> String {
    match reach {
        Reach::Connect => crate::tr!(
            "cannot reach the platform at {url} — no connection could be opened",
            "plateforme injoignable à {url} — aucune connexion n'a pu s'ouvrir"
        ),
        Reach::Timeout => crate::tr!(
            "cannot reach the platform at {url} — it did not answer in time",
            "plateforme injoignable à {url} — elle n'a pas répondu à temps"
        ),
        Reach::Transport => crate::tr!(
            "cannot reach the platform at {url} — the connection failed",
            "plateforme injoignable à {url} — la connexion a échoué"
        ),
    }
}

/// The error to surface when a request does not go through.
pub fn unreachable(url: &str, failure: reqwest::Error) -> anyhow::Error {
    let sentence = describe(url, reach(&failure));
    anyhow::Error::new(failure).context(sentence)
}

/// What we say about a response that does have a status, just not the one hoped for.
///
/// Distinct from [`describe`]: here the platform answered. The status and the URL together are
/// enough to tell "the route does not exist on this host" apart from a business refusal.
pub fn refused(url: &str, status: u16, body: &str) -> String {
    match crate::api::error_code(body) {
        Some(code) => crate::tr!(
            "the platform answered {status} ({code}) at {url}",
            "la plateforme a répondu {status} ({code}) à {url}"
        ),
        None => crate::tr!(
            "the platform answered {status} at {url}",
            "la plateforme a répondu {status} à {url}"
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The three deadlines are bounded: none must ever be able to become "no deadline".
    #[test]
    fn every_deadline_is_finite_and_ordered() {
        assert!(CONNECT < REQUEST);
        assert!(REQUEST < TRANSFER);
        assert!(!CONNECT.is_zero());
    }

    /// The defect this fixes: an unreachable platform must say so, and name the URL it tried.
    #[test]
    fn an_unreachable_platform_names_the_url_that_was_tried() {
        let url = "https://api-staging.portaki.app/api/v1/auth/device/code";

        for reach in [Reach::Connect, Reach::Timeout, Reach::Transport] {
            let sentence = describe(url, reach);
            assert!(sentence.contains(url), "{sentence}");
            assert!(sentence.contains("cannot reach the platform"), "{sentence}");
        }
    }

    /// The three causes are not worded the same way: otherwise we may as well keep only one.
    #[test]
    fn the_three_causes_read_differently() {
        let url = "https://example.test";

        assert_ne!(describe(url, Reach::Connect), describe(url, Reach::Timeout));
        assert_ne!(
            describe(url, Reach::Timeout),
            describe(url, Reach::Transport)
        );
    }

    /// A response that was received is not an unreachable platform — confusing the two sent
    /// people looking for a network problem when the route had answered.
    #[test]
    fn a_status_is_never_reported_as_unreachable() {
        let sentence = refused("https://example.test/api/v1/auth/device/code", 404, "");

        assert!(sentence.contains("404"), "{sentence}");
        assert!(sentence.contains("https://example.test"), "{sentence}");
        assert!(!sentence.contains("cannot reach"), "{sentence}");
    }

    /// When the envelope carries a code, it shows: that code is what says what was refused.
    #[test]
    fn a_refusal_carries_its_error_code_when_there_is_one() {
        let body = r#"{"success":false,"error_code":"unsupported_grant_type"}"#;

        assert!(refused("https://example.test/x", 400, body).contains("unsupported_grant_type"));
    }
}
