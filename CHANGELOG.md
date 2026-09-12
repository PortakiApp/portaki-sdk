# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [4.1.0](https://github.com/PortakiApp/portaki-sdk/compare/v4.0.0...v4.1.0) (2026-09-12)


### Features

* **cli:** add portaki dev --forget ([e462051](https://github.com/PortakiApp/portaki-sdk/commit/e4620511c293404e906a647b2e7502206c21faa9))
* **contracts:** describe the types the primitives only name ([f841209](https://github.com/PortakiApp/portaki-sdk/commit/f8412098e30f13931f87ad7eb73333d9036613b4))


### Bug Fixes

* **cli:** point both ways when the SDK versions disagree ([02b755c](https://github.com/PortakiApp/portaki-sdk/commit/02b755cdc35e65e61e1035a875b0ea3dcb8b73b2))

## [4.0.0](https://github.com/PortakiApp/portaki-sdk/compare/v3.2.0...v4.0.0) (2026-09-12)


### ⚠ BREAKING CHANGES

* **sdui:** BottomTabBar.tabs and Map.clustering go from serde_json::Value to Vec<TabBarItem> and MapClustering. A module building them by hand keeps working — the wire shape is unchanged.

### Features

* **cli:** add portaki check, the gate CI runs ([0f838d3](https://github.com/PortakiApp/portaki-sdk/commit/0f838d339104fe1522060a91d8b26a1f9efb157a)), closes [#120](https://github.com/PortakiApp/portaki-sdk/issues/120)
* **cli:** add portaki connectors, and make lint keep its word ([1340c62](https://github.com/PortakiApp/portaki-sdk/commit/1340c629cd9d295cd22619a21fb91722f3485648)), closes [#121](https://github.com/PortakiApp/portaki-sdk/issues/121)
* **cli:** give the scaffold a settings round trip ([7d8a33d](https://github.com/PortakiApp/portaki-sdk/commit/7d8a33da4277726cf3a0a7053f9c37eedcf67c75))
* **cli:** show a module's log lines in the trace ([e5b08e0](https://github.com/PortakiApp/portaki-sdk/commit/e5b08e07f5bd993731e80f47a3bac3332502ad3f))
* **sdui:** declare what the booklet actually renders ([3aafdb1](https://github.com/PortakiApp/portaki-sdk/commit/3aafdb1075d54751857f28e4e41a8c6b9c96a08d))
* **test-utils:** let a connector fail, and record what was sent ([035de27](https://github.com/PortakiApp/portaki-sdk/commit/035de2756a58782dc1744dc1ec5686f385a375e7)), closes [#119](https://github.com/PortakiApp/portaki-sdk/issues/119)


### Bug Fixes

* **cli:** embed the init templates in the binary ([9592d94](https://github.com/PortakiApp/portaki-sdk/commit/9592d94587a02cc77a371255c0d9973b6c2c2539)), closes [#113](https://github.com/PortakiApp/portaki-sdk/issues/113)
* **cli:** English help, and a registry the author owns ([b09b58f](https://github.com/PortakiApp/portaki-sdk/commit/b09b58fc4bfd11d6178ccee43c37531da188b766)), closes [#118](https://github.com/PortakiApp/portaki-sdk/issues/118)
* **cli:** look for the wasm of the profile just built ([c70a826](https://github.com/PortakiApp/portaki-sdk/commit/c70a826922d36accc16a15742c941b4b1d392969)), closes [#117](https://github.com/PortakiApp/portaki-sdk/issues/117)
* **cli:** make the dry run check what the push would send ([d9db0a6](https://github.com/PortakiApp/portaki-sdk/commit/d9db0a6161b6dc495031296575a92b5c45f398d0)), closes [#132](https://github.com/PortakiApp/portaki-sdk/issues/132)
* **cli:** make the scaffold build, test and deploy ([e3e99b6](https://github.com/PortakiApp/portaki-sdk/commit/e3e99b6992a3dfb53ede4a5567fea75782204045)), closes [#114](https://github.com/PortakiApp/portaki-sdk/issues/114)
* **cli:** print the catalogue of the linked SDK ([5307e1c](https://github.com/PortakiApp/portaki-sdk/commit/5307e1c03805b0e0355a1dd85e98e6353548c517))
* **cli:** scaffold into a directory that already exists ([4900e5f](https://github.com/PortakiApp/portaki-sdk/commit/4900e5f9fed929d88cb3f42478dc6c248a694933)), closes [#115](https://github.com/PortakiApp/portaki-sdk/issues/115)
* **macros:** emit the field type, not the whole field ([8286b48](https://github.com/PortakiApp/portaki-sdk/commit/8286b48ef26b4bd6db6e193fb61661f846d2542f)), closes [#116](https://github.com/PortakiApp/portaki-sdk/issues/116)

## [3.2.0](https://github.com/PortakiApp/portaki-sdk/compare/v3.1.1...v3.2.0) (2026-09-11)


### Features

* **sdk:** add a secondary tone ([6109362](https://github.com/PortakiApp/portaki-sdk/commit/61093622bbb31ef9de35ff825de13a4409ad78cc))
* **sdk:** add named swatches for content colors ([9d43196](https://github.com/PortakiApp/portaki-sdk/commit/9d431963362e2f6f265ed6d4e9b3d56ab117c2ed))

## [3.1.1](https://github.com/PortakiApp/portaki-sdk/compare/v3.1.0...v3.1.1) (2026-09-11)


### Bug Fixes

* **cli:** don't blame an upgrade for checks already failing ([7f232fc](https://github.com/PortakiApp/portaki-sdk/commit/7f232fc2105e590b858fc2d0fbcb342d5922881c))
* **cli:** report the resolved SDK after an upgrade ([361b3c7](https://github.com/PortakiApp/portaki-sdk/commit/361b3c7891effff098bcea53fdfcaf6ab205b1eb))

## [3.1.0](https://github.com/PortakiApp/portaki-sdk/compare/v3.0.1...v3.1.0) (2026-09-11)


### Features

* **cli:** add portaki sdk upgrade ([1f80687](https://github.com/PortakiApp/portaki-sdk/commit/1f80687e785d58bfa790237de6d99b8b67730804))
* **sdk:** describe operation arguments with #[params] ([c46d211](https://github.com/PortakiApp/portaki-sdk/commit/c46d2117058267075da513fa3f3f80aec85b94b8))


### Bug Fixes

* **cli:** build the manifest from the latest emissions ([6701a4a](https://github.com/PortakiApp/portaki-sdk/commit/6701a4a7e8f0ec13a82855b7289ea1313704b920))

## [3.0.1](https://github.com/PortakiApp/portaki-sdk/compare/v3.0.0...v3.0.1) (2026-09-11)


### Bug Fixes

* **cli:** send built operations to the sandbox ([f5f47b3](https://github.com/PortakiApp/portaki-sdk/commit/f5f47b317ee0582297489e3934295b84d3098749))

## [3.0.0](https://github.com/PortakiApp/portaki-sdk/compare/v2.6.0...v3.0.0) (2026-09-10)


### ⚠ BREAKING CHANGES

* **cli:** needs a platform serving /dev/v1/dev-watch. Against an older one the account-wide lease is unavailable and only the local lock applies.

### Bug Fixes

* **cli:** take the dev session lease from devapi ([5406a12](https://github.com/PortakiApp/portaki-sdk/commit/5406a12a9febff1d0b79a83e0944e1181ac02804))

## [2.6.0](https://github.com/PortakiApp/portaki-sdk/compare/v2.5.0...v2.6.0) (2026-09-10)


### Features

* **cli:** tenir la session --watch au nom du compte ([e343f80](https://github.com/PortakiApp/portaki-sdk/commit/e343f808cd8489d25f492a11d63dba8653efbb5e))
* **cli:** une seule session --watch a la fois ([70ba552](https://github.com/PortakiApp/portaki-sdk/commit/70ba552ad07a580f4304ed30ae14357e7beffee8))


### Bug Fixes

* **cli:** logout ferme la session sur la plateforme aussi ([1cb5e22](https://github.com/PortakiApp/portaki-sdk/commit/1cb5e22693c2401b8d60d3b9e59177de9d1c2990))
* **cli:** refuser de republier avant de pousser, pas apres ([4e75d27](https://github.com/PortakiApp/portaki-sdk/commit/4e75d2744ac5691e3feae9e2c6840cf6d95a7c58))
* **cli:** un dev ponctuel prend la place, lui aussi ([1cf1f7b](https://github.com/PortakiApp/portaki-sdk/commit/1cf1f7b27950f3220323f25fe654e969c7312744))
* **cli:** un refus qui dit ce qui manque, et ou le lire ([7845d8c](https://github.com/PortakiApp/portaki-sdk/commit/7845d8cd65e19d2be90a1f2319e9b60be6c2affd))

## [2.5.0](https://github.com/PortakiApp/portaki-sdk/compare/v2.4.0...v2.5.0) (2026-09-10)


### Features

* **cli:** dire qu'une version plus recente existe ([abd7688](https://github.com/PortakiApp/portaki-sdk/commit/abd7688e01ec51aad9bfc343b84867a616007691))


### Bug Fixes

* **cli:** une URL de plateforme vide vaut « non definie » ([3dcc12e](https://github.com/PortakiApp/portaki-sdk/commit/3dcc12e2c4613ad7b8960ea6cd11ffe6c0ce08c0))

## [2.4.0](https://github.com/PortakiApp/portaki-sdk/compare/v2.3.0...v2.4.0) (2026-09-10)


### Features

* **cli:** --plain, une sortie faite pour etre lue par un programme ([dc1aa9b](https://github.com/PortakiApp/portaki-sdk/commit/dc1aa9bb3b8f6f7334d6e04ca109130d7c1761b7))
* **cli:** le point en vert, et de l'air au-dessus du logo ([2146ad0](https://github.com/PortakiApp/portaki-sdk/commit/2146ad0ec0b5845424b26903d054f31837ae88b2))
* **cli:** portaki ci info, l'identite du module ([4019c44](https://github.com/PortakiApp/portaki-sdk/commit/4019c445640d8939fccf6449ab434370a0b91c24))
* **cli:** portaki ci, ce qu'un workflow faisait en bash ([b70990f](https://github.com/PortakiApp/portaki-sdk/commit/b70990f31992026a9387a9e0f924389e7c7ad35a))
* **sdk:** declarer ce que la plateforme retire ([cd1116e](https://github.com/PortakiApp/portaki-sdk/commit/cd1116e3f9c08a254a90328383e22da9b29c3478))


### Bug Fixes

* **cli:** lint refuse une crate et un manifeste qui divergent ([2de8657](https://github.com/PortakiApp/portaki-sdk/commit/2de8657feb7844a09f4210b5047296c3f20972d6))
* **template:** un module scaffolde qui compile hors du depot ([b5c3704](https://github.com/PortakiApp/portaki-sdk/commit/b5c3704c335bb269d04d6fca719c3ef1f723f503))

## [2.3.0](https://github.com/PortakiApp/portaki-sdk/compare/v2.2.0...v2.3.0) (2026-09-09)


### Features

* **cli:** le logo de la marque, en bas de casse ([c2ed131](https://github.com/PortakiApp/portaki-sdk/commit/c2ed13131e4b4665b2c432330c3ed2ac6bccebd8))


### Bug Fixes

* **cli:** transporter les surfaces emises par le build ([c17257a](https://github.com/PortakiApp/portaki-sdk/commit/c17257aaea0477f29ee3b89a84115d5232f984f0))

## [2.2.0](https://github.com/PortakiApp/portaki-sdk/compare/v2.1.1...v2.2.0) (2026-09-09)


### Features

* **cli:** `portaki login` annonce la machine qui demande ([b0d5cc0](https://github.com/PortakiApp/portaki-sdk/commit/b0d5cc09afc844e321ab759944af918ef8d30554))
* **cli:** annoncer la publication au registre ([a7715d8](https://github.com/PortakiApp/portaki-sdk/commit/a7715d80783b223aacef99bdd9f28621c2c6cb0b))
* **cli:** annoncer une version deja sur GHCR ([90cd7b8](https://github.com/PortakiApp/portaki-sdk/commit/90cd7b8d2598fee4f8750fd6be6511b9bff071d5))
* **cli:** demander un scope par geste, pas un fourre-tout ([6374b58](https://github.com/PortakiApp/portaki-sdk/commit/6374b58463e67cc78b022e4f3fcf4e2be709261f))
* **cli:** dire ce que fait chaque commande ([25dc59c](https://github.com/PortakiApp/portaki-sdk/commit/25dc59c1e01184166d8142f942f1d6d783d1adf6))
* **cli:** portaki dev builds, deploys and shows the run ([ec4cdce](https://github.com/PortakiApp/portaki-sdk/commit/ec4cdce2de0a71e1870ee9d44e2ebdd93c140eaa))
* **cli:** portaki login stores the token in the system keychain ([28c83aa](https://github.com/PortakiApp/portaki-sdk/commit/28c83aaeefa4f145bbdf599828ecf3d204062344))
* **cli:** publier depuis une CI sans secret ([23615f2](https://github.com/PortakiApp/portaki-sdk/commit/23615f26f133244853bf39c50e84093ab46cf406))
* **cli:** ranger les identifiants dans un fichier ([212d08a](https://github.com/PortakiApp/portaki-sdk/commit/212d08ad461c18ba133de73df25d1f2989bcb06b))
* **cli:** repondre « laquelle ? » au lieu de refuser ([78d3afa](https://github.com/PortakiApp/portaki-sdk/commit/78d3afa7557ca6028f98a347a5949b3039e3655e))
* **cli:** tamponner la version SDK liee au build ([08ec4bd](https://github.com/PortakiApp/portaki-sdk/commit/08ec4bd95ca8454ef6df99594a742b717fba2747))
* **cli:** un logo, et ce qui protege le projet ([51b3f43](https://github.com/PortakiApp/portaki-sdk/commit/51b3f43ac749698c23e7f16d7f4aa82d814a6ca5))
* **connectors:** add OpenAgenda nearby events client ([ab53d1f](https://github.com/PortakiApp/portaki-sdk/commit/ab53d1f9c2a1f66170905c86a012f1fd47fcd123))
* **context:** expose stay booking_channel to modules ([3ac7d48](https://github.com/PortakiApp/portaki-sdk/commit/3ac7d4816740a87da607eaec0dbcd38f42c3fc78))
* **contracts:** add booking channel vocabulary ([6e84543](https://github.com/PortakiApp/portaki-sdk/commit/6e845434dc761e912bb0ba747f05eaa245495da2))
* **contracts:** add shared StayImportRow shape ([c695a53](https://github.com/PortakiApp/portaki-sdk/commit/c695a539749830d7785e9328f6d7ac1bfe86c4f8))
* **host:** add host::notify + core.host.notifications capability ([a3b8715](https://github.com/PortakiApp/portaki-sdk/commit/a3b87157cc14acde7553892b2049589f9045743e))
* **schema:** add maturity and sortOrder fields ([411e626](https://github.com/PortakiApp/portaki-sdk/commit/411e62640a08cb2c6b2567ffed41b2bfbaa7eb87))
* **schema:** declare module permissions, rename SDK field ([f588b92](https://github.com/PortakiApp/portaki-sdk/commit/f588b92010e005dc2edf527ae8cc4c102cfee82b))
* **sdui:** add optional blurHash to Image ([5c4ce59](https://github.com/PortakiApp/portaki-sdk/commit/5c4ce598137e840390413ab710c95cec36717b0b))


### Bug Fixes

* **cli:** --watch surveille aussi le manifeste ([0d5aaaf](https://github.com/PortakiApp/portaki-sdk/commit/0d5aaaf3023e146bc6a314db09c2ac3e9db24743))
* **cli:** lire les reponses de devapi en camelCase ([54eba86](https://github.com/PortakiApp/portaki-sdk/commit/54eba86def30353ac87fd2d293ae9906eca9623a))
* **cli:** portaki dev ignorait PORTAKI_API_URL ([85fe671](https://github.com/PortakiApp/portaki-sdk/commit/85fe671a2575f402aa098f2afa2670c6eadbeb48))
* **cli:** portaki dev sur un module au nom composé ([c23f144](https://github.com/PortakiApp/portaki-sdk/commit/c23f144bdfae182dfeae728b417821bbfb91948b))
* **cli:** portaki dev tamponne la version du SDK ([505ab78](https://github.com/PortakiApp/portaki-sdk/commit/505ab7885329df7a7c3731b18b3574910cebbf66))
* **cli:** portaki nu ouvre l'aide, pas une croix ([b8f84cb](https://github.com/PortakiApp/portaki-sdk/commit/b8f84cb046e893d2a82922714cb8bc86b232048b))
* **cli:** read the platform envelope, renew on 401 ([f00bbcc](https://github.com/PortakiApp/portaki-sdk/commit/f00bbcc334c196cf58cc78a3dbb7cdc54ae8dd20))
* **cli:** une seule croix par echec ([f730c79](https://github.com/PortakiApp/portaki-sdk/commit/f730c79f48106837fa46b1f04c51303a9331b1ef))
* **deps:** update rust crate extism-pdk to 1.4.1 ([440ca31](https://github.com/PortakiApp/portaki-sdk/commit/440ca31322af04242dc0fe13e2bf290b7cb9004e))
* **deps:** update rust crate inventory to 0.3.24 ([706e9ec](https://github.com/PortakiApp/portaki-sdk/commit/706e9eccd83c705b4d2088d23bef17609e2f8c84))
* **deps:** update rust crate proc-macro2 to 1.0.107 ([8961164](https://github.com/PortakiApp/portaki-sdk/commit/896116410758a7c8e56f8bc7eeb3f65f7e3cb196))
* **deps:** update rust crate quote to 1.0.47 ([e07b7f8](https://github.com/PortakiApp/portaki-sdk/commit/e07b7f8637bc29f74433b01a443d27e98035b7f5))
* **deps:** update rust crate syn to 2.0.119 ([d0c2e85](https://github.com/PortakiApp/portaki-sdk/commit/d0c2e850747936619a48e46f885f50d230c25d0e))

## [Unreleased]

### Features

* **contracts:** add `booking_channel` — canonical `BookingChannel` /
  `ChannelSignal` vocabulary answering *who sold the stay* (vocabulary only, no
  decision table; behavioural attributes stay on the gateway)
* **contracts:** add `stay_import::StayImportRow` — canonical import row shape
  for `ModuleGatewayStayImportAdapter`, now carrying `bookingChannel` /
  `bookingChannelSignal` on every row
* **sdui:** add optional `icon` on `ToggleRow` (leading icon token)
* **sdui:** add `IndexedInput` (index + optional checkbox + text field tile)
* **sdui:** add `Grid.minColumnWidth` for auto-fit host grids

## [2.1.1] — 2026-07-25

### Features

* **email:** `LocalizedEmailText` multi-locale (`translations` map) + `resolve` /
  `from_i18n_key` helpers with guestLang → tag → en → fr fallback (wire-compatible
  with `{fr,en}`)

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
