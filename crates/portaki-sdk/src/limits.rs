//! Plafonds appliqués par la plateforme aux modules — une seule table.
//!
//! Chaque valeur ici est **aussi** appliquée par la plateforme (runtime et orchestrateur),
//! et c'est elle qui fait foi : le SDK n'accorde rien de plus. Il recopie les valeurs pour
//! échouer tôt. Un dépassement vérifié côté SDK remonte en erreur typée dès `cargo test`
//! (via `portaki-test-utils`), alors qu'en production un email refusé par l'orchestrateur
//! est simplement abandonné, sans retour vers le module.
//!
//! Certaines limites comptent au-delà d'une invocation ou d'un module (fenêtres glissantes,
//! tous modules confondus) : le SDK ne peut pas les vérifier. Elles figurent quand même ici,
//! marquées « plateforme seule », pour qu'un auteur de module les trouve au même endroit.
//!
//! Changer une valeur ici sans que la plateforme change la sienne ne change rien en
//! production — seulement ce que les tests laissent passer.

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

/// Nombre maximal d'appels `connector.call` par invocation.
pub const CONNECTOR_CALLS_PER_INVOCATION: usize = 5;

/// Taille maximale d'une réponse de connecteur, en octets (1 Mio).
pub const CONNECTOR_RESPONSE_MAX_BYTES: usize = 1024 * 1024;

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

// ── Fichiers invités (`ImageUpload`, permission `guest:files`) ───────────────────────────
//
// Appliqués par l'endpoint d'upload voyageur de la plateforme : le SDK ne voit jamais les
// octets, seulement la référence ([`crate::files::FileRef`]) qu'un formulaire lui transmet.

/// Taille maximale d'un fichier invité, en octets (5 Mio).
pub const GUEST_FILE_MAX_BYTES: usize = 5 * 1024 * 1024;

/// Types acceptés, vérifiés sur les octets et non sur l'en-tête déclaré. La plateforme
/// réencode l'image, ce qui retire les métadonnées (EXIF, position GPS).
pub const GUEST_FILE_CONTENT_TYPES: &[&str] = &["image/jpeg", "image/png"];

/// Fichiers qu'un séjour peut envoyer, tous modules confondus.
///
/// Plateforme seule : le compte couvre les autres modules et les invocations passées.
pub const GUEST_FILES_PER_STAY: usize = 20;
