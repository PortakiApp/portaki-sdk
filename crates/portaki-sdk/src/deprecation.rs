//! Ce que la plateforme retire, depuis quand, et par quoi le remplacer.
//!
//! # Pourquoi ici plutôt que dans un fichier
//!
//! Une capacité est déclarée en Rust ; sa dépréciation l'est au même endroit, sinon les deux
//! divergent le jour où l'une bouge sans l'autre. Le document JSON que le registre distribue
//! est **produit** à partir de cette table, et un test vérifie que celui qui est versionné n'a
//! pas dérivé — c'est ce qui rend la duplication sûre.
//!
//! # Comment il voyage
//!
//! La CI du SDK publie `deprecations.json` avec les autres contrats d'une version, sur
//! `/registry/v1/sdk-releases`. `portaki ci check` le relit et avertit un module qui s'appuie
//! encore sur ce qui part. Rien n'échoue jamais pour cette raison : une dépréciation prévient,
//! elle n'interdit pas.
//!
//! # Exemples
//!
//! ```
//! use portaki_sdk::deprecation;
//!
//! // Rien n'est déprécié aujourd'hui ; le mécanisme, lui, répond.
//! assert!(deprecation::find("core.storage").is_none());
//! ```

use serde::{Deserialize, Serialize};

/// Ce à quoi un identifiant déprécié se rapporte.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Subject {
    /// Une capacité de [`crate::capability`].
    Capability,
    /// Un connecteur déclaré par un module.
    Connector,
    /// Une opération hôte de `host-ops.json`.
    HostOp,
}

/// Un retrait annoncé.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Deprecation {
    /// L'identifiant qui part — `core.storage`, `nuki`, `kv.list`.
    pub id: &'static str,
    /// Ce dont il s'agit.
    pub subject: Subject,
    /// La version du SDK à partir de laquelle il est déprécié.
    pub since: &'static str,
    /// Par quoi le remplacer, quand un remplaçant existe.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replacement: Option<&'static str>,
    /// Ce qu'un auteur de module a besoin de savoir pour agir.
    pub note: &'static str,
}

/// Tout ce qui est déprécié à ce jour.
///
/// Vide, et c'est exact : rien n'a encore été retiré. La table existe pour que la première
/// dépréciation soit une ligne à ajouter, pas un dispositif à concevoir dans l'urgence — au
/// moment précis où l'on veut prévenir les auteurs, pas construire.
pub const DEPRECATIONS: &[Deprecation] = &[];

/// Ce qui est déprécié sous cet identifiant, s'il l'est.
pub fn find(id: &str) -> Option<&'static Deprecation> {
    DEPRECATIONS.iter().find(|entry| entry.id == id)
}

/// Le document que la CI publie au registre, tel qu'il doit être versionné.
///
/// Une fonction plutôt qu'un fichier de référence : le contenu vient de [`DEPRECATIONS`], et
/// le JSON du dépôt n'en est qu'une empreinte, vérifiée par les tests.
pub fn contract() -> serde_json::Value {
    serde_json::json!({
        "description":
            "Capabilities, connectors and host ops being withdrawn — advisory, never blocking",
        "deprecations": DEPRECATIONS,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Le JSON versionné est une empreinte de la table, pas une seconde source.
    ///
    /// Sans ce test, ajouter une dépréciation en Rust sans régénérer le fichier publierait un
    /// contrat qui ne décrit pas le SDK — l'exacte erreur que la table est censée empêcher.
    #[test]
    fn the_checked_in_contract_matches_the_table() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../contracts/deprecations.json");
        let Ok(raw) = std::fs::read_to_string(&path) else {
            // Le crate empaqueté n'emporte pas le dossier du dépôt ; il n'y a alors rien à
            // comparer, et rien à signaler.
            return;
        };
        let versioned: serde_json::Value =
            serde_json::from_str(&raw).expect("contracts/deprecations.json est du JSON");

        assert_eq!(
            versioned,
            contract(),
            "contracts/deprecations.json a dérivé de deprecation::DEPRECATIONS"
        );
    }

    #[test]
    fn an_unknown_id_is_not_deprecated() {
        assert!(find("core.storage").is_none());
        assert!(find("nothing.at.all").is_none());
    }

    /// La forme du document compte autant que son contenu : c'est elle que le CLI relit.
    #[test]
    fn a_deprecation_serialises_as_the_cli_reads_it() {
        let entry = Deprecation {
            id: "core.storage",
            subject: Subject::Capability,
            since: "2.4.0",
            replacement: Some("core.kv"),
            note: "typed repositories replace raw storage",
        };

        let rendered = serde_json::to_value(&entry).unwrap();

        assert_eq!(rendered["id"], "core.storage");
        assert_eq!(rendered["subject"], "capability");
        assert_eq!(rendered["since"], "2.4.0");
        assert_eq!(rendered["replacement"], "core.kv");
    }

    /// Sans remplaçant, la clé disparaît au lieu d'apparaître nulle : un lecteur distingue
    /// « pas de remplaçant » de « remplaçant inconnu » sans convention supplémentaire.
    #[test]
    fn a_deprecation_without_a_replacement_omits_the_field() {
        let entry = Deprecation {
            id: "kv.list",
            subject: Subject::HostOp,
            since: "2.4.0",
            replacement: None,
            note: "unbounded listing never scaled",
        };

        let rendered = serde_json::to_value(&entry).unwrap();

        assert!(rendered.get("replacement").is_none());
    }
}
