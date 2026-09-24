//! The conformance battery every Portaki module runs — one call, seven checks.
//!
//! A module's own tests say what it does. These say what every module owes the platform, the same
//! way for all of them: a manifest the registry accepts, a listing it can serve, surfaces that
//! render and parse, operations that do not bring the invocation down, i18n keys that exist in
//! both bundles, emails that compose.
//!
//! # Adopting it
//!
//! One file, `tests/conformance.rs`:
//!
//! ```ignore
//! portaki_test_utils::conformance!();
//! ```
//!
//! That generates one `#[test]` per check under `portaki_conformance::` — `cargo test` (and
//! `portaki publish`, which runs it) reports each on its own line. The macro links the module's
//! library into the test binary; when `[lib] name` cannot be read, name it:
//! `conformance!(crate = my_module)`.
//!
//! # What is checked
//!
//! | Test | Check |
//! |------|-------|
//! | `manifest` | `portaki.module.json` validates against the `module.v1.json` schema bundled in this crate |
//! | `listing` | `listing.json`, when the module versions one, validates against the `listing.v1.json` schema bundled in this crate and no longer holds the `portaki init` instructions (`À compléter …` / `To be completed …`); no `listing.json` passes — the listing can be written in the dashboard |
//! | `surfaces` | every `#[surface]` renders in its shell (guest or host) with an empty mock, without panicking or failing; the tree it sends parses as SDUI primitives of the contract, and every `Select` in it has options and a `value` among them (or none); every `guestSurfaces[].surfaceId` of the manifest is a declared guest surface |
//! | `operations` | every `#[command]` and `#[query]` dispatched with `{}` in a guest and a host mock does not panic — an `Err` is a fine answer to empty input |
//! | `i18n` | every key the manifest (`guestSurfaces[].labelKey`), the rendered surfaces (`"i18n:…"`) and the handlers (`host::i18n::translate`) use exists in the `fr` and `en` bundles of `i18n/` |
//! | `emails` | every `emails[]` entry that names a command dispatches it, and an `emailContext` query composes for every template key, around a mock stay, without panicking |
//! | `contracts` | a `property-stats-card` surface: `statsSummary` answers for its `pathSegment` over 30, 90 and 365 days, on the `stats-summary.v1.json` contract (`fr` and `en`, `value` ≤ 12 characters), within 300 ms; a `property-stats-detail`: a host surface of id `pathSegment` renders with `input.periodDays`; a `workspace-timeline-task`: `timelineTasks` answers on three fixture stays on the `timeline-tasks.v1.json` contract (ISO dates, items never empty), and `taskToggle` refuses a photo-required item ticked without a photo with `photo_required`; an exported `publishReadiness` answers on the `publish-readiness.v1.json` contract |
//!
//! "Empty mock" is [`MockContext::guest`](crate::MockContext::guest) or
//! [`MockContext::host`](crate::MockContext::host) as they come: no KV, no seeded translation, no
//! connector stub (a connector answers `{}`). A first install looks exactly like that.
//!
//! # Where the handlers come from
//!
//! `#[query]`, `#[command]` and `#[surface]` register a
//! [`portaki_sdk::wasm::registry::HandlerDeclaration`] on native targets. The
//! battery reads those — the same shims the Wasm entry points call — so there is no list to keep in
//! sync, and a handler added tomorrow is checked tomorrow.
//!
//! # Not checked, and why
//!
//! - **The sandbox clock.** A module must read time through `host::time::now`; `Utc::now()` compiles
//!   and runs natively, and nothing observable from a test tells the two apart. `portaki lint` and
//!   clippy's `disallowed-methods` are where that belongs.
//! - **`portaki_module!` display keys** (`module.displayName`, …). The macro writes them to the build
//!   emissions only; a test binary cannot read them back.
//! - **Rendered content beyond the empty state.** A surface with data exercises other branches; that
//!   is what the module's own tests are for.
//! - **Event handlers.** `#[event_handler]` registers no dispatch shim to call.

mod contracts;
mod emails;
mod findings;
mod i18n;
mod invoke;
mod listing;
mod manifest;
mod operations;
mod surfaces;

use std::path::{Path, PathBuf};

use portaki_sdk::wasm::registry::{self, HandlerDeclaration};
use serde_json::Value;

pub use contracts::{
    PUBLISH_READINESS_SCHEMA_V1, STATS_SUMMARY_SCHEMA_V1, TIMELINE_TASKS_SCHEMA_V1,
};
pub use findings::Findings;
pub use listing::{LISTING_FILE, LISTING_SCHEMA_V1, TEMPLATE_MARKERS};
pub use manifest::MODULE_SCHEMA_V1;

/// The module under test: its crate directory, and the handlers linked into this test binary.
#[derive(Debug, Clone)]
pub struct Module {
    root: PathBuf,
}

impl Module {
    /// The module whose `Cargo.toml` lives in `root` — `env!("CARGO_MANIFEST_DIR")` in a test.
    pub fn at(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// The crate directory.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// `portaki.module.json` validates against the bundled `module.v1.json` schema.
    pub fn check_manifest(&self) -> Result<(), Findings> {
        manifest::check(self)
    }

    /// `listing.json`, if any, validates against the bundled `listing.v1.json` and is filled in.
    pub fn check_listing(&self) -> Result<(), Findings> {
        listing::check(self)
    }

    /// Every surface renders in its shell with an empty mock, into primitives of the contract.
    pub fn check_surfaces(&self) -> Result<(), Findings> {
        surfaces::check(self)
    }

    /// Every command and query survives `{}` in a guest and a host mock.
    pub fn check_operations(&self) -> Result<(), Findings> {
        operations::check(self)
    }

    /// Every i18n key the manifest, the surfaces and the handlers use exists in `fr` and `en`.
    pub fn check_i18n(&self) -> Result<(), Findings> {
        i18n::check(self)
    }

    /// Every declared email command and the `emailContext` query compose around a mock stay.
    pub fn check_emails(&self) -> Result<(), Findings> {
        emails::check(self)
    }

    /// What the declared host surfaces commit the module to: `statsSummary` for a stats card, a
    /// host surface for a stats detail, `timelineTasks` for a timeline task — and a
    /// `publishReadiness` on the contract when it is exported.
    pub fn check_contracts(&self) -> Result<(), Findings> {
        contracts::check(self)
    }

    /// All seven checks; the findings of every failing one, together.
    pub fn check_all(&self) -> Result<(), Findings> {
        let results = [
            self.check_manifest(),
            self.check_listing(),
            self.check_surfaces(),
            self.check_operations(),
            self.check_i18n(),
            self.check_emails(),
            self.check_contracts(),
        ];
        let failed: Vec<Findings> = results.into_iter().filter_map(Result::err).collect();
        if failed.is_empty() {
            Ok(())
        } else {
            Err(Findings::merge(failed))
        }
    }

    /// The module's manifest, parsed: the one `portaki build` wrote — the code, with a kept
    /// `portaki.module.json` merged in — or that file alone before any build; `Ok(None)` when
    /// neither exists.
    pub(crate) fn manifest(&self) -> Result<Option<Value>, String> {
        let Some(path) = [BUILT_MANIFEST_FILE, MANIFEST_FILE]
            .iter()
            .map(|file| self.root.join(file))
            .find(|path| path.exists())
        else {
            return Ok(None);
        };
        let raw = std::fs::read_to_string(&path)
            .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
        serde_json::from_str(&raw)
            .map(Some)
            .map_err(|error| format!("{} is not JSON: {error}", path.display()))
    }

    /// The handlers linked into this test binary, sorted so reports read the same on every run.
    pub(crate) fn declarations(&self) -> Vec<&'static HandlerDeclaration> {
        let mut all: Vec<_> = registry::declarations().collect();
        all.sort_by_key(|declaration| (declaration.context, declaration.name, declaration.fn_name));
        all
    }
}

/// The catalogue manifest a module may still write by hand, at the crate root.
pub const MANIFEST_FILE: &str = "portaki.module.json";

/// The manifest `portaki build` writes from the code — what is published when there is no
/// hand-written one.
pub const BUILT_MANIFEST_FILE: &str = "target/portaki/publish-manifest.json";

/// Why a handler-based check found nothing to check.
///
/// Every module declares at least one handler. None at all means the battery cannot see them —
/// not that there is nothing to see — so it fails instead of passing on an empty list.
pub(crate) const NO_DECLARATIONS: &str =
    "no #[surface], #[query] or #[command] is linked into this test binary — the module crate \
     must be built with portaki-sdk-macros from the same release as portaki-test-utils (older \
     macros register handlers for wasm32 only), and the test must call conformance!() so the \
     library is linked";

/// Generates the conformance tests of the module this integration test belongs to.
///
/// ```ignore
/// // tests/conformance.rs
/// portaki_test_utils::conformance!();
/// ```
///
/// Expands to a `portaki_conformance` module with one `#[test]` per check — `manifest`,
/// `listing`, `surfaces`, `operations`, `i18n`, `emails`, `contracts` — see
/// [`conformance`](mod@crate::conformance) for what each one verifies.
///
/// # Forms
///
/// - `conformance!()` — the package's library, read from `[lib] name` or the package name.
/// - `conformance!(crate = my_module)` — the library named explicitly.
/// - `conformance!(dir = "…")` — another crate directory than `CARGO_MANIFEST_DIR`, for fixtures.
///
/// Call it from an integration test (`tests/*.rs`), not from the library itself: it links the
/// library with `extern crate`, which a crate cannot do to itself.
#[macro_export]
macro_rules! conformance {
    () => {
        $crate::conformance!(@tests ::core::env!("CARGO_MANIFEST_DIR"));
        $crate::__private::portaki_sdk::__link_module_crate!();
    };
    (crate = $lib:ident $(,)?) => {
        $crate::conformance!(@tests ::core::env!("CARGO_MANIFEST_DIR"));
        $crate::__private::portaki_sdk::__link_module_crate!($lib);
    };
    (dir = $dir:expr $(,)?) => {
        $crate::conformance!(@tests $dir);
    };
    (@tests $dir:expr) => {
        /// The Portaki conformance battery — see `portaki_test_utils::conformance`.
        #[cfg(test)]
        mod portaki_conformance {
            fn module() -> $crate::conformance::Module {
                $crate::conformance::Module::at($dir)
            }

            #[test]
            fn manifest() {
                if let Err(findings) = module().check_manifest() {
                    panic!("{findings}");
                }
            }

            #[test]
            fn listing() {
                if let Err(findings) = module().check_listing() {
                    panic!("{findings}");
                }
            }

            #[test]
            fn surfaces() {
                if let Err(findings) = module().check_surfaces() {
                    panic!("{findings}");
                }
            }

            #[test]
            fn operations() {
                if let Err(findings) = module().check_operations() {
                    panic!("{findings}");
                }
            }

            #[test]
            fn i18n() {
                if let Err(findings) = module().check_i18n() {
                    panic!("{findings}");
                }
            }

            #[test]
            fn emails() {
                if let Err(findings) = module().check_emails() {
                    panic!("{findings}");
                }
            }

            #[test]
            fn contracts() {
                if let Err(findings) = module().check_contracts() {
                    panic!("{findings}");
                }
            }
        }
    };
}
