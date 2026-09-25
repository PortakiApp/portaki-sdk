<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://portaki.app/logo-dark.svg">
    <img src="https://portaki.app/logo-light.svg" width="177" height="48" alt="Portaki">
  </picture>
</p>

<h1 align="center">portaki-test-utils</h1>

<p align="center">
  <strong>In-process mock host for Portaki module unit tests</strong><br>
  <code>MockContext</code>, in-memory host functions, and SDUI assertions — no Wasm, no Extism.
</p>

<p align="center">
  <a href="https://crates.io/crates/portaki-test-utils"><img src="https://img.shields.io/crates/v/portaki-test-utils.svg" alt="crates.io"></a>
  <a href="https://docs.rs/portaki-test-utils"><img src="https://img.shields.io/docsrs/portaki-test-utils" alt="docs.rs"></a>
  <a href="https://github.com/PortakiApp/portaki-sdk/blob/main/LICENSE"><img src="https://img.shields.io/badge/license-Apache--2.0-blue.svg" alt="License Apache-2.0"></a>
  <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/Rust-1.75+-dea584?logo=rust&logoColor=white" alt="Rust 1.75+"></a>
  <a href="https://portaki.app"><img src="https://img.shields.io/badge/site-portaki.app-f59e0b" alt="portaki.app"></a>
</p>

<p align="center">
  <a href="#install">Install</a> ·
  <a href="#quick-start">Quick start</a> ·
  <a href="#conformance-battery">Conformance</a> ·
  <a href="#what-you-get">What you get</a> ·
  <a href="#documentation">Docs</a> ·
  <a href="#license">License</a>
</p>

---

Dev-dependency for module crates. Production code calls `portaki_sdk::host::*`; this crate supplies a [`HostBackend`](https://docs.rs/portaki-sdk/latest/portaki_sdk/host/trait.HostBackend.html) that stays entirely in memory on the test thread.

## Install

```toml
[dev-dependencies]
portaki-sdk = "0.1"
portaki-test-utils = "0.1"
```

## Quick start

```rust,ignore
use portaki_test_utils::{MockContext, Property, SurfaceAssertions};

#[test]
fn guest_home_renders() {
    MockContext::guest()
        .with_property(Property::default())
        .with_capabilities(&["core.storage"])
        .run(|ctx| {
            let surface = guest_home(ctx).expect("render");
            let tree = SurfaceAssertions::new(&surface);
            // Every primitive of the SDUI contract is walked: no hand-written
            // `contains_component_type` helper needed.
            assert!(tree.contains_type("Card"));
            assert_eq!(tree.count_type("ListItem"), 3);
        });
}
```

Stub connectors used by [`portaki-connectors`](https://crates.io/crates/portaki-connectors):

```rust,ignore
MockContext::guest()
    .with_connector_response(
        "open-weather",
        "current",
        r#"{"main":{"temp":21.5},"weather":[{"main":"Clear"}]}"#,
    )
    .run(|_ctx| { /* OpenWeather::current reads the stub */ });
```

## Conformance battery

Every module runs the same checks, from one file — `tests/conformance.rs`:

```rust,ignore
portaki_test_utils::conformance!();
```

It generates one test per check under `portaki_conformance::`. `portaki publish` runs them and refuses to publish while one fails.

| Test | Fails when |
|------|------------|
| `manifest` | the manifest — `portaki.module.json` if kept, else the one `portaki build` wrote — does not validate against `module.v1.json` (bundled, no network); an `I18nText` config field is not `localized` there, a row's `I18nText` fields are not its `item.localized` (its `#[field(secret)]` fields not its `item.secret`), or `item.localized` / `item.id` name a sub-key the row type (`#[params]`) does not have |
| `listing` | `listing.json` is there and does not validate against `listing.v1.json` (bundled, no network), or still holds the `portaki init` instructions — no `listing.json` passes, the listing can be written in the dashboard |
| `surfaces` | a `#[surface]` panics or errors with an empty mock in its shell (a guest `Err` the SDK shows as its error state included), sends a tree that does not parse as contract primitives or holds a `Select` without options or with a `value` outside them, or a `guestSurfaces[].surfaceId` has no guest surface; a guest surface panics, fails or renders nothing to read with the module inactive, incomplete, or `host::module::status` failing |
| `operations` | a `#[command]` or `#[query]` panics on `{}` in a guest or host mock (an `Err` is fine) |
| `i18n` | a key used by `guestSurfaces[].labelKey`, a rendered `"i18n:…"` string or `host::i18n::translate` is missing from the `fr` or `en` bundle in `i18n/` (the guest-state keys the SDK ships a text for aside); a `config.fields[]` label, description or option label has no text in one of the languages of `i18n/` |
| `emails` | an `emails[]` command is not declared or panics around a mock stay, `emailContext` panics for a template key, or a variable `#[email_vars]` declares does not come back on the module's fixture (`conformance!(email_fixture = …)`) |
| `contracts` | a `property-stats-card` has no `statsSummary`, or it answers off `stats-summary.v1.json` (`fr`/`en`, `value` ≤ 12 chars) or slower than 300 ms; a `property-stats-detail` has no host surface of id `pathSegment`, or it fails with `input.periodDays`; a `workspace-timeline-task` has no `timelineTasks`, or it answers off `timeline-tasks.v1.json` on three fixture stays (ISO dates, non-empty items), or `taskToggle` ticks a `photoRequired` item without a photo instead of refusing it with `photo_required`; an exported `publishReadiness` answers off `publish-readiness.v1.json` |

The battery finds handlers through the `HandlerDeclaration`s that `#[query]`, `#[command]` and `#[surface]` register on native targets: nothing to list by hand. It needs `portaki-sdk-macros` from the same release. Not checked: the sandbox clock (`Utc::now()` runs natively — use clippy's `disallowed-methods`), `portaki_module!` display keys, and `#[event_handler]`s.

## Catalogue previews

`previews.json` — every guest surface the booklet serves (`#[surface(guest, path = …)]`), rendered
on sample data for the module's public page — is written and checked by `previews`:

```rust,ignore
// tests/previews.rs
use portaki_test_utils::previews;

portaki_test_utils::link_module!();

#[test]
fn previews_match_the_rendered_surfaces() {
    let root = env!("CARGO_MANIFEST_DIR");
    previews::check_all(root, previews::guest(root).with_config(&sample_config()));
}
```

`previews::guest` is a French guest, a stay from 1 to 8 June 2026, the clock the day before;
`PORTAKI_UPDATE_PREVIEWS=1 cargo test --test previews` rewrites the file. `previews::check` takes
surfaces rendered by hand instead.

## What you get

| Type | Role |
|------|------|
| `MockContext` / `MockContextBuilder` | Fluent guest/host context + backend install; `with_config(&cfg)` sets the install's config (`#[portaki_sdk::config]`; `I18nText` values included, as the platform stores them), `with_module_status(…)` what `host::module::status` answers (`incomplete: true`…), `with_module_status_error(…)` a status that fails, `with_coordinates(None)` a property not geocoded; `run_with` hands over the backend, whose `logs()` lists what the module logged |
| `MockHostFunctions` | In-memory KV, i18n, connectors, repo stubs; enforces the platform's per-invocation email / event caps and the after-stay email rule (`with_stay`, `with_now`, `sent_emails`) |
| `Property`, `Booking`, … | Default fixtures |
| `SurfaceAssertions` | Depth-first SDUI queries over every primitive: `contains_type("Card")`, `count_type`, `find::<Card>()`, `count::<Card>()`, … |
| `conformance!` / `conformance::Module` | The shared battery: manifest, listing, surfaces, operations, i18n, emails, contracts |

## Documentation

- API — [docs.rs/portaki-test-utils](https://docs.rs/portaki-test-utils)
- Workspace — [`PortakiApp/portaki-sdk`](https://github.com/PortakiApp/portaki-sdk)

## License

[Apache-2.0](https://github.com/PortakiApp/portaki-sdk/blob/main/LICENSE) · Copyright 2026 Syntax Labs
