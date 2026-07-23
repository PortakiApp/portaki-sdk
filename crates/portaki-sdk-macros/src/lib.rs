//! Proc-macros that declare Portaki Wasm module metadata at compile time.
//!
//! Each macro prepends an invisible `const` block (via `emit::write_emission`) that writes a
//! JSON fragment when `OUT_DIR` is set during `cargo build`. Fragments land under:
//!
//! ```text
//! {OUT_DIR}/portaki-emissions/{kind}-{sanitized_key}.json
//! ```
//!
//! where `OUT_DIR` is Cargo's per-crate build output directory (e.g.
//! `target/debug/build/<crate-hash>/out/` for host builds, or the analogous path under
//! `target/wasm32-unknown-unknown/` for Wasm targets).
//!
//! # Build pipeline
//!
//! 1. Module source uses these macros (typically via `portaki_sdk::prelude::*` re-exports).
//! 2. `cargo build --target wasm32-unknown-unknown` expands macros and writes emission files.
//! 3. [`portaki build`](https://docs.rs/portaki-cli) calls `find_emissions_dir`, which picks the
//!    **most recently modified** `portaki-emissions` directory under `target/`, then
//!    `collect_emissions` + `generate_manifest` merge fragments into `target/portaki/manifest.json`.
//! 4. `portaki lint` validates capability ids against the host catalog
//!    (`portaki_sdk::capability::ALL` / `is_known`), i18n keys, and manifest shape.
//!
//! A `portaki.module.json` catalog at the module root can override or supplement emissions; if
//! neither emissions nor a catalog exist, `portaki build` fails.
//!
//! # Emission kinds
//!
//! | Macro | `kind` field | Filename pattern |
//! |-------|--------------|------------------|
//! | `portaki_module` | `module` | `module-{id}.json` |
//! | `entity` | `entity` | `entity-{StructName}.json` |
//! | `entity_indexes` | `entity_indexes` | `entity_indexes-{EntityName}.json` |
//! | `surface` | `surface` | `surface-{context}_{id}.json` |
//! | `query` | `query` | `query-{name}.json` |
//! | `command` | `command` | `command-{name}.json` |
//! | `event_handler` | `event_handler` | `event_handler-{event_type}.json` |
//! | `capability` | `capability` | `capability-{id}.json` |
//! | `connector` | `connector_builtin` | `connector_builtin-{builtin}.json` |
//! | `custom_connector` | `connector_custom` | `connector_custom-{id}.json` |
//! | `connector_op` | `connector_op` | `connector_op-{fn_name}.json` |
//!
//! `entity_indexes` emissions are written for Atlas/migration tooling; they are **not** merged into
//! `manifest.json` today (only `entity` field metadata is).
//!
//! # Wasm runtime code generation
//!
//! `portaki_module` additionally emits Extism export shims (`portaki_query`, `portaki_command`) and
//! a `__getrandom_v03_custom` hook when `target_arch = "wasm32"`.
//!
//! `query`, `command`, and `surface` emit `inventory::submit!` handler registrations (Wasm only)
//! that wire manifest operation names to the annotated Rust function via JSON dispatch.

#![deny(missing_docs)]

mod capability;
mod command;
mod connector;
mod emit;
mod entity;
mod event_handler;
mod module;
mod query;
mod surface;
mod wasm_handler;
mod wire_lit;

use proc_macro::TokenStream;

/// Declares module identity and bootstraps Wasm exports.
///
/// # Forms
///
/// **Function-like (recommended)** — invoke once at the root of `lib.rs` (re-exported as
/// `portaki_sdk::portaki_module!`):
///
/// ```ignore
/// portaki_sdk::portaki_module!(
///     id = "weather",
///     display_name_key = "module.displayName",
///     description_key = "module.description",
///     author = "Syntax Labs",
///     version = "1.0.0", // optional; defaults to compiling crate's CARGO_PKG_VERSION
/// );
/// ```
///
/// **Attribute** — attach to a `mod` item (same attribute keys). Rarely used; prefer the
/// function-like form at crate root.
///
/// ```ignore
/// #[portaki_sdk_macros::portaki_module(id = "weather")]
/// mod internal { /* ... */ }
/// ```
///
/// # Attributes
///
/// | Key | Required | Default |
/// |-----|----------|---------|
/// | `id` | no | `"unknown"` |
/// | `display_name_key` | no | `"module.displayName"` |
/// | `description_key` | no | `"module.description"` |
/// | `author` | no | `"Syntax Labs"` |
/// | `version` | no | `CARGO_PKG_VERSION` of the **module crate** (not this proc-macro crate) |
///
/// Unknown keys are a **compile error** (`unknown portaki_module attribute: …`).
///
/// # Emission
///
/// Writes `module-{id}.json` with `"kind": "module"`, `displayName`, `description`, `author.name`,
/// `version`, `manifestVersion: "1"`, and `uiSchema: { host: "1", guest: "1" }`.
///
/// `portaki build` requires exactly one `module` emission (or a `portaki.module.json` catalog).
///
/// # Generated Rust (Wasm only)
///
/// - `mod __portaki_wasm_getrandom` — fills random bytes via `portaki_sdk::host::wasm_getrandom`.
/// - `mod __portaki_wasm_exports` — `#[plugin_fn] portaki_query` / `portaki_command` delegating
///   to `portaki_sdk::wasm::dispatch`.
#[proc_macro_attribute]
pub fn portaki_module(attr: TokenStream, item: TokenStream) -> TokenStream {
    module::expand(attr, item)
}

/// Function-like module declaration (`portaki_module!(…)`).
///
/// Re-exported from `portaki_sdk` as [`portaki_module`](https://docs.rs/portaki-sdk/latest/portaki_sdk/fn.portaki_module.html).
/// See [`portaki_module`] (attribute) for attribute keys, defaults, emission shape, and Wasm shims.
///
/// Unlike the attribute form, this macro does not wrap a `mod` item — it only emits metadata and
/// Wasm bootstrap modules.
#[proc_macro]
pub fn portaki_module_decl(input: TokenStream) -> TokenStream {
    module::expand_invocation(input)
}

/// Declares a persistent entity schema for Atlas and `host::repo`.
///
/// # Syntax
///
/// ```ignore
/// #[portaki_sdk::entity(schema_version = 1)]
/// pub struct WeatherCache {
///     pub id: uuid::Uuid,
///     pub lat: f64,
///     pub lng: f64,
///     // ...
/// }
/// ```
///
/// # Attributes
///
/// | Key | Required |
/// |-----|----------|
/// | `schema_version` | **yes** — positive integer, bumped when the on-disk schema changes |
///
/// Missing `schema_version` → **compile error**. Unknown keys → **compile error**.
///
/// # Target item
///
/// Must be a struct with **named fields**. Tuple/unit structs emit `"fields": []`.
/// Field types are captured as token strings (e.g. `uuid :: Uuid`), not semantic type names.
///
/// # Emission
///
/// `entity-{StructName}.json`:
///
/// ```json
/// { "kind": "entity", "name": "WeatherCache", "schemaVersion": 1, "fields": [ … ] }
/// ```
///
/// Merged into `manifest.json` → `entities[]`. `portaki build` uses the max `schemaVersion` across
/// entities when writing Atlas migration bundles.
#[proc_macro_attribute]
pub fn entity(attr: TokenStream, item: TokenStream) -> TokenStream {
    entity::expand_entity(attr, item)
}

/// Declares repository indexes for an entity (Atlas spatial/field indexes).
///
/// # Syntax
///
/// ```ignore
/// #[portaki_sdk::entity_indexes(WeatherCache)]
/// pub const WEATHER_CACHE_INDEXES: &[&str] = &["lat", "lng"];
/// ```
///
/// The attribute argument is the entity **type name** (not a string). The decorated item must be a
/// `const` whose value is a string array literal `&["field", …]` or `&[ "field", … ]`.
///
/// Non-literal elements → **compile error** (`entity index entries must be string literals`).
/// Non-array value → **compile error** (message suggests `&["lat", "lng"]`).
///
/// # Index inference
///
/// - `lat` + `lng` together → one `"kind": "spatial"` index with both fields.
/// - Otherwise → one `"kind": "field"` index per entry.
/// - Empty array → `"indexes": []`.
///
/// # Emission
///
/// `entity_indexes-{EntityName}.json`. Not merged into `manifest.json` today; consumed by Atlas /
/// migration tooling. Host and Wasm builds may emit different contents if the const is not visible
/// to the Wasm compile — `portaki build` picks the latest `portaki-emissions` tree under `target/`.
#[proc_macro_attribute]
pub fn entity_indexes(attr: TokenStream, item: TokenStream) -> TokenStream {
    entity::expand_entity_indexes(attr, item)
}

/// Declares a host or guest SDUI surface renderer.
///
/// # Syntax
///
/// ```text
/// #[portaki_sdk::surface(guest, id = "home.card", display_name_key = "surface.home.card")]
/// pub fn render_home_card(ctx: GuestContext) -> Surface { /* ... */ }
///
/// #[portaki_sdk::surface(host, id = "main")]
/// pub fn render_host_main(ctx: HostContext) -> Surface { /* ... */ }
/// ```
///
/// # Attributes
///
/// | Position | Key | Required |
/// |----------|-----|----------|
/// | 1st | `host` or `guest` (identifier) or `"host"` / `"guest"` (string) | yes — surface context |
/// | 2nd | `id = "…"` or `id = SurfaceId::new("…")` | yes — stable surface id in the manifest |
/// | 3rd | `display_name_key = "…"` | no — i18n key; omitted from JSON when absent |
///
/// Wrong first token (not `id =`) → **compile error**. Bare const paths are not
/// resolved at macro time — use a string lit or `Type::new("…")`.
///
/// # Target item
///
/// A function. Its Rust symbol is recorded as `renderFn`. Return type may be `Surface` or
/// `Result<Surface, _>`; the Wasm shim serializes the return value with `serde_json::to_value`.
///
/// # Emission
///
/// `surface-{context}_{id}.json` → merged into `manifest.surfaces.host` or `.guest`.
///
/// # Wasm registration
///
/// Emits `inventory::submit!` with `operation_names: &[render_fn_symbol]` for JSON dispatch.
#[proc_macro_attribute]
pub fn surface(attr: TokenStream, item: TokenStream) -> TokenStream {
    surface::expand(attr, item)
}

/// Declares a read-only gateway query operation.
///
/// # Syntax
///
/// ```text
/// #[portaki_sdk::query(name = "getCurrent")]
/// pub fn get_current(ctx: Context, args: GetCurrentArgs) -> Result<WeatherCurrent> { /* ... */ }
/// ```
///
/// # Attributes
///
/// | Key | Required |
/// |-----|----------|
/// | `name = "…"` | yes — gateway-visible operation name (camelCase convention) |
///
/// Wrong key → **compile error** (`expected name = "…"`).
///
/// # Target item
///
/// A function with signature `(Context, …) -> Result<T>` where the second parameter is optional:
///
/// - `(ctx: Context)` — no JSON params deserialized.
/// - `(ctx: Context, args: Args)` — `args` deserialized from the Wasm `params` JSON object.
///
/// Handlers must not take `self`. Missing context parameter **panics at macro expansion** (proc-macro
/// panic, not `compile_error`).
///
/// # Emission
///
/// `query-{name}.json` → `manifest.queries[]` with `{ name, fn }`.
///
/// # Wasm registration
///
/// `inventory::submit!` registers both the manifest `name` and the Rust `fn` symbol for dispatch.
#[proc_macro_attribute]
pub fn query(attr: TokenStream, item: TokenStream) -> TokenStream {
    query::expand(attr, item)
}

/// Declares a mutating gateway command operation.
///
/// # Syntax
///
/// ```text
/// #[portaki_sdk::command(name = "refreshForecast")]
/// pub fn refresh_forecast(ctx: Context) -> Result<()> { /* ... */ }
/// ```
///
/// Same attribute and signature rules as [`query`]. Emits `command-{name}.json` →
/// `manifest.commands[]`. Wasm `inventory` registration mirrors [`query`].
#[proc_macro_attribute]
pub fn command(attr: TokenStream, item: TokenStream) -> TokenStream {
    command::expand(attr, item)
}

/// Declares a subscription to a platform event type.
///
/// # Syntax
///
/// ```text
/// #[portaki_sdk::event_handler(event_type = "core.booking.confirmed")]
/// pub fn on_booking_confirmed(ctx: Context, event: BookingConfirmedEvent) -> Result<()> { /* ... */ }
/// ```
///
/// `type = "…"` is accepted as an alias for `event_type`.
///
/// # Attributes
///
/// | Key | Required |
/// |-----|----------|
/// | `event_type = "…"` or `type = "…"` | yes |
///
/// Wrong key → **compile error** (`expected type = "…" or event_type = "…"`).
///
/// # Target item
///
/// Handler function; only the symbol name is emitted (`handler` field). No Wasm `inventory`
/// registration — event delivery is orchestrator-driven using manifest metadata.
///
/// # Emission
///
/// `event_handler-{event_type}.json` → `manifest.events.subscribes[]`.
#[proc_macro_attribute]
pub fn event_handler(attr: TokenStream, item: TokenStream) -> TokenStream {
    event_handler::expand(attr, item)
}

/// Declares a required or optional host capability dependency.
///
/// # Syntax
///
/// ```ignore
/// use portaki_sdk::capability;
///
/// #[portaki_sdk::capability(required)]
/// pub const STORAGE: &str = capability::core::STORAGE;
///
/// #[portaki_sdk::capability(
///     optional,
///     id = "external.open-weather.pool",
///     purpose_key = "capability.openWeather.purpose",
///     fallback_key = "capability.openWeather.fallback"
/// )]
/// pub const OPEN_WEATHER_POOL: &str = "external.open-weather.pool";
/// ```
///
/// # Attributes
///
/// | Key | Required | Notes |
/// |-----|----------|-------|
/// | `required` or `optional` | one of | bare flags (no `=`), default is required |
/// | `id = "…"` | if const value is not a string literal | capability id |
/// | `purpose_key = "…"` | no | i18n key for optional capabilities |
/// | `fallback_key = "…"` | no | i18n key when capability unavailable |
///
/// Capability id is taken from `id = "…"` or from a **string literal** const initializer. A const
/// referencing another const (e.g. `capability::core::STORAGE`) without explicit `id` → **compile
/// error** (`requires a string literal value or an explicit id = "…" attribute`).
///
/// Unknown keys → **compile error**.
///
/// # Host catalog
///
/// Ids should match `portaki_sdk::capability::{core,external,ai}::*` constants, which mirror the
/// Java `Capability` enum. `portaki lint` rejects unknown ids via `capability::is_known`.
///
/// # Emission
///
/// `capability-{id}.json` → `manifest.capabilities.required[]` or `.optional[]` (with
/// `purposeKey` / `fallbackKey` for optional entries).
#[proc_macro_attribute]
pub fn capability(attr: TokenStream, item: TokenStream) -> TokenStream {
    capability::expand(attr, item)
}

/// Declares use of a **built-in** platform connector (manifest reference only).
///
/// # Syntax
///
/// ```ignore
/// #[portaki_sdk::connector(builtin = "open-weather")]
/// pub struct UsesOpenWeather;
/// ```
///
/// # Attributes
///
/// | Key | Required |
/// |-----|----------|
/// | `builtin = "…"` | yes — connector id from the host connector catalog |
///
/// Wrong key → **compile error** (`expected builtin = "…"`).
///
/// Accepts any `syn::Item` (struct, const, etc.); the item is passed through unchanged.
///
/// # Emission
///
/// `connector_builtin-{builtin}.json` → `manifest.connectors.builtin[]`.
#[proc_macro_attribute]
pub fn connector(attr: TokenStream, item: TokenStream) -> TokenStream {
    connector::expand_builtin(attr, item)
}

/// Declares a **custom** HTTP connector implemented by the module.
///
/// # Syntax
///
/// ```ignore
/// #[portaki_sdk::custom_connector(
///     id = "open-weather",
///     display_name_key = "connector.openWeather.name",
///     base_url = "https://api.openweathermap.org",
///     credential_provider_id = "open-weather"
/// )]
/// pub struct ModuleOpenWeather;
/// ```
///
/// # Attributes
///
/// | Key | Required |
/// |-----|----------|
/// | `id = "…"` | **yes** |
/// | `display_name_key = "…"` | no |
/// | `base_url = "…"` | no |
/// | `credential_provider_id = "…"` | no |
///
/// Missing `id` → **compile error**. Unknown keys → **compile error**.
///
/// Must decorate a **struct** (metadata-only marker; runtime uses generic egress from manifest).
///
/// # Emission
///
/// `connector_custom-{id}.json` → `manifest.connectors.custom[]` (operations filled by
/// [`connector_op`] emissions that follow in source order).
#[proc_macro_attribute]
pub fn custom_connector(attr: TokenStream, item: TokenStream) -> TokenStream {
    connector::expand_custom(attr, item)
}

/// Declares one HTTP operation on the most recently emitted custom connector, or a validator stub.
///
/// # Syntax
///
/// ```ignore
/// impl ModuleOpenWeather {
///     #[portaki_sdk::connector_op(method = "GET", path = "/data/2.5/weather")]
///     pub fn current() {}
///
///     #[portaki_sdk::connector_op(method = "GET", path = "/data/2.5/forecast", cache = "5m")]
///     pub fn forecast() {}
/// }
///
/// // Validator-only stub (no method/path):
/// #[portaki_sdk::connector_op(validator)]
/// pub fn validate_credentials() {}
/// ```
///
/// # Attributes
///
/// | Key | Required | Notes |
/// |-----|----------|-------|
/// | `method = "…"` | for HTTP ops | e.g. `"GET"` |
/// | `path = "…"` | for HTTP ops | URL path on `base_url` |
/// | `cache = "…"` | no | cache hint (opaque string) |
/// | `validator` | bare flag | sets `"validator": true`, ignores method/path |
///
/// Unknown keys → **compile error**. Empty attribute list → HTTP op with null method/path.
///
/// `portaki build` appends each op to the **last** `connector_custom` entry's `operations` array,
/// using the Rust `fn` symbol as operation `id`.
///
/// # Emission
///
/// `connector_op-{fn_name}.json` with `{ fn, method, path, cache, validator }`.
#[proc_macro_attribute]
pub fn connector_op(attr: TokenStream, item: TokenStream) -> TokenStream {
    connector::expand_op(attr, item)
}
