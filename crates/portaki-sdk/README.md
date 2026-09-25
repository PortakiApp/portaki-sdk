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

An old KV blob in another shape (a key renamed, a number now a string, a nested object):
`#[portaki_sdk::config(legacy = legacy::read)]`, with `fn read(old: Value) -> Value` returning an
object of the declared keys — or `-> Result<Value, E>` when an old blob may not map: the error
fails the import, which the platform retries, where a panic would kill the call. The platform imports that result once (`legacyConfig`), and `load`
reads the KV through it until then. Once the import is stored, the platform calls
`legacyConfigAdopted`, which deletes the KV `config` — and the other old keys named by
`#[portaki_sdk::config(legacy_keys = ["texts/fr", "texts/en"])]`.

Labels are i18n keys, translated from `i18n/*.json` by `portaki build`. Keep
`publishReadiness` for conditional rules the schema cannot say ("a code once the smart lock is
off"). Tests: `MockContext::host().with_config(&config)`.

### Translated text

A text the guest reads in their language is an `I18nText` (`contracts::i18n`): the field is
`localized`, the platform keeps one text per language, and a save from the host form writes
the host's language only — the other languages stay. In a list, put `#[portaki_sdk::params]` on
the row type: `portaki build` reads its `I18nText` fields into `item.localized`, and a field named
`id` (or `#[field(item_id = "…")]`) into `item.id`, so each row keeps what the form did not send.
A row field marked `#[field(secret)]` (an iCal URL, a door code) goes to `item.secret`: encrypted
at rest, and kept when the form sends it back empty or masked; `null` clears it.

```rust,ignore
#[portaki_sdk::params]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Step {
    pub id: String,          // item.id — rows survive a reorder or a removal
    pub title: I18nText,     // item.localized
    pub ends_at: Option<String>,
    #[field(secret)]
    pub code: String,        // item.secret
}

#[portaki_sdk::config]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    #[field(required, label = "config.welcome")]
    pub welcome: I18nText,   // "type": "localized"
    #[field(label = "config.steps")]
    pub steps: Vec<Step>,    // "item": { "id": "id", "localized": ["title"], "secret": ["code"] }
}

// Host form: the editor's language, else fr, en, any. Guest: the guest's, else the property's
// default language (`context.propertyLang`), else fr, en, any.
TextInput::new().name("welcome").value(config.welcome.host_value(&ctx));
let shown = config.welcome.for_ctx(&ctx);
```

In the host form, each row of the list sends its id back with
`sdui::row_id("steps", index, Some(&step.id))` — a hidden `steps.<index>.id`, a fresh UUID for a
new row — so a removal or a reorder does not shift the other rows' languages.

`I18nText` also reads a plain string (a config saved before it was translated) as the same
text in every language; `is_blank()` is the platform's notion of empty.

## Texts from the bundles

An SDUI text is an `i18n:` key the shell translates. A text that needs variables — or that
leaves the booklet (a stats tile, a task, an email) — is resolved in the module:

```rust,ignore
// Every i18n/*.json and email_i18n/*.json of the crate, found at compile time; a literal key
// missing from all of them does not compile.
let tile = bundle_text!("stats.tile.title");                        // I18nText, every language
let body = bundle_text!("guest.nights", &[("n", &nights.to_string())]);
Text::new().text(body.for_ctx(&ctx));                                // the reader's language
let subject: LocalizedEmailText = bundle_text!("email.subject").into();

ctx.lang();          // "fr" for fr-FR — no lang_code() of your own
ctx.property_lang(); // Some("en"): the property's default language
```

`t!("key", n = 3)` still asks the host to translate into the request language.

Dates and durations come written, in the property's time (`host::time`):

```rust,ignore
use portaki_sdk::host::time;

let lang = ctx.lang();
let tz = ctx.property_tz();                         // None for a zone the SDK does not know
let local = tz.map_or(checkout.fixed_offset(), |tz| tz.to_local(checkout));
time::short_date(local, &lang);                     // "12 sept." / "Sep 12"
time::date_time(local, &lang);                      // "12 sept. 2026 · 10:00"
time::ago(last_sync, time::now()?, &lang);          // "il y a 3 h" / "3 h ago"
time::elapsed(last_sync, now, &lang);               // "3 h" — a tile value
time::weekday_name(local.weekday(), &lang);         // "samedi"
```

## Secrets revealed in time

A code or a password the guest sees only from a moment of the stay: `portaki_sdk::reveal`.

```rust,ignore
use portaki_sdk::reveal::RevealPolicy;

// Settings: #[field(kind = "select", options = ["always", "hours_before_24", "day_before_16h",
// "at_checkin"], label = "…")] pub reveal_policy: RevealPolicy,
RevealPolicy::choice_list("reveal_policy", config.reveal_policy, &ctx.lang()); // host form

let decision = config.reveal_policy.evaluate_for(&ctx, time::now()?); // check-in, property tz
Text::new().text(decision.show(&config.door_code));                   // or "••••••"
if let Some(why) = decision.locked_message(&ctx) { /* "Disponible à partir du 19 juil. · 16:00" */ }
```

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
