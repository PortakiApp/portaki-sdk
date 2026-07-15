# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
