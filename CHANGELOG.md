# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
