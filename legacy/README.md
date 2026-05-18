# Legacy Java SDK

Maven artifacts for **JAR module backends** still used by catalogue modules in [portaki-modules](https://github.com/PortakiApp/portaki-modules).

| Path | Artifact |
|------|----------|
| [`java/`](java/) | `app.portaki:portaki-module-sdk` |
| [`java-test-support/`](java-test-support/) | `app.portaki:portaki-module-sdk-test` |

New modules should use the TypeScript stack in [`packages/sdk`](../packages/sdk) + [`packages/cli`](../packages/cli) instead of adding Java backends here.

CI still verifies and publishes these JARs until the catalogue is fully migrated.
