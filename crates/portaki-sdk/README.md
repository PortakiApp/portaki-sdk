<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://portaki.app/logo-dark.svg">
    <img src="https://portaki.app/logo-light.svg" width="177" height="48" alt="Portaki">
  </picture>
</p>

<h1 align="center">portaki-sdk</h1>

<p align="center">
  <strong>Authoring toolkit for Portaki Extism Wasm guest modules</strong><br>
  Host APIs, SDUI catalog, capability constants, and re-exported proc-macros.
</p>

<p align="center">
  <a href="https://crates.io/crates/portaki-sdk"><img src="https://img.shields.io/crates/v/portaki-sdk.svg" alt="crates.io"></a>
  <a href="https://docs.rs/portaki-sdk"><img src="https://img.shields.io/docsrs/portaki-sdk" alt="docs.rs"></a>
  <a href="https://github.com/PortakiApp/portaki-sdk/blob/main/LICENSE"><img src="https://img.shields.io/badge/license-Apache--2.0-blue.svg" alt="License Apache-2.0"></a>
  <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/Rust-1.75+-dea584?logo=rust&logoColor=white" alt="Rust 1.75+"></a>
  <a href="https://extism.org/"><img src="https://img.shields.io/badge/Extism-Wasm-7C3AED" alt="Extism"></a>
  <a href="https://portaki.app"><img src="https://img.shields.io/badge/site-portaki.app-f59e0b" alt="portaki.app"></a>
</p>

<p align="center">
  <a href="#install">Install</a> ·
  <a href="#quick-start">Quick start</a> ·
  <a href="#what-you-get">What you get</a> ·
  <a href="#workspace">Workspace</a> ·
  <a href="#documentation">Docs</a> ·
  <a href="#license">License</a>
</p>

---

This is the crate module authors depend on day to day. Proc-macros are re-exported from [`portaki-sdk-macros`](https://crates.io/crates/portaki-sdk-macros); the `portaki` CLI merges their compile-time emissions into `manifest.json` at build time.

## Install

```toml
[dependencies]
portaki-sdk = "0.1"
```

Guest builds target `wasm32-unknown-unknown`:

```bash
rustup target add wasm32-unknown-unknown
```

## Quick start

```rust,ignore
use portaki_sdk::prelude::*;

portaki_module!(
    id = "weather",
    name = "Weather",
    description = "Current weather and forecast",
);

#[surface(id = "guest.home", audience = "guest")]
fn guest_home(_ctx: &Context) -> Result<Surface> {
    Ok(Surface::new("guest.home"))
}
```

```bash
cargo install portaki-cli
portaki build --release
portaki lint
```

## Host configuration

Declare the settings on a struct; the platform validates what the host saves, encrypts secrets,
blocks publication while a `required` field is empty and hands the config back on every call.
No `updateConfig` command, no KV key, no `publishReadiness` for an empty field.

```rust,ignore
#[portaki_sdk::config]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    #[field(required, label = "config.ssid")]
    pub ssid: String,
    #[field(secret, recommended, label = "config.password")]
    pub password: String,
    #[field(structured, label = "config.contacts")]
    pub contacts: Vec<Contact>,
}

let config = Config::load(&ctx)?; // context.moduleConfig; KV `config` only when none is sent
```

Labels are i18n keys, translated from `i18n/*.json` by `portaki build`. Keep
`publishReadiness` for conditional rules the schema cannot say ("a code once the smart lock is
off"). Tests: `MockContext::host().with_config(&config)`.

### Translated text

A text the guest reads in their language is an `I18nText` (`contracts::i18n`): the field is
`localized`, the platform keeps one text per language, and a save from the host form writes
the host's language only — the other languages stay. In a list, put `#[portaki_sdk::params]` on
the row type: `portaki build` reads its `I18nText` fields into `item.localized`, and a field named
`id` (or `#[field(item_id = "…")]`) into `item.id`, so each row keeps what the form did not send.

```rust,ignore
#[portaki_sdk::params]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Step {
    pub id: String,          // item.id — rows survive a reorder or a removal
    pub title: I18nText,     // item.localized
    pub ends_at: Option<String>,
}

#[portaki_sdk::config]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    #[field(required, label = "config.welcome")]
    pub welcome: I18nText,   // "type": "localized"
    #[field(label = "config.steps")]
    pub steps: Vec<Step>,    // "item": { "id": "id", "localized": ["title"] }
}

// Host form: the editor's language, else fr, en, any. Guest: the guest's.
TextInput::new().name("welcome").value(config.welcome.host_value(&ctx));
let shown = config.welcome.get(&ctx.locale);
```

`I18nText` also reads a plain string (a config saved before it was translated) as the same
text in every language; `is_blank()` is the platform's notion of empty.

## Guest surfaces: the happy path only

A guest surface does not check whether the module is ready, nor catch its own errors:

```rust,ignore
#[portaki_sdk::surface(guest, id = "home.card")]
pub fn render_home_card(ctx: GuestContext) -> Result<Surface> {
    let config = Config::load(&ctx)?;
    Ok(Surface::new(/* … */))
}
```

The SDK renders « inactive » (module off) or « incomplete » (a required field empty) without
calling it, and an `Err` as a logged error state (`<module>_<surface>_render_failed`), with the
`portaki_module!` icon. Their texts come with the SDK in en, fr, es, de, it and nl; a key of the
same name in the module's bundle overrides one (`module.status.inactive.*`,
`module.status.incomplete.*`, `guest.error.*`). No `guest/empty.rs`. Reading the status takes the
`platform` feature; `gate = false` opts a surface out. See `portaki_sdk::guest_shell`.

## Where the property is

`ctx.property.coordinates` is `Option<GeoPoint>`: `None` while the property is not geocoded —
render the empty state, do not guess. `ctx.property.lat` / `lng` are deprecated: `0.0` then (they
used to read Paris). Tests: `MockContext::guest().with_coordinates(None)`.

## What you get

| Surface | Role |
|---------|------|
| `host::*` | Typed host wrappers — KV, repo, connectors, events, i18n, … |
| `sdui::*` | Surfaces, components, actions the shell can render |
| `capability::*` | Capability ids checked by the orchestrator and `portaki lint` |
| Proc-macros | `portaki_module!`, `#[surface]`, `#[query]`, `#[command]`, … |

## Workspace

| Crate | Role |
|-------|------|
| **portaki-sdk** | Runtime APIs + SDUI (this crate) |
| [`portaki-sdk-macros`](https://crates.io/crates/portaki-sdk-macros) | Manifest emissions at compile time |
| [`portaki-connectors`](https://crates.io/crates/portaki-connectors) | Typed built-in connector ops |
| [`portaki-test-utils`](https://crates.io/crates/portaki-test-utils) | In-process mock host for tests |
| [`portaki-cli`](https://crates.io/crates/portaki-cli) | `portaki` binary |

Monorepo: [`PortakiApp/portaki-sdk`](https://github.com/PortakiApp/portaki-sdk).

## Documentation

- API — [docs.rs/portaki-sdk](https://docs.rs/portaki-sdk)
- Module / crate layout — [module-layout.md](https://github.com/PortakiApp/portaki-sdk/blob/main/docs/module-layout.md)
- Typed boundary ids — [typed-ids.md](https://github.com/PortakiApp/portaki-sdk/blob/main/docs/typed-ids.md)
- Connectors & credentials — [guide](https://github.com/PortakiApp/portaki-sdk/blob/main/docs/connectors-and-credentials.md)
- Releases — [RELEASE.md](https://github.com/PortakiApp/portaki-sdk/blob/main/docs/RELEASE.md)

## License

[Apache-2.0](https://github.com/PortakiApp/portaki-sdk/blob/main/LICENSE) · Copyright 2026 Syntax Labs
