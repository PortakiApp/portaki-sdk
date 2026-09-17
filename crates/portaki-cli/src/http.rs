//! Le client HTTP de la CLI — jamais sans échéance.
//!
//! `reqwest::Client::new()` n'en pose aucune : une plateforme injoignable ne rendait pas la main,
//! et `portaki login` tournait indéfiniment sur son spinner « asking the platform for a code »
//! sans jamais échouer. Tout client construit ici porte donc deux échéances — une pour ouvrir la
//! connexion, une pour la requête entière — et les erreurs de transport se lisent en une phrase
//! qui nomme l'URL réellement appelée, parce qu'un `PORTAKI_API_URL` de travers ne se voit
//! autrement nulle part.
//!
//! Deux clients seulement, pour que le choix reste lisible : [`client`] pour les appels JSON
//! ordinaires, [`patient_client`] pour ceux qui transportent un artefact ou attendent un travail
//! côté plateforme. Un appel qui a son propre rythme garde le droit de poser son échéance sur la
//! requête (`RequestBuilder::timeout`), ce qui remplace celle du client sans toucher à celle de
//! la connexion.

use std::time::Duration;

/// Le temps laissé à l'ouverture de la connexion (DNS, TCP, TLS).
///
/// Court exprès : passé ce délai, l'hôte n'existe pas, le port est fermé, ou le réseau avale les
/// paquets. Aucune de ces trois réponses ne s'améliore en attendant.
pub const CONNECT: Duration = Duration::from_secs(5);

/// Le temps laissé à une requête ordinaire, ouverture de connexion comprise.
pub const REQUEST: Duration = Duration::from_secs(15);

/// Le temps laissé à un transfert d'artefact ou à un travail exécuté côté plateforme.
///
/// Un `dev-deploy` pousse un `.wasm` de plusieurs mégaoctets depuis une connexion quelconque, et
/// un `dispatch` fait tourner une opération avant de répondre : les couper à quinze secondes
/// casserait le travail normal. L'échéance existe quand même — sans elle, une connexion qui meurt
/// en silence bloque la CLI pour toujours.
pub const TRANSFER: Duration = Duration::from_secs(300);

/// Un client aux échéances par défaut : à utiliser pour tout appel JSON.
pub fn client() -> reqwest::Client {
    client_with(CONNECT, REQUEST)
}

/// Un client pour les transferts et les appels longs.
pub fn patient_client() -> reqwest::Client {
    client_with(CONNECT, TRANSFER)
}

/// Un client dont on choisit les deux échéances.
pub fn client_with(connect: Duration, request: Duration) -> reqwest::Client {
    reqwest::Client::builder()
        .connect_timeout(connect)
        .timeout(request)
        .build()
        // `build` n'échoue que si le backend TLS ne s'initialise pas, auquel cas `Client::new()`
        // échouerait de la même façon — mais en paniquant avec le message que reqwest donne
        // partout ailleurs, plutôt qu'avec une erreur inventée ici.
        .unwrap_or_else(|_| reqwest::Client::new())
}

/// Pourquoi une requête n'a pas abouti — avant même qu'il y ait un statut à lire.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reach {
    /// La connexion ne s'est pas ouverte : hôte inconnu, port fermé, paquets perdus.
    Connect,
    /// Elle s'est ouverte, mais la réponse n'est pas venue à temps.
    Timeout,
    /// Tout le reste : TLS refusé, redirection en boucle, corps interrompu.
    Transport,
}

/// Lit une erreur reqwest. Volontairement mince : c'est la phrase rendue qui se teste.
///
/// `is_connect` d'abord, parce qu'une échéance de connexion porte les deux marques à la fois et
/// que « la connexion ne s'est pas ouverte » reste vrai dans ce cas, alors que « la plateforme
/// n'a pas répondu » laisserait croire qu'on lui a parlé.
pub fn reach(failure: &reqwest::Error) -> Reach {
    if failure.is_connect() {
        Reach::Connect
    } else if failure.is_timeout() {
        Reach::Timeout
    } else {
        Reach::Transport
    }
}

/// La phrase montrée à l'utilisateur quand la requête n'a pas abouti.
///
/// L'URL y figure entière : c'est le seul endroit où un `PORTAKI_API_URL` mal réglé se voit.
pub fn describe(url: &str, reach: Reach) -> String {
    match reach {
        Reach::Connect => {
            format!("cannot reach the platform at {url} — no connection could be opened")
        }
        Reach::Timeout => format!("cannot reach the platform at {url} — it did not answer in time"),
        Reach::Transport => format!("cannot reach the platform at {url} — the connection failed"),
    }
}

/// L'erreur à remonter quand une requête n'aboutit pas.
pub fn unreachable(url: &str, failure: reqwest::Error) -> anyhow::Error {
    let sentence = describe(url, reach(&failure));
    anyhow::Error::new(failure).context(sentence)
}

/// Ce qu'on dit d'une réponse qui a bien un statut, mais pas celui espéré.
///
/// Distinct de [`describe`] : ici la plateforme a répondu. Le statut et l'URL ensemble suffisent
/// à séparer « la route n'existe pas sur cet hôte » d'un refus métier.
pub fn refused(url: &str, status: u16, body: &str) -> String {
    match crate::api::error_code(body) {
        Some(code) => format!("the platform answered {status} ({code}) at {url}"),
        None => format!("the platform answered {status} at {url}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Les trois échéances sont bornées : aucune ne doit pouvoir devenir « pas d'échéance ».
    #[test]
    fn every_deadline_is_finite_and_ordered() {
        assert!(CONNECT < REQUEST);
        assert!(REQUEST < TRANSFER);
        assert!(!CONNECT.is_zero());
    }

    /// Le défaut corrigé : une plateforme injoignable doit se nommer, et nommer l'URL essayée.
    #[test]
    fn an_unreachable_platform_names_the_url_that_was_tried() {
        let url = "https://api-staging.portaki.app/api/v1/auth/device/code";

        for reach in [Reach::Connect, Reach::Timeout, Reach::Transport] {
            let sentence = describe(url, reach);
            assert!(sentence.contains(url), "{sentence}");
            assert!(sentence.contains("cannot reach the platform"), "{sentence}");
        }
    }

    /// Les trois causes ne se disent pas de la même façon : sinon autant n'en garder qu'une.
    #[test]
    fn the_three_causes_read_differently() {
        let url = "https://example.test";

        assert_ne!(describe(url, Reach::Connect), describe(url, Reach::Timeout));
        assert_ne!(
            describe(url, Reach::Timeout),
            describe(url, Reach::Transport)
        );
    }

    /// Une réponse reçue n'est pas une plateforme injoignable — la confondre envoyait chercher
    /// un problème de réseau alors que la route avait répondu.
    #[test]
    fn a_status_is_never_reported_as_unreachable() {
        let sentence = refused("https://example.test/api/v1/auth/device/code", 404, "");

        assert!(sentence.contains("404"), "{sentence}");
        assert!(sentence.contains("https://example.test"), "{sentence}");
        assert!(!sentence.contains("cannot reach"), "{sentence}");
    }

    /// Quand l'enveloppe porte un code, il apparaît : c'est lui qui dit ce qui a été refusé.
    #[test]
    fn a_refusal_carries_its_error_code_when_there_is_one() {
        let body = r#"{"success":false,"error_code":"unsupported_grant_type"}"#;

        assert!(refused("https://example.test/x", 400, body).contains("unsupported_grant_type"));
    }
}
