//! OCI artifact packaging and push to Portaki's OCI repository (ORAS-compatible layout).
//!
//! Le dépôt est celui de la plateforme, et lui seul : le registre Portaki délivre un droit de
//! push court, limité au module et à la version (`POST /registry/v1/publications/push-token`),
//! et nomme l'hôte OCI dans sa réponse. Aucun hôte n'est écrit ici, aucun identifiant n'est lu
//! sur la machine.

pub mod pack;

use std::path::Path;

use anyhow::{Context, Result};
use oci_distribution::client::{Client, ClientConfig, ClientProtocol, Config};
use oci_distribution::secrets::RegistryAuth;
use oci_distribution::Reference;

/// Le droit de pousser une version, tel que le registre Portaki le rend.
///
/// `password` est un secret de quinze minutes : il ne s'affiche pas, ne s'écrit que dans le
/// `DOCKER_CONFIG` éphémère que cosign lit, et ne sort jamais sur la ligne de commande.
#[derive(Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PushGrant {
    /// `oci.portaki.app` — l'hôte OCI, jamais deviné par la CLI.
    pub registry: String,
    /// `modules/<id>`
    pub repository: String,
    /// `oci.portaki.app/modules/<id>:<version>`
    pub reference: String,
    pub username: String,
    pub password: String,
}

impl std::fmt::Debug for PushGrant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PushGrant")
            .field("reference", &self.reference)
            .field("password", &"<redacted>")
            .finish()
    }
}

impl PushGrant {
    /// Refuse un droit qui ne nomme pas ce module et cette version : la CLI ne pousse pas sous
    /// un autre nom que celui qu'elle a demandé, même si on le lui propose.
    pub fn check(&self, coords: &pack::ModuleCoordinates) -> Result<()> {
        let repository = format!("modules/{}", coords.id);
        let reference = format!("{}/{repository}:{}", self.registry, coords.version);
        if self.repository != repository || self.reference != reference {
            anyhow::bail!(
                "the registry granted a push to {}, not to {reference} — refusing to push",
                self.reference
            );
        }
        Ok(())
    }

    fn auth(&self) -> RegistryAuth {
        RegistryAuth::Basic(self.username.clone(), self.password.clone())
    }
}

/// Ce qu'une poussée laisse derrière elle.
///
/// Le digest est relu chez le dépôt OCI plutôt que déduit de l'URL du manifeste : c'est lui
/// qui identifie une publication chez le registre Portaki (ADR-0005).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PushedArtifact {
    /// `oci.portaki.app`
    pub registry: String,
    /// `modules/nuki`
    pub repository: String,
    /// `sha256:…`
    pub digest: String,
}

impl PushedArtifact {
    /// `oci.portaki.app/modules/nuki@sha256:…` — ce que cosign signe : le digest, jamais le tag.
    pub fn subject(&self) -> String {
        format!("{}/{}@{}", self.registry, self.repository, self.digest)
    }

    /// La forme que `POST /registry/v1/publications` exige.
    pub fn artifact_ref(&self) -> String {
        format!("oci://{}", self.subject())
    }
}

/// Un dépôt de développement (compose local) se parle en HTTP ; tous les autres en HTTPS.
pub fn is_local(registry: &str) -> bool {
    let host = registry.rsplit_once(':').map_or(registry, |(host, _)| host);
    host == "localhost" || host == "127.0.0.1" || host.ends_with(".localhost")
}

/// Pushes the module artifact under the reference the push grant names.
///
/// Expects `portaki build` output under `artifact_dir`:
/// - `publish-manifest.json` (frozen catalog for OCI)
/// - `module_root/target/wasm32-unknown-unknown/release/*.wasm`
/// - `module_root/i18n/*.json` (optional)
pub async fn push_artifact(
    module_root: &Path,
    artifact_dir: &Path,
    grant: &PushGrant,
) -> Result<PushedArtifact> {
    let coords = pack::read_module_coordinates(module_root, artifact_dir)?;
    grant.check(&coords)?;
    let layers = pack::collect_push_layers(module_root, artifact_dir)?;
    let reference: Reference = grant
        .reference
        .parse()
        .with_context(|| format!("invalid OCI reference: {}", grant.reference))?;

    let image_layers = pack::layers_to_image_layers(&layers)?;
    let config = Config::new(
        br#"{}"#.to_vec(),
        "application/vnd.oci.empty.v1+json".to_string(),
        None,
    );

    let client = client(&grant.registry);
    let auth = grant.auth();
    client
        .push(&reference, &image_layers, config, &auth, None)
        .await
        .with_context(|| format!("push to {}", grant.reference))?;

    // Le tag, relu : c'est ce que le registre relira à l'annonce, et ce que cosign signera.
    let digest = client
        .fetch_manifest_digest(&reference, &auth)
        .await
        .context("read back the pushed manifest digest")?;

    Ok(PushedArtifact {
        registry: grant.registry.clone(),
        repository: grant.repository.clone(),
        digest,
    })
}

fn client(registry: &str) -> Client {
    let protocol = if is_local(registry) {
        ClientProtocol::HttpsExcept(vec![registry.to_string()])
    } else {
        ClientProtocol::Https
    };
    Client::new(ClientConfig {
        protocol,
        ..ClientConfig::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grant(reference: &str, repository: &str) -> PushGrant {
        PushGrant {
            registry: "oci-staging.portaki.app".to_string(),
            repository: repository.to_string(),
            reference: reference.to_string(),
            username: "portaki-push".to_string(),
            password: "secret".to_string(),
        }
    }

    fn nuki() -> pack::ModuleCoordinates {
        pack::ModuleCoordinates {
            id: "nuki".to_string(),
            version: "1.4.0".to_string(),
        }
    }

    #[test]
    fn the_announced_reference_pins_the_digest_in_the_modules_repository() {
        let pushed = PushedArtifact {
            registry: "oci-staging.portaki.app".to_string(),
            repository: "modules/nuki".to_string(),
            digest: "sha256:9f2c".to_string(),
        };

        assert_eq!(
            pushed.subject(),
            "oci-staging.portaki.app/modules/nuki@sha256:9f2c"
        );
        assert_eq!(
            pushed.artifact_ref(),
            "oci://oci-staging.portaki.app/modules/nuki@sha256:9f2c"
        );
    }

    #[test]
    fn a_grant_for_another_module_or_version_is_refused() {
        let good = grant("oci-staging.portaki.app/modules/nuki:1.4.0", "modules/nuki");
        assert!(good.check(&nuki()).is_ok());

        for (reference, repository) in [
            (
                "oci-staging.portaki.app/modules/other:1.4.0",
                "modules/other",
            ),
            ("oci-staging.portaki.app/modules/nuki:1.5.0", "modules/nuki"),
            ("evil.example/modules/nuki:1.4.0", "modules/nuki"),
        ] {
            let error = grant(reference, repository)
                .check(&nuki())
                .unwrap_err()
                .to_string();
            assert!(error.contains("refusing to push"), "{reference}: {error}");
        }
    }

    #[test]
    fn the_password_never_shows_in_debug_output() {
        let shown = format!("{:?}", grant("r", "modules/nuki"));
        assert!(!shown.contains("secret"), "{shown}");
    }

    #[test]
    fn only_a_development_host_is_spoken_to_in_http() {
        assert!(is_local("oci.localhost:8080"));
        assert!(is_local("localhost:5000"));
        assert!(is_local("127.0.0.1:5000"));
        assert!(!is_local("oci-staging.portaki.app"));
        assert!(!is_local("localhost.evil.example"));
    }
}
