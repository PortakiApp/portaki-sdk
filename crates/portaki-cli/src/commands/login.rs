//! `portaki login` — device grant, RFC 8628.
//!
//! The CLI has no browser, so it asks for a code, the developer approves it from a session that
//! is already signed in, and the CLI polls until the answer comes. The model is `gh auth login`;
//! improvised by hand this flow is shaky, so it follows the spec.

use std::time::Duration;

use anyhow::{bail, Context, Result};
use clap::Parser;

use crate::{auth, http, ui};

const CLIENT_ID: &str = "portaki-cli";

/// How many **consecutive** transport failures polling tolerates before giving up.
///
/// The policy comes down to two opposed sentences. A hiccup — wifi switching over, a proxy
/// recycling a connection, a 502 for the length of a deployment — must not cancel a sign-in that
/// the person may well be approving in their browser right now: we retry at the same interval,
/// without saying anything. But a platform that has become unreachable must not keep the wheel
/// spinning until `expires_in`: past this many failures in a row, we stop and we say why.
///
/// Consecutive, then: a poll that goes through resets the counter to zero. It is a run that is
/// counted, not a total — over a quarter of an hour of waiting, isolated hiccups are normal.
const BLIPS_TOLERATED: u32 = 3;

/// What the CLI may ask for. Narrowed server-side to what this client is allowed.
///
/// One entry per thing the CLI actually does, so the approval screen can show them one by one:
/// asking for a single coarse scope would put "grant developer access" in front of the person
/// deciding, which is not something anyone can weigh. Nothing here touches host data — no
/// `host:` scope is grantable to this client, and the server would strip one anyway.
const SCOPES: [&str; 5] = [
    "dev:read",
    "dev:deploy",
    "dev:dispatch",
    "dev:stay:read",
    "registry:publish",
];

/// This binary's version, and the SDK it was built against.
///
/// Sent with the device code request so the approval screen can show which build is asking.
/// The two travel separately because they can differ: a machine may run an old `portaki`
/// against a freshly published SDK, and the person approving should see both numbers.
const CLIENT_VERSION: &str = env!("CARGO_PKG_VERSION");
const SDK_VERSION: &str = portaki_sdk::VERSION;

#[derive(Debug, Parser)]
/// Arguments for `portaki login`.
pub struct LoginArgs {
    /// Alias of the global --api, kept for older scripts.
    #[arg(long, hide = true)]
    pub url: Option<String>,

    /// Print the URL instead of opening it — for a remote shell or a headless box.
    #[arg(long)]
    pub no_browser: bool,
}

#[derive(Debug, Parser)]
/// Arguments for `portaki logout`.
/// No `--url`: `--api` / `--env` name the platform, and only that platform's session is ended —
/// its refresh token is sent nowhere else.
pub struct LogoutArgs {}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeviceCode {
    device_code: String,
    user_code: String,
    verification_uri: String,
    /// The URL with the code already in it (RFC 8628 §3.3.1). The server is not required to
    /// return one, and building it ourselves would mean guessing the shape of a parameter: when
    /// it is there, there is nothing left to copy out; when it is missing, we open the bare URL
    /// and the code is displayed.
    #[serde(default)]
    verification_uri_complete: Option<String>,
    expires_in: u64,
    interval: u64,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct Granted {
    access_token: String,
    refresh_token: String,
    #[serde(default)]
    scopes: Vec<String>,
}

/// Runs `portaki login`.
pub async fn run(args: LoginArgs) -> Result<()> {
    ui::header(
        "portaki login",
        &crate::tr!(
            "Device grant — the code below ties this terminal to your Portaki account.",
            "Connecter la CLI — le code ci-dessous relie ce terminal à votre compte Portaki."
        ),
    );

    let base = base_url(args.url.as_deref());
    // The token will come back over this connection: in clear, any network crossed reads it.
    auth::ensure_transport(&base)?;
    // The default client: five seconds to open the connection, fifteen for the request. The code
    // request comes before everything else, so it fails fast — a wrong address or a platform
    // that is down shows up straight away, rather than at the end of an endless spinner.
    let client = http::client();
    let code_url = format!("{base}/api/v1/auth/device/code");

    let asking = ui::step("asking the platform for a code");
    // What this machine says about itself. None of it proves anything — a hostile client would
    // lie — but the approval screen has nothing else to show, and a developer recognises their
    // own machine name at a glance. Absent fields simply render as unknown.
    let sent = client
        .post(&code_url)
        .json(&serde_json::json!({
            "clientId": CLIENT_ID,
            "scopes": SCOPES,
            "deviceLabel": device_label(),
            "clientVersion": CLIENT_VERSION,
            "sdkVersion": SDK_VERSION,
        }))
        .send()
        .await;

    // Every exit from here puts the spinner out before returning: a wheel that keeps turning
    // under an error message makes it look as if the CLI were still working.
    let response = match sent {
        Ok(response) => response,
        Err(failure) => {
            asking.abandon();
            // No `context` on top of it: "cannot reach the platform at …" is the sentence that
            // has to come first, not as a "caused by" under a task heading.
            return Err(http::unreachable(&code_url, failure));
        }
    };
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    if !status.is_success() {
        asking.abandon();
        bail!("{}", http::refused(&code_url, status.as_u16(), &body));
    }
    let started: DeviceCode = crate::api::unwrap(&body).map_err(|failure| {
        asking.abandon();
        failure
    })?;
    asking.done("got a code");

    present(&started, args.no_browser);

    // The server dictates the interval: the spec wants it to be able to slow down a client that
    // is in too much of a hurry.
    let mut interval = Duration::from_secs(started.interval.max(1));
    let deadline = std::time::Instant::now() + Duration::from_secs(started.expires_in);
    let token_url = format!("{base}/api/v1/auth/device/token");
    let waiting = ui::step("waiting for approval");
    let mut blips: u32 = 0;

    loop {
        let now = std::time::Instant::now();
        if now >= deadline {
            waiting.abandon();
            bail!("the code expired before it was approved — run `portaki login` again");
        }
        waiting.say(format!(
            "waiting for approval — {} left",
            ui::countdown(deadline - now)
        ));
        tokio::time::sleep(interval).await;

        let sent = client
            .post(&token_url)
            .json(&serde_json::json!({ "deviceCode": started.device_code }))
            // The poll's deadline follows the interval, not the client's: a stuck poll lasting
            // fifteen seconds every round would eat up the code's lifetime without ever asking
            // the question.
            .timeout(poll_timeout(interval))
            .send()
            .await;

        let response = match sent {
            Ok(response) => response,
            Err(failure) => {
                blips += 1;
                if give_up_after(blips) {
                    waiting.abandon();
                    return Err(http::unreachable(&token_url, failure)).context(format!(
                        "the platform stopped answering — {blips} polls in a row failed"
                    ));
                }
                continue;
            }
        };

        let status = response.status();
        let body = response.text().await.unwrap_or_default();

        if status.is_success() {
            let granted: Granted = crate::api::unwrap(&body).map_err(|failure| {
                waiting.abandon();
                failure
            })?;
            auth::store_issued_by(&base, &granted.access_token, &granted.refresh_token)?;
            waiting.done("approved");
            ui::success(format!(
                "signed in — session stored in {}",
                auth::storage_label()
            ));
            if !granted.scopes.is_empty() {
                ui::field("scopes", granted.scopes.join(" "));
            }
            ui::advice(
                "the access token lasts minutes and renews itself — the session is kept until \
                 portaki logout, and only ever sent back to this platform",
            );
            ui::next(&[
                (
                    "portaki dev --watch",
                    &crate::tr!(
                        "build, deploy to the sandbox, redeploy on every save",
                        "déployer en sandbox, redéployer à chaque sauvegarde"
                    ),
                ),
                (
                    "portaki release",
                    &crate::tr!(
                        "test, build, sign and announce a version",
                        "la porte de check, puis pousser, signer et annoncer une version"
                    ),
                ),
            ]);
            ui::blank();
            return Ok(());
        }

        match interpret(status.as_u16(), &body, &token_url) {
            // Neither one of these is a failure: "not yet" and "less quickly".
            Pending::KeepWaiting => blips = 0,
            Pending::SlowDown => {
                blips = 0;
                interval += Duration::from_secs(5);
            }
            // The platform answered, but with nothing usable: same policy as for a transport
            // hiccup, and the same counter — it is the run that decides.
            Pending::Blip => {
                blips += 1;
                if give_up_after(blips) {
                    waiting.abandon();
                    bail!(
                        "{} — {blips} polls in a row failed",
                        http::refused(&token_url, status.as_u16(), &body)
                    );
                }
            }
            Pending::GiveUp(reason) => {
                waiting.abandon();
                bail!(reason);
            }
        }
    }
}

/// Should we give up, after `consecutive` polls in a row that gave nothing?
///
/// See [`BLIPS_TOLERATED`] for the reasoning behind the policy.
fn give_up_after(consecutive: u32) -> bool {
    consecutive > BLIPS_TOLERATED
}

/// How long a poll is allowed to last.
///
/// Long enough not to cut off a slow answer, never much more than one interval's worth: beyond
/// that, a stuck poll shifts every following one and the wheel turns without the question ever
/// being asked. A floor, because an interval of one second would leave no time for a TLS
/// handshake; a ceiling, because a repeated `slow_down` makes the interval climb without end.
fn poll_timeout(interval: Duration) -> Duration {
    (interval + Duration::from_secs(5)).clamp(Duration::from_secs(8), Duration::from_secs(20))
}

/// What a poll that returned no token is saying.
#[derive(Debug, PartialEq, Eq)]
enum Pending {
    /// Not approved yet: we come back at the same rate.
    KeepWaiting,
    /// The server is asking us to slow down (RFC 8628 §3.5).
    SlowDown,
    /// Nothing usable, but nothing final either: to be retried, within the tolerated limit.
    Blip,
    /// Over, and here is what to say.
    GiveUp(String),
}

/// Reads a polling response.
///
/// The spec's codes arrive inside the `error_code` of our own envelope. Read flat, they looked
/// like an unknown answer and the CLI gave up from the very first poll.
///
/// Three families, and they are not worded alike: an OAuth code is an answer from the protocol,
/// a status without a code is a failure of the route (a 404 here means "this is not a Portaki
/// platform"), and a transport error never even reaches this far.
fn interpret(status: u16, body: &str, url: &str) -> Pending {
    match crate::api::error_code(body).as_deref() {
        Some("authorization_pending") => Pending::KeepWaiting,
        Some("slow_down") => Pending::SlowDown,
        Some("access_denied") => Pending::GiveUp("the request was denied".to_owned()),
        Some("expired_token") => {
            Pending::GiveUp("the code expired — run `portaki login` again".to_owned())
        }
        Some(other) => Pending::GiveUp(format!("the platform answered {other}")),
        // 5xx with no OAuth code: the platform is stammering, it is not refusing. A restart
        // behind a load balancer must not cancel an approval in progress.
        None if status >= 500 => Pending::Blip,
        None => Pending::GiveUp(http::refused(url, status, body)),
    }
}

/// Shows the code, then takes the user to where they approve it.
///
/// The browser opens on the pre-filled URL when the server gives one: all that is left then is
/// to confirm. The code stays on screen whatever happens — it is the only fallback if opening
/// fails, if the CLI is running inside an SSH session, or if the browser that opened is not the
/// one where the session is already open.
fn present(started: &DeviceCode, no_browser: bool) {
    let target = started
        .verification_uri_complete
        .as_deref()
        .unwrap_or(&started.verification_uri);

    ui::blank();
    ui::code_block(&started.user_code);
    ui::blank();

    if no_browser || !ui::open_browser(target) {
        ui::field("open", target);
        ui::field("code", &started.user_code);
    } else {
        ui::success(if started.verification_uri_complete.is_some() {
            "opened your browser — check the code above, then approve"
        } else {
            "opened your browser — paste the code above to approve"
        });
        ui::detail(target);
    }
    ui::blank();
}

/// Runs `portaki logout`.
pub async fn logout(_args: LogoutArgs) -> Result<()> {
    ui::header(
        "portaki logout",
        &crate::tr!(
            "End the session here, and on the platform.",
            "Fermer la session ici, et sur la plateforme."
        ),
    );

    let issuer = crate::profile::api_url(None);
    let origin = auth::origin_of(&issuer).with_context(|| format!("{issuer} is not a URL"))?;
    let stored = auth::refresh_token(&origin);

    // Erased first, whatever happens next: a sign-out that leaves the credentials in place
    // because the network hiccuped would be the worse of the two halves — you believe you are
    // out, and you are out nowhere.
    auth::forget(&origin)?;

    let Some(refresh_token) = stored else {
        ui::skipped(format!("no session was stored for {origin}"));
        let elsewhere: Vec<String> = auth::signed_in_origins()
            .into_iter()
            .filter(|other| *other != origin)
            .collect();
        if !elsewhere.is_empty() {
            ui::detail(format!(
                "signed in to {} — add --api <url> or --env <name>",
                elsewhere.join(", ")
            ));
        }
        crate::exit::nothing_to_do();
        ui::blank();
        return Ok(());
    };
    ui::success(format!("signed out of {origin} here — credentials cleared"));

    match revoke(&issuer, &refresh_token).await {
        Ok(()) => ui::success("the platform revoked this session"),
        Err(failure) => {
            // Say it, because this is the half that protects: a token that was not revoked
            // stays usable by whoever holds a copy of the file.
            ui::warn("could not reach the platform — this session is still valid there");
            ui::detail(format!("{failure:#}"));
            ui::advice("run portaki logout again once you are online");
        }
    }
    ui::blank();
    Ok(())
}

/// Tells the platform to forget this token.
///
/// Without it, `portaki logout` only erases a file: the refresh token stays valid until it
/// expires, and whoever holds a copy of the file stays signed in.
async fn revoke(base: &str, refresh_token: &str) -> Result<()> {
    let url = format!("{base}/api/v1/auth/logout");
    // Ten seconds in all, but now five to open the connection: with no connection deadline, an
    // unreachable platform held `portaki logout` back until the kernel's timeout.
    let response = http::client()
        .post(&url)
        .json(&serde_json::json!({ "refreshToken": refresh_token }))
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map_err(|failure| http::unreachable(&url, failure))?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        anyhow::bail!("{}", http::refused(&url, status.as_u16(), &body));
    }
    Ok(())
}

fn base_url(explicit: Option<&str>) -> String {
    crate::profile::api_url(explicit)
}

/// This machine's name, the way its owner would recognise it.
///
/// No dependency for it: `COMPUTERNAME` on Windows, the `hostname` binary everywhere else, then
/// the environment as a last resort — zsh exports `HOST`, bash exports `HOSTNAME`, and neither
/// is guaranteed. Returning `None` is a normal outcome, not a failure: the approval screen shows
/// one fewer line and login proceeds.
fn device_label() -> Option<String> {
    if let Ok(name) = std::env::var("COMPUTERNAME") {
        if let Some(name) = non_empty(name) {
            return Some(name);
        }
    }
    if let Ok(output) = std::process::Command::new("hostname").output() {
        if output.status.success() {
            if let Some(name) = non_empty(String::from_utf8_lossy(&output.stdout).into_owned()) {
                return Some(name);
            }
        }
    }
    std::env::var("HOSTNAME")
        .or_else(|_| std::env::var("HOST"))
        .ok()
        .and_then(non_empty)
}

fn non_empty(value: String) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_cli_never_asks_for_a_host_scope() {
        // A CLI token does not perform host operations; the server would strip it anyway, but
        // asking for it would already be one intention too many. Written against the prefix and
        // not against a named scope: a `host:billing` added tomorrow must fail here too.
        assert!(!SCOPES.iter().any(|s| s.starts_with("host:")));
    }

    /// Sandbox stays and a real traveller's stays do not carry the same scope.
    #[test]
    fn sandbox_stays_are_asked_for_under_the_dev_domain() {
        assert!(SCOPES.contains(&"dev:stay:read"));
        assert!(!SCOPES.contains(&"stay:read"));
    }

    /// A hostname reaches an approval screen, so it must never arrive as a raw command output —
    /// `hostname` ends its line with a newline, and a trailing one would render as a blank row.
    #[test]
    fn a_device_label_is_trimmed_or_absent() {
        assert_eq!(
            non_empty("  my-laptop.local\n".to_owned()).as_deref(),
            Some("my-laptop.local")
        );
        assert_eq!(non_empty("   ".to_owned()), None);
        assert_eq!(non_empty(String::new()), None);
    }

    /// Nothing about login depends on knowing the machine name; it only makes the screen poorer.
    #[test]
    fn a_missing_device_label_is_not_an_error() {
        let label = device_label();

        assert!(label
            .as_deref()
            .map(str::trim)
            .map(|l| !l.is_empty())
            .unwrap_or(true));
    }

    #[test]
    fn the_versions_announced_are_the_ones_compiled_in() {
        assert!(!CLIENT_VERSION.is_empty());
        assert_eq!(SDK_VERSION, portaki_sdk::VERSION);
    }

    #[test]
    fn an_explicit_url_wins_over_the_environment() {
        std::env::set_var("PORTAKI_API_URL", "https://from-env.example");

        assert_eq!(
            base_url(Some("https://explicit.example/")),
            "https://explicit.example"
        );

        std::env::remove_var("PORTAKI_API_URL");
    }

    #[test]
    fn a_trailing_slash_never_doubles_in_the_path() {
        assert_eq!(
            base_url(Some("https://api.example/")),
            "https://api.example"
        );
    }

    const TOKEN_URL: &str = "https://api-staging.portaki.app/api/v1/auth/device/token";

    /// An isolated hiccup must not cancel a sign-in that is being approved right now.
    #[test]
    fn a_blip_does_not_end_the_login() {
        for consecutive in 1..=BLIPS_TOLERATED {
            assert!(!give_up_after(consecutive), "gave up after {consecutive}");
        }
    }

    /// But a platform gone silent must not keep the wheel spinning until the code expires.
    #[test]
    fn a_run_of_failures_ends_the_login() {
        assert!(give_up_after(BLIPS_TOLERATED + 1));
        assert!(give_up_after(BLIPS_TOLERATED + 9));
    }

    /// It is the run that is counted, not the total: what resetting the counter must guarantee.
    #[test]
    fn the_counter_is_a_run_and_not_a_total() {
        let mut blips = 0_u32;

        // Two hiccups, a poll that goes through, two hiccups: five failures in all, never four
        // in a row — the sign-in carries on.
        for outcome in [false, false, true, false, false] {
            if outcome {
                blips = 0;
            } else {
                blips += 1;
            }
            assert!(!give_up_after(blips));
        }
    }

    /// A poll must not last longer than what separates two polls, or barely longer: otherwise
    /// they pile up and the code expires without the question having been asked.
    #[test]
    fn a_poll_never_outlives_the_code_it_asks_about() {
        let expires_in = Duration::from_secs(600);
        let interval = Duration::from_secs(5);

        assert!(poll_timeout(interval) < expires_in / 10);
        // An interval of one second still keeps enough room for a TLS handshake.
        assert!(poll_timeout(Duration::from_secs(1)) >= Duration::from_secs(8));
        // And a repeated `slow_down` does not make the deadline climb without end.
        assert_eq!(
            poll_timeout(Duration::from_secs(600)),
            Duration::from_secs(20)
        );
    }

    /// The two answers that mean "come back later".
    #[test]
    fn pending_and_slow_down_are_not_failures() {
        assert_eq!(
            interpret(400, r#"{"error_code":"authorization_pending"}"#, TOKEN_URL),
            Pending::KeepWaiting
        );
        assert_eq!(
            interpret(429, r#"{"error_code":"slow_down"}"#, TOKEN_URL),
            Pending::SlowDown
        );
    }

    /// A denial and an expired code are final: retrying them would spin the wheel for nothing.
    #[test]
    fn a_denial_stops_the_login_at_once() {
        let denied = interpret(403, r#"{"error_code":"access_denied"}"#, TOKEN_URL);
        let expired = interpret(400, r#"{"error_code":"expired_token"}"#, TOKEN_URL);

        assert!(matches!(denied, Pending::GiveUp(said) if said.contains("denied")));
        assert!(matches!(expired, Pending::GiveUp(said) if said.contains("expired")));
    }

    /// A route failure is not an OAuth code: it is reported with its status and its URL, so that
    /// a `PORTAKI_API_URL` that does not point at a Portaki platform shows up.
    #[test]
    fn a_status_without_an_oauth_code_names_the_url_that_was_polled() {
        let said = match interpret(404, "<html>not found</html>", TOKEN_URL) {
            Pending::GiveUp(said) => said,
            other => panic!("{other:?}"),
        };

        assert!(said.contains("404"), "{said}");
        assert!(said.contains(TOKEN_URL), "{said}");
    }

    /// A 502 for the length of a restart is not a refusal: the person may be standing in front
    /// of the approval screen, and giving up there would make them start all over again.
    #[test]
    fn a_platform_hiccup_is_retried_rather_than_fatal() {
        assert_eq!(
            interpret(502, "<html>bad gateway</html>", TOKEN_URL),
            Pending::Blip
        );
        assert_eq!(interpret(503, "", TOKEN_URL), Pending::Blip);
    }

    /// A code the spec does not know is still an ending: we have nothing better to do with it,
    /// and saying so is better than polling a platform that always gives the same answer.
    #[test]
    fn an_unknown_oauth_code_is_reported_verbatim() {
        let said = match interpret(400, r#"{"error_code":"invalid_client"}"#, TOKEN_URL) {
            Pending::GiveUp(said) => said,
            other => panic!("{other:?}"),
        };

        assert!(said.contains("invalid_client"), "{said}");
    }
}
