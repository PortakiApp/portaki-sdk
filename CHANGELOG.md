# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [2.0.0](https://github.com/PortakiApp/portaki-sdk/compare/v1.0.0...v2.0.0) (2026-07-24)


### ⚠ BREAKING CHANGES

* **sdk:** SDUI props, CapabilityId, OverlayArgs, EmailTemplateKey, and Action/event builders require typed IDs (SurfaceId/OperationName/ModuleId/EventType). Bare &str dropped in 2.1.0; EmptyArgs/json_value replace json! authoring.

### Features

* **host:** add email.send for module-owned mail ([106cde1](https://github.com/PortakiApp/portaki-sdk/commit/106cde1fd9905baaf5dca9e7913cb782834be4e7))
* **schema:** document stay-action host surface ([02a10eb](https://github.com/PortakiApp/portaki-sdk/commit/02a10eb1bc606930394f47b8d7c8d20d00f8d356))
* **sdk:** add Nuki connector and auth attribute ([1f442ee](https://github.com/PortakiApp/portaki-sdk/commit/1f442eef8f3c787e1a4eec04a054f263f0ffb2cb))
* **sdk:** add wire macro and command helpers ([778fcd2](https://github.com/PortakiApp/portaki-sdk/commit/778fcd202bcc813d7861574de17e59c9b2ccf558))
* **sdk:** mock extra capability ids for tests ([4362d27](https://github.com/PortakiApp/portaki-sdk/commit/4362d272739c5e5dfed5c5535980bbeea22c5f61))
* **sdk:** ship typed SDK 2.x boundary APIs ([cbeec7e](https://github.com/PortakiApp/portaki-sdk/commit/cbeec7e5f77ef8b262e883d612b284ab947c4733))
* **wire:** inject Debug and Clone derives ([6178770](https://github.com/PortakiApp/portaki-sdk/commit/617877085d471e773cd2524896d884fb1c722145))

## [Unreleased]

## [2.1.0](https://github.com/PortakiApp/portaki-sdk/compare/v2.0.1...v2.1.0) (2026-07-23)


### ⚠ BREAKING CHANGES

* **ids:** boundary builders no longer accept bare `&str` / `String` where a
  typed id exists. Use [`SurfaceId`], [`OperationName`], [`ModuleId`],
  [`EventType`], [`CapabilityId`], [`NavigateTarget`].
* **action:** `Action::command(module_id, name, args)` takes `&ModuleId` +
  [`OperationName`] (not `impl Into<String>`).
* **action:** `Action::open_overlay(..., surface_render, ...)` takes
  [`SurfaceId`] only.
* **action:** `Action::navigate(to, params)` takes
  [`NavigateTarget`] / [`SurfaceId`] (via `From`) — not free `String`.
  Dynamic shell routes use `NavigateTarget::path(...)`.
* **action:** `Action::emit(event, payload)` takes [`EventType`] only.
* **surface:** `Surface::with_id` takes [`SurfaceId`] only.
* **host:** `events::emit` takes [`EventType`] only.
* **host:** `module::list_by_capability` and `capabilities::has` take
  [`CapabilityId`] only.
* **context:** `Context::has_capability` takes [`CapabilityId`] only;
  `Context::module_id` is [`ModuleId`].
* **ids:** removed `From<&str>` / `From<String>` for [`ModuleId`]. Construct
  with `ModuleId::new` / `ModuleId::from_static` at declaration / test sites.

### Features

* **ids:** newtypes [`SurfaceId`], [`OperationName`], [`ModuleId`],
  [`EventType`] (serde string wire) plus
  `define_surface_ids!` / `define_operation_names!` / `define_event_types!`
* **ids:** shared booklet conventions under [`ids::convention`]
  (`HOME_CARD`, `EXPLORE_DETAIL`, `HOST_MAIN`)
* **action:** [`NavigateTarget`] (`Surface` | `Path`) for typed navigation
* **contracts:** SDK-owned cross-module catalogs —
  `contracts::smart_lock` (capability + `unlock` / `getGuestCredential`),
  `contracts::shell::SURFACE_INPUT`, `contracts::platform::BOOKING_CONFIRMED`
* **macros:** `#[surface]` / `#[command]` / `#[query]` / `#[event_handler]`
  accept `Type::new("…")` wire literals in addition to bare `"…"`

### Documentation

* **docs:** [typed-ids.md](docs/typed-ids.md) — declare once, typed consts at
  every use site
* **docs:** [module-layout.md](docs/module-layout.md) — SDK crate modules
  and Wasm module `guest/` / `host/` / `connectors` / `ids` conventions
* **templates:** empty-module ships `ids.rs` + layout notes aligned with
  guest/host/`ids` conventions

### Refactor

* **organization:** default module template splits guest / host surfaces and
  documents `ids.rs` catalogs (see module-layout)

## [2.0.1](https://github.com/PortakiApp/portaki-sdk/compare/v2.0.0...v2.0.1) (2026-07-23)


### Features

* **action:** `Action::command` takes `impl Serialize` (typed DTOs / [`EmptyArgs`])
* **action:** add [`EmptyArgs`] (`{}`) and [`json_value`] for navigate/emit payloads

## [2.0.0](https://github.com/PortakiApp/portaki-sdk/compare/v1.0.0...v2.0.0) (2026-07-23)


### ⚠ BREAKING CHANGES

* **sdui:** generated primitive props are typed (`String`, `bool`, `f64`/`u32`,
  `Action`, closed enums, nested structs). Scalar / action setters no longer
  accept `serde_json::Value` — drop `json!` on the common authoring path.
* **capability:** `capability::*` constants are now [`CapabilityId`] (serde
  string wire). `Context::with_capabilities` takes `&[CapabilityId]`. Manifest
  `capabilities.required` / `optional[].id` / `provided` deserialize as
  `CapabilityId`.
* **action:** `Action::OpenOverlay.presentation` is [`OverlayPresentation`]
  (not a raw string). Prefer `Action::open_overlay(...)`.
* **action:** `Action::OpenOverlay.args` is [`OverlayArgs`] (not
  `serde_json::Value`). Prefer `OverlayArgs::new().icon(...).title(...)`.
* **email:** guest-stay modules should filter on [`EmailTemplateKey`] instead of
  ad-hoc template strings.

### Features

* **capability:** add closed `CapabilityId` catalog with `as_str` / `FromStr`
* **email:** add `EmailTemplateKey`, `EmailContextArgs`, contribution docs
* **sdui:** typed codegen from `sdui_primitives.json` (`fields` map)
* **sdui:** nested types — `MapViewport`, `MapMarker`, `ChoiceOption`,
  `TemperatureUnit`, `RichTextDoc`, animation / visibility enums
* **action:** `OverlayPresentation`, `OverlayArgs`, `Action::open_overlay`

## [1.0.0](https://github.com/PortakiApp/portaki-sdk/compare/v0.2.1...v1.0.0) (2026-07-21)


### ⚠ BREAKING CHANGES

* **sdk:** host::credentials, images, notifications, repo::update,

### Features

* **capability:** add ai.guest.assistant ([2175387](https://github.com/PortakiApp/portaki-sdk/commit/2175387fc8f9123ff7565c400f28d37315b232f7))
* **cli:** emit operations.bundle v2 schema ([9b66db3](https://github.com/PortakiApp/portaki-sdk/commit/9b66db3bf96f4a7a85da47b6f96fd8db2bcfba5a))
* **connectors:** enrich OpenWeather current and forecast ([7b7f745](https://github.com/PortakiApp/portaki-sdk/commit/7b7f745c6645812bea9a05bef6b4043226ecb7e1))
* **connectors:** expose precip chance and wind speed ([78f7f85](https://github.com/PortakiApp/portaki-sdk/commit/78f7f85630cffdb4cf2f8257d75f14c7eb93031a))
* **sdk:** add provided caps and listByCapability ([c81a1b7](https://github.com/PortakiApp/portaki-sdk/commit/c81a1b75c9de9f84b72c08ddb70414a69ab25153))
* **sdk:** add StayContext for guest reveal ([c83d079](https://github.com/PortakiApp/portaki-sdk/commit/c83d07977c95f19d38ae496c1cb8f602d15f6f3a))
* **sdk:** pass host params as Context.input ([f8c8273](https://github.com/PortakiApp/portaki-sdk/commit/f8c82732933ebfb3ae8fc82af10c6fd5ca11777d))
* **sdk:** remove stub host APIs ([6f271f5](https://github.com/PortakiApp/portaki-sdk/commit/6f271f5fc95f6e47fe3e0751c38046e89f592aae))
* **sdui:** add Card.subtitle and ChoiceList.layout ([6bfed5e](https://github.com/PortakiApp/portaki-sdk/commit/6bfed5e953f981cce0e20dca1831d81dfb2a2674))
* **sdui:** add host form primitives ([da8e9fb](https://github.com/PortakiApp/portaki-sdk/commit/da8e9fbdb8f8b29e87844db4390a8afa5c1d06e2))
* **sdui:** add Stack/Grid/Card layout fields ([d757331](https://github.com/PortakiApp/portaki-sdk/commit/d757331e779cac137ece3cb48859b9aa51eb68e9))
* **sdui:** extend guest primitives for booklet redesign ([a80793f](https://github.com/PortakiApp/portaki-sdk/commit/a80793f1e3d72aeae6e903c1263ae39fe9242043))


### Bug Fixes

* **connectors:** use checked_div for humidity avg ([8ee2291](https://github.com/PortakiApp/portaki-sdk/commit/8ee2291f10f58cf3c617544436749f63109644f6))
* **wasm:** read property lat/lng from configJson ([c93479d](https://github.com/PortakiApp/portaki-sdk/commit/c93479d7ad22f71c047ac5c6d12b5b895228d14d))

## [0.2.1](https://github.com/PortakiApp/portaki-sdk/compare/v0.2.0...v0.2.1) (2026-07-15)


### Bug Fixes

* **publish:** drop sdk↔test-utils publish cycle ([0eb8e16](https://github.com/PortakiApp/portaki-sdk/commit/0eb8e1634600e6ccb905ecd4ce6feffcbed740d2))

## [0.2.0](https://github.com/PortakiApp/portaki-sdk/compare/v0.1.0...v0.2.0) (2026-07-15)


### Features

* **host:** add module.status readiness snapshot ([4ca2e6b](https://github.com/PortakiApp/portaki-sdk/commit/4ca2e6b56a50b2286f710bc780b47f54e4969faa))


### Bug Fixes

* **ci:** rustfmt auth + add quality and release-please ([5b0038a](https://github.com/PortakiApp/portaki-sdk/commit/5b0038af6624514d86b9b61999cf3a7a6987c6f3))
* **deps:** drop invalid Renovate rustMonorepo preset ([65b6357](https://github.com/PortakiApp/portaki-sdk/commit/65b63573bd2474c56a4a6a3a63bf323e9a1d2f25))
* **docs:** indent rustdoc list continuation for clippy ([bb88ee5](https://github.com/PortakiApp/portaki-sdk/commit/bb88ee571e6ed7616df5f4372767a8aff5125a73))

## [0.1.0]

### Features

- Initial open-source SDK workspace (host functions, SDUI, connectors, CLI)
- `host::module::status` readiness snapshot for Wasm modules
