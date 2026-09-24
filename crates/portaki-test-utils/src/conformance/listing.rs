//! `listing.json` — the public listing, when the module versions it — against its schema.

use serde_json::Value;

use super::{Findings, Module};

/// The public listing a module may version next to its manifest.
pub const LISTING_FILE: &str = "listing.json";

/// `schema/listing.v1.json` of the SDK, carried in this crate for the same reason as
/// [`MODULE_SCHEMA_V1`](super::MODULE_SCHEMA_V1) — and checked for drift the same way.
pub const LISTING_SCHEMA_V1: &str = include_str!("../../schema/listing.v1.json");

/// How the `portaki init` template's instructions start, in French and in English.
///
/// Published as is, they would become the catalogue listing: the author left them, so the check
/// refuses them instead of letting a placeholder through.
pub const TEMPLATE_MARKERS: [&str; 2] = ["À compléter", "To be completed"];

const TEMPLATE_LEFT: &str = "listing.json still holds the init template — fill it in, or delete \
                             it to write the listing in the dashboard";

pub(super) fn check(module: &Module) -> Result<(), Findings> {
    Findings::of("listing", problems(module))
}

fn problems(module: &Module) -> Vec<String> {
    let path = module.root().join(LISTING_FILE);
    // Optional: the listing can be written in the dashboard instead.
    if !path.exists() {
        return Vec::new();
    }
    let listing: Value = match std::fs::read_to_string(&path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))
        .and_then(|raw| {
            serde_json::from_str(&raw)
                .map_err(|error| format!("{} is not JSON: {error}", path.display()))
        }) {
        Ok(listing) => listing,
        Err(error) => return vec![error],
    };

    let mut problems = Vec::new();
    if holds_template(&listing) {
        problems.push(TEMPLATE_LEFT.to_string());
    }

    let schema: Value =
        serde_json::from_str(LISTING_SCHEMA_V1).expect("the bundled listing.v1.json is JSON");
    let validator = match jsonschema::validator_for(&schema) {
        Ok(validator) => validator,
        Err(error) => {
            problems.push(format!(
                "the bundled listing.v1.json does not compile: {error}"
            ));
            return problems;
        }
    };
    problems.extend(validator.iter_errors(&listing).map(|error| {
        let at = error.instance_path().to_string();
        let at = if at.is_empty() { "/".to_string() } else { at };
        format!("{LISTING_FILE} at {at}: {error}")
    }));
    problems
}

/// Any text of the listing still starts with one of the template's instructions.
fn holds_template(value: &Value) -> bool {
    match value {
        Value::String(text) => TEMPLATE_MARKERS
            .iter()
            .any(|marker| text.trim_start().starts_with(marker)),
        Value::Array(items) => items.iter().any(holds_template),
        Value::Object(fields) => fields.values().any(holds_template),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn module_with(listing: Option<&str>) -> (tempdir::Dir, Module) {
        let dir = tempdir::Dir::new();
        if let Some(listing) = listing {
            std::fs::write(dir.path().join(LISTING_FILE), listing).unwrap();
        }
        let module = Module::at(dir.path());
        (dir, module)
    }

    /// A throwaway directory, removed on drop — enough here to avoid a dev-dependency.
    mod tempdir {
        use std::path::{Path, PathBuf};
        use std::sync::atomic::{AtomicUsize, Ordering};

        pub struct Dir(PathBuf);

        impl Dir {
            pub fn new() -> Self {
                static NEXT: AtomicUsize = AtomicUsize::new(0);
                let path = std::env::temp_dir().join(format!(
                    "portaki-listing-{}-{}",
                    std::process::id(),
                    NEXT.fetch_add(1, Ordering::Relaxed)
                ));
                std::fs::create_dir_all(&path).unwrap();
                Self(path)
            }

            pub fn path(&self) -> &Path {
                &self.0
            }
        }

        impl Drop for Dir {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
    }

    const VALID: &str = r#"{
        "$schema": "https://raw.githubusercontent.com/PortakiApp/portaki-sdk/main/schema/listing.v1.json",
        "category": "stay",
        "tagline": { "fr": "La météo du séjour.", "en": "Weather for the stay." },
        "description": { "fr": "Prévisions à 5 jours." },
        "hostSurface": { "fr": "Une fiche pour les unités." },
        "configItems": [{ "fr": "Unités" }],
        "capabilities": [{ "label": { "fr": "Données météo" }, "kind": "pool" }],
        "publishedLangs": ["fr", "en"]
    }"#;

    #[test]
    fn no_listing_is_fine() {
        let (_dir, module) = module_with(None);
        assert!(check(&module).is_ok());
    }

    #[test]
    fn a_valid_listing_passes() {
        let (_dir, module) = module_with(Some(VALID));
        check(&module).unwrap();
    }

    #[test]
    fn an_invalid_listing_is_reported_where_it_breaks() {
        let (_dir, module) = module_with(Some(
            r#"{
                "category": "shopping",
                "tagline": { "en": "No French", "pt": "Olá" },
                "description": { "fr": "ok" },
                "hostSurface": { "fr": "ok" },
                "capabilities": [{ "label": { "fr": "x" }, "kind": "free" }],
                "publishedLangs": ["fr", "fr"],
                "screenshots": []
            }"#,
        ));
        let problems = problems(&module);
        let reports = |fragments: &[&str]| {
            problems
                .iter()
                .any(|problem| fragments.iter().all(|fragment| problem.contains(fragment)))
        };

        assert!(
            reports(&["listing.json at /category", "shopping"]),
            "{problems:?}"
        );
        assert!(reports(&["listing.json at /tagline", "fr"]), "{problems:?}");
        assert!(reports(&["/tagline", "pt"]), "{problems:?}");
        assert!(reports(&["/capabilities/0/kind", "free"]), "{problems:?}");
        assert!(reports(&["/publishedLangs"]), "{problems:?}");
        assert!(
            reports(&["listing.json at /", "screenshots"]),
            "{problems:?}"
        );
        assert_eq!(check(&module).unwrap_err().checks(), vec!["listing"]);
    }

    #[test]
    fn a_tagline_past_90_characters_is_refused() {
        let long = "é".repeat(91);
        let listing = VALID.replace("La météo du séjour.", &long);
        let (_dir, module) = module_with(Some(&listing));
        let problems = problems(&module);
        assert!(
            problems.iter().any(|p| p.contains("/tagline/fr")),
            "{problems:?}"
        );
    }

    #[test]
    fn the_unfilled_init_template_fails_explicitly() {
        let listing = VALID.replace(
            "Une fiche pour les unités.",
            "À compléter : ce que l'hôte voit dans le dashboard.",
        );
        let (_dir, module) = module_with(Some(&listing));
        // Valid against the schema, refused all the same.
        assert_eq!(problems(&module), vec![TEMPLATE_LEFT.to_string()]);
    }
}
