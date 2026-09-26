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

/// Combien d'échecs de transport **consécutifs** le sondage tolère avant d'abandonner.
///
/// La politique tient en deux phrases opposées. Un hoquet — un wifi qui bascule, un proxy qui
/// recycle une connexion, un 502 le temps d'un déploiement — ne doit pas annuler une connexion
/// que la personne est peut-être en train d'approuver dans son navigateur : on retente au même
/// intervalle, sans rien dire. Mais une plateforme devenue injoignable ne doit pas faire tourner
/// la roulette jusqu'à `expires_in` : au-delà de ce nombre d'échecs d'affilée, on s'arrête et on
/// dit pourquoi.
///
/// Consécutifs, donc : un sondage qui aboutit remet le compteur à zéro. C'est une série qu'on
/// compte, pas un total — sur un quart d'heure d'attente, des hoquets isolés sont normaux.
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
    /// Base URL of the platform. Defaults to PORTAKI_API_URL, then production.
    #[arg(long)]
    pub url: Option<String>,

    /// Print the URL instead of opening it — for a remote shell or a headless box.
    #[arg(long)]
    pub no_browser: bool,
}

#[derive(Debug, Parser)]
/// Arguments for `portaki logout`.
pub struct LogoutArgs {
    /// Ignored, kept for scripts that pass it: the session is revoked on the platform that
    /// issued it, and its refresh token is sent nowhere else.
    #[arg(long, hide = true)]
    pub url: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeviceCode {
    device_code: String,
    user_code: String,
    verification_uri: String,
    /// L'URL avec le code déjà dedans (RFC 8628 §3.3.1). Le serveur n'est pas tenu de la rendre,
    /// et la fabriquer soi-même serait deviner la forme d'un paramètre : quand elle est là, il
    /// n'y a plus rien à recopier ; quand elle manque, on ouvre l'URL nue et le code s'affiche.
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
        "Device grant — the code below ties this terminal to your Portaki account.",
    );

    let base = base_url(args.url.as_deref());
    // Le jeton reviendra par cette connexion : en clair, n'importe quel réseau traversé le lit.
    auth::ensure_transport(&base)?;
    // Le client par défaut : cinq secondes pour ouvrir la connexion, quinze pour la requête.
    // La demande de code précède tout le reste, alors elle échoue vite — une adresse fausse ou
    // une plateforme à terre se voit tout de suite, plutôt qu'au bout d'un spinner sans fin.
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

    // Chaque sortie d'ici éteint le spinner avant de remonter : une roue qui continue de tourner
    // sous un message d'erreur laisse croire que la CLI travaille encore.
    let response = match sent {
        Ok(response) => response,
        Err(failure) => {
            asking.abandon();
            // Pas de `context` par-dessus : « cannot reach the platform at … » est la phrase
            // qui doit arriver en tête, pas en « caused by » sous un intitulé de tâche.
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

    // Le serveur dicte l'intervalle : la spec veut qu'il puisse ralentir un client trop pressé.
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
            // L'échéance du sondage se règle sur l'intervalle, pas sur celle du client : un
            // sondage bloqué qui durerait quinze secondes à chaque tour mangerait la vie du
            // code sans jamais poser la question.
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
                    "build, deploy to the sandbox, redeploy on every save",
                ),
                (
                    "portaki publish",
                    "push a release and announce it to the registry",
                ),
            ]);
            ui::blank();
            return Ok(());
        }

        match interpret(status.as_u16(), &body, &token_url) {
            // Ni l'un ni l'autre n'est un échec : « pas encore » et « moins vite ».
            Pending::KeepWaiting => blips = 0,
            Pending::SlowDown => {
                blips = 0;
                interval += Duration::from_secs(5);
            }
            // La plateforme a répondu, mais rien d'exploitable : même politique que le hoquet
            // de transport, et même compteur — c'est la série qui décide.
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

/// Faut-il abandonner, après `consecutive` sondages d'affilée qui n'ont rien donné ?
///
/// Voir [`BLIPS_TOLERATED`] pour le pourquoi de la politique.
fn give_up_after(consecutive: u32) -> bool {
    consecutive > BLIPS_TOLERATED
}

/// Combien de temps un sondage a le droit de durer.
///
/// Assez pour ne pas couper une réponse lente, jamais beaucoup plus qu'un tour d'intervalle : au
/// delà, un sondage bloqué décale tous les suivants et la roue tourne sans que la question soit
/// posée. Plancher parce qu'un intervalle d'une seconde ne laisserait pas le temps d'une poignée
/// de main TLS ; plafond parce qu'un `slow_down` répété fait grimper l'intervalle sans fin.
fn poll_timeout(interval: Duration) -> Duration {
    (interval + Duration::from_secs(5)).clamp(Duration::from_secs(8), Duration::from_secs(20))
}

/// Ce que dit un sondage qui n'a pas rendu de jeton.
#[derive(Debug, PartialEq, Eq)]
enum Pending {
    /// Pas encore approuvé : on repasse au même rythme.
    KeepWaiting,
    /// Le serveur demande qu'on ralentisse (RFC 8628 §3.5).
    SlowDown,
    /// Rien d'exploitable, mais rien de définitif non plus : à retenter, dans la limite tolérée.
    Blip,
    /// Fini, et voici quoi dire.
    GiveUp(String),
}

/// Lit une réponse de sondage.
///
/// Les codes de la spec arrivent dans le `error_code` de l'enveloppe maison. Lus à plat, ils
/// ressemblaient à une réponse inconnue et la CLI abandonnait dès le premier sondage.
///
/// Trois familles, et elles ne se disent pas pareil : un code OAuth est une réponse du protocole,
/// un statut sans code est une panne de la route (un 404 ici veut dire « ce n'est pas une
/// plateforme Portaki »), et une erreur de transport n'arrive même pas jusqu'ici.
fn interpret(status: u16, body: &str, url: &str) -> Pending {
    match crate::api::error_code(body).as_deref() {
        Some("authorization_pending") => Pending::KeepWaiting,
        Some("slow_down") => Pending::SlowDown,
        Some("access_denied") => Pending::GiveUp("the request was denied".to_owned()),
        Some("expired_token") => {
            Pending::GiveUp("the code expired — run `portaki login` again".to_owned())
        }
        Some(other) => Pending::GiveUp(format!("the platform answered {other}")),
        // 5xx sans code OAuth : la plateforme bafouille, elle ne refuse pas. Un redémarrage
        // derrière un load balancer ne doit pas annuler une approbation en cours.
        None if status >= 500 => Pending::Blip,
        None => Pending::GiveUp(http::refused(url, status, body)),
    }
}

/// Montre le code, puis emmène l'utilisateur là où il l'approuve.
///
/// Le navigateur s'ouvre sur l'URL pré-remplie quand le serveur en donne une : il ne reste alors
/// qu'à confirmer. Le code reste affiché quoi qu'il arrive — c'est le seul recours si l'ouverture
/// échoue, si la CLI tourne dans un SSH, ou si le navigateur ouvert n'est pas celui où la session
/// est déjà ouverte.
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
        "End the session here, and on the platform.",
    );

    let stored = auth::refresh_token();
    let issuer = auth::issuer();

    // Effacé d'abord, quoi qu'il arrive ensuite : une déconnexion qui laisse les identifiants
    // en place parce que le réseau a hoqueté serait la pire des deux moitiés — on croit être
    // sorti, et on ne l'est nulle part.
    auth::forget()?;
    ui::success("signed out here — credentials cleared");

    let Some(refresh_token) = stored else {
        ui::detail("no session was stored");
        ui::blank();
        return Ok(());
    };

    match revoke(&issuer, &refresh_token).await {
        Ok(()) => ui::success("the platform revoked this session"),
        Err(failure) => {
            // Le dire, parce que c'est la moitié qui protège : un jeton non révoqué reste
            // utilisable par qui détient une copie du fichier.
            ui::warn("could not reach the platform — this session is still valid there");
            ui::detail(format!("{failure:#}"));
            ui::advice("run portaki logout again once you are online");
        }
    }
    ui::blank();
    Ok(())
}

/// Dit à la plateforme d'oublier ce jeton.
///
/// Sans quoi `portaki logout` n'efface qu'un fichier : le jeton de rafraîchissement reste
/// valide jusqu'à son expiration, et qui détient une copie du fichier reste connecté.
async fn revoke(base: &str, refresh_token: &str) -> Result<()> {
    let url = format!("{base}/api/v1/auth/logout");
    // Dix secondes en tout, mais désormais cinq pour ouvrir la connexion : sans échéance de
    // connexion, une plateforme injoignable retenait `portaki logout` jusqu'au timeout du noyau.
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
    auth::api_base_url(explicit)
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
        // Un jeton de CLI ne fait pas d'opérations hôte ; le serveur le raboterait de toute
        // façon, mais le demander serait déjà une intention de trop. Écrit sur le préfixe et
        // non sur un scope nommé : `host:billing` ajouté demain doit échouer ici aussi.
        assert!(!SCOPES.iter().any(|s| s.starts_with("host:")));
    }

    /// Les séjours de la sandbox et ceux d'un vrai voyageur ne portent pas le même scope.
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

    /// Un hoquet isolé ne doit pas annuler une connexion qu'on est en train d'approuver.
    #[test]
    fn a_blip_does_not_end_the_login() {
        for consecutive in 1..=BLIPS_TOLERATED {
            assert!(!give_up_after(consecutive), "gave up after {consecutive}");
        }
    }

    /// Mais une plateforme devenue muette ne doit pas faire tourner la roue jusqu'à expiration.
    #[test]
    fn a_run_of_failures_ends_the_login() {
        assert!(give_up_after(BLIPS_TOLERATED + 1));
        assert!(give_up_after(BLIPS_TOLERATED + 9));
    }

    /// La série se compte, pas le total : ce que le compteur remis à zéro doit garantir.
    #[test]
    fn the_counter_is_a_run_and_not_a_total() {
        let mut blips = 0_u32;

        // Deux hoquets, un sondage qui aboutit, deux hoquets : cinq échecs en tout, jamais
        // quatre d'affilée — la connexion continue.
        for outcome in [false, false, true, false, false] {
            if outcome {
                blips = 0;
            } else {
                blips += 1;
            }
            assert!(!give_up_after(blips));
        }
    }

    /// Un sondage ne doit pas durer plus longtemps que ce qui sépare deux sondages, ou presque :
    /// sinon ils s'empilent et le code expire sans qu'on ait posé la question.
    #[test]
    fn a_poll_never_outlives_the_code_it_asks_about() {
        let expires_in = Duration::from_secs(600);
        let interval = Duration::from_secs(5);

        assert!(poll_timeout(interval) < expires_in / 10);
        // Un intervalle d'une seconde garde quand même de quoi faire une poignée de main TLS.
        assert!(poll_timeout(Duration::from_secs(1)) >= Duration::from_secs(8));
        // Et un `slow_down` répété ne fait pas grimper l'échéance sans fin.
        assert_eq!(
            poll_timeout(Duration::from_secs(600)),
            Duration::from_secs(20)
        );
    }

    /// Les deux réponses qui veulent dire « repasse ».
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

    /// Un refus et un code périmé sont définitifs : les retenter ferait tourner la roue pour rien.
    #[test]
    fn a_denial_stops_the_login_at_once() {
        let denied = interpret(403, r#"{"error_code":"access_denied"}"#, TOKEN_URL);
        let expired = interpret(400, r#"{"error_code":"expired_token"}"#, TOKEN_URL);

        assert!(matches!(denied, Pending::GiveUp(said) if said.contains("denied")));
        assert!(matches!(expired, Pending::GiveUp(said) if said.contains("expired")));
    }

    /// Une panne de route n'est pas un code OAuth : elle se dit avec son statut et son URL, de
    /// sorte qu'un `PORTAKI_API_URL` qui ne pointe pas sur une plateforme Portaki se voie.
    #[test]
    fn a_status_without_an_oauth_code_names_the_url_that_was_polled() {
        let said = match interpret(404, "<html>not found</html>", TOKEN_URL) {
            Pending::GiveUp(said) => said,
            other => panic!("{other:?}"),
        };

        assert!(said.contains("404"), "{said}");
        assert!(said.contains(TOKEN_URL), "{said}");
    }

    /// Un 502 le temps d'un redémarrage n'est pas un refus : la personne est peut-être devant
    /// l'écran d'approbation, et abandonner là lui ferait tout recommencer.
    #[test]
    fn a_platform_hiccup_is_retried_rather_than_fatal() {
        assert_eq!(
            interpret(502, "<html>bad gateway</html>", TOKEN_URL),
            Pending::Blip
        );
        assert_eq!(interpret(503, "", TOKEN_URL), Pending::Blip);
    }

    /// Un code inconnu de la spec reste une fin : on ne sait pas quoi en faire de mieux, et le
    /// dire vaut mieux que sonder une plateforme qui répond toujours la même chose.
    #[test]
    fn an_unknown_oauth_code_is_reported_verbatim() {
        let said = match interpret(400, r#"{"error_code":"invalid_client"}"#, TOKEN_URL) {
            Pending::GiveUp(said) => said,
            other => panic!("{other:?}"),
        };

        assert!(said.contains("invalid_client"), "{said}");
    }
}
