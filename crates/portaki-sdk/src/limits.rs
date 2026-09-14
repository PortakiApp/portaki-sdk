//! Plafonds appliqués par la plateforme aux modules — une seule table.
//!
//! # La plateforme d'abord
//!
//! **La plateforme est la seule autorité sur ces limites.** Elle les définit dans
//! `contracts/module-limits.json` du dépôt `PortakiApp/portaki-platform`, et c'est elle qui
//! refuse un appel (runtime et orchestrateur). Le SDK ne définit rien : il **recopie** les
//! valeurs pour échouer tôt, et n'accorde jamais plus que la plateforme.
//!
//! La copie vit dans `contracts/platform/module-limits.json` à la racine du dépôt SDK, mise à
//! jour par `scripts/sync-platform-contracts.sh` (`--check` pour comparer sans écrire). Le test
//! `crates/portaki-sdk/tests/limits_contract.rs` échoue tant qu'une constante ci-dessous diffère
//! de la copie, ou qu'une limite de la copie n'a pas de constante ici.
//!
//! Pour changer une valeur : la plateforme d'abord, puis le script, puis la constante. Changer
//! une constante seule ne change rien en production — seulement ce que les tests laissent
//! passer — et le test de contrat la refuse.
//!
//! # Ce que le SDK en fait
//!
//! Un dépassement vérifié côté SDK remonte en erreur typée dès `cargo test` (via
//! `portaki-test-utils`), alors qu'en production un email refusé par l'orchestrateur est
//! simplement abandonné, sans retour vers le module.
//!
//! Certaines limites comptent au-delà d'une invocation ou d'un module (fenêtres glissantes,
//! tous modules confondus, sortie réseau de la gateway) : le SDK ne peut pas les vérifier. Elles
//! figurent quand même ici, marquées « plateforme seule », pour qu'un auteur de module les trouve
//! au même endroit.

// ── Contenu d'un email (`host::email::send`) ─────────────────────────────────────────────
//
// Comptés en caractères (`char`), pas en octets : un sujet accentué ne doit pas être refusé
// avant un sujet ASCII de même longueur. Chaque locale est vérifiée séparément.

/// Longueur maximale du sujet, en caractères, pour chaque locale.
pub const EMAIL_SUBJECT_MAX_CHARS: usize = 200;

/// Longueur maximale de l'eyebrow (au-dessus du titre), en caractères, pour chaque locale.
pub const EMAIL_EYEBROW_MAX_CHARS: usize = 120;

/// Longueur maximale du titre, en caractères, pour chaque locale.
pub const EMAIL_TITLE_MAX_CHARS: usize = 200;

/// Longueur maximale du corps, en caractères, pour chaque locale.
pub const EMAIL_BODY_MAX_CHARS: usize = 5000;

/// Longueur maximale du libellé du CTA, en caractères, pour chaque locale.
pub const EMAIL_CTA_LABEL_MAX_CHARS: usize = 80;

// ── Par invocation ───────────────────────────────────────────────────────────────────────

/// Nombre maximal d'appels à l'hôte (`portaki_host_dispatch`, toutes ops confondues) par
/// invocation.
///
/// Appliqué par la plateforme ; le SDK recopie la valeur. Le mock de `portaki-test-utils` ne
/// compte pas les appels : un test qui boucle sur `host::kv` ne le verra pas.
pub const HOST_CALLS_PER_INVOCATION: usize = 200;

/// Nombre maximal d'appels `email.send` par invocation.
///
/// Au-delà, la plateforme répond `email_limit_exceeded`
/// ([`crate::host::email::EmailError::LimitExceeded`]). Le SDK ne compte pas lui-même : le
/// compteur vit côté hôte, que le mock de `portaki-test-utils` reproduit.
pub const EMAIL_SENDS_PER_INVOCATION: usize = 5;

/// Nombre maximal d'événements émis vers la gateway par invocation.
///
/// Au-delà, la plateforme répond `event_limit_exceeded`
/// ([`crate::error::PortakiError::EventLimitExceeded`]).
pub const EVENTS_PER_INVOCATION: usize = 20;

/// Taille maximale du payload d'un événement émis, en octets (64 Kio).
///
/// Au-delà, la plateforme répond `event_payload_too_large` ; l'événement refusé ne compte pas
/// dans [`EVENTS_PER_INVOCATION`]. Appliqué par la plateforme, reproduit par le mock de
/// `portaki-test-utils` sur le JSON du payload.
pub const EVENT_PAYLOAD_MAX_BYTES: usize = 64 * 1024;

/// Nombre maximal d'appels `connector.call` par invocation.
///
/// Appliqué par la plateforme ; le SDK recopie la valeur.
pub const CONNECTOR_CALLS_PER_INVOCATION: usize = 5;

// ── Connecteurs (`host::connectors::call`) ───────────────────────────────────────────────

/// Taille maximale d'une réponse de connecteur, en octets (1 Mio).
///
/// Au-delà, la plateforme coupe la lecture (`connector_response_too_large`).
pub const CONNECTOR_RESPONSE_MAX_BYTES: usize = 1024 * 1024;

/// Délai maximal d'une requête sortante de connecteur, en millisecondes.
///
/// Plateforme seule : la requête part de la gateway, le module n'en voit que l'échec.
pub const CONNECTOR_REQUEST_TIMEOUT_MS: u64 = 15_000;

/// Les requêtes sortantes de connecteur ne partent qu'en HTTPS.
///
/// Plateforme seule : la gateway refuse toute autre URL de base, quoi que déclare le module.
pub const CONNECTOR_HTTPS_ONLY: bool = true;

// ── KV (`host::kv`) ──────────────────────────────────────────────────────────────────────
//
// Comptés en octets. Une portée est un couple (module, logement) : c'est l'espace de clés
// qu'un module voit, et le quota couvre toutes ses invocations passées.

/// Taille maximale d'une clé KV, en octets UTF-8.
///
/// Au-delà, la plateforme répond `kv_key_too_large`. Reproduit par le mock de
/// `portaki-test-utils`.
pub const KV_KEY_MAX_BYTES: usize = 256;

/// Taille maximale d'une valeur KV, en octets (64 Kio).
///
/// Au-delà, la plateforme répond `kv_value_too_large`. Reproduit par le mock de
/// `portaki-test-utils`.
pub const KV_VALUE_MAX_BYTES: usize = 64 * 1024;

/// Nombre maximal de clés KV dans une portée (module, logement).
///
/// Au-delà, la plateforme répond `kv_quota_exceeded` ; remplacer une clé existante ne compte
/// pas comme une clé de plus. Le mock de `portaki-test-utils` compte sur son propre magasin,
/// entrées pré-remplies comprises.
pub const KV_KEYS_PER_SCOPE: usize = 1000;

/// Nombre maximal d'octets de valeurs KV dans une portée (module, logement) (5 Mio).
///
/// Au-delà, la plateforme répond `kv_quota_exceeded` ; remplacer une valeur ne consomme que la
/// différence. Le mock de `portaki-test-utils` compte sur son propre magasin, entrées
/// pré-remplies comprises.
pub const KV_BYTES_PER_SCOPE: usize = 5 * 1024 * 1024;

// ── Emails invités ───────────────────────────────────────────────────────────────────────

/// Jours après le check-out pendant lesquels un email à l'audience invité reste accepté.
///
/// Passé `checkout_at + 7 jours`, la plateforme refuse (`email_stay_ended`) : l'invité est
/// parti depuis longtemps, et un rappel tardif ressemble à du spam. Le SDK vérifie la règle
/// quand le séjour visé est celui de l'invocation — le seul dont il connaît le check-out.
pub const GUEST_EMAIL_DAYS_AFTER_CHECKOUT: i64 = 7;

/// Emails de modules par séjour invité sur 24 h glissantes, tous modules confondus.
///
/// Plateforme seule : le compte couvre les autres modules et les invocations passées.
pub const GUEST_STAY_EMAILS_PER_24H: usize = 3;

/// Emails de modules par séjour invité au total, tous modules confondus.
///
/// Plateforme seule : le compte couvre les autres modules et les invocations passées.
pub const GUEST_STAY_EMAILS_TOTAL: usize = 10;

// ── Emails hôte ──────────────────────────────────────────────────────────────────────────

/// Emails à l'audience hôte par module et par workspace sur 24 h glissantes.
///
/// Plateforme seule : le compte couvre les invocations passées.
pub const HOST_EMAILS_PER_MODULE_PER_24H: usize = 20;
