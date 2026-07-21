# Connectors and credentials (BYOK / pool)

How a Wasm module declares which external API key it needs, and how the host knows whether that key is required.

## The required triplet

For every paid / keyed external API, declare **all three**:

1. **Custom connector** with `credential_provider_id` — identifies the secret provider (`open-weather`, `mapbox`, …).
2. **Optional capability** `external.<provider>.pool` — plan may grant platform pool access.
3. **Optional capability** `external.<provider>.byok` — workspace may attach its own key (BYOK).

Example (weather):

```rust
#[portaki_sdk::custom_connector(
    id = "open-weather",
    display_name_key = "connector.openWeather.name",
    base_url = "https://api.openweathermap.org",
    credential_provider_id = "open-weather"
)]
pub struct ModuleOpenWeather;

#[portaki_sdk::capability(
    optional,
    id = "external.open-weather.pool",
    purpose_key = "capability.openWeather.purpose",
    fallback_key = "capability.openWeather.fallback"
)]
pub const OPEN_WEATHER_POOL: &str = portaki_sdk::capability::external::OPEN_WEATHER_POOL;

#[portaki_sdk::capability(
    optional,
    id = "external.open-weather.byok",
    purpose_key = "capability.openWeather.byok.purpose",
    fallback_key = "capability.openWeather.byok.fallback"
)]
pub const OPEN_WEATHER_BYOK: &str = portaki_sdk::capability::external::OPEN_WEATHER_BYOK;
```

Prefer the constants in `portaki_sdk::capability::external::*` so ids stay aligned with the orchestrator.

### Known providers

| `credential_provider_id` | Pool capability | BYOK capability |
|--------------------------|-----------------|-----------------|
| `open-weather` | `external.open-weather.pool` | `external.open-weather.byok` |
| `mapbox` | `external.mapbox.pool` | `external.mapbox.byok` |
| `google-places` | `external.google-places.pool` | `external.google-places.byok` |
| `osm` | `external.osm.pool` | — (no BYOK today) |

## What ends up in the manifest

`portaki build` merges macro emissions into `target/portaki/manifest.json`:

```json
{
  "manifestVersion": "1",
  "id": "weather",
  "capabilities": {
    "required": ["core.storage"],
    "optional": [
      { "id": "external.open-weather.pool", "purpose_key": "...", "fallback_key": "..." },
      { "id": "external.open-weather.byok", "purpose_key": "...", "fallback_key": "..." }
    ]
  },
  "connectors": {
    "custom": [
      {
        "id": "open-weather",
        "credentialProviderId": "open-weather",
        "baseUrl": "https://api.openweathermap.org",
        "operations": []
      }
    ]
  }
}
```

When there is no `portaki.module.json`, publish copies this SDK manifest as the OCI host catalog layer. The runtime keeps `connectors` / `capabilities` on the catalog so the **orchestrator** can derive credential bindings.

## Who consumes what

| Consumer | Reads | Purpose |
|----------|--------|---------|
| **Orchestrator** (registry + Integrations) | `connectors.custom[].credentialProviderId` + `external.*.pool` / `.byok` | Per-module readiness, “which module uses which key”, BYOK required vs pool granted |
| **Module runtime** (egress) | Same connector metadata + workspace secret / pool env | Inject key at HTTP call time; fail with `connector_credential_missing` if neither is available |
| **Dashboard** | Orchestrator readiness APIs | Intégrations UI + module banners — no hardcoded module lists |

## Browser-safe client tokens

Some providers expose a **public** token meant for browser SDKs (today: Mapbox `pk.*`). Those are **not** injected at module egress — frontends resolve them via:

| Audience | Endpoint |
|----------|----------|
| Host (dashboard / mobile) | `GET /api/v1/workspace/client-tokens/{providerId}` |
| Guest (stay-scoped) | `GET /api/v1/guest/{slug}/{accessCode}/client-tokens/{providerId}` |

Response: `{ providerId, token, source: "byok" | "platform" }`.

Only providers with `CredentialProvider.clientExposable == true` are allowed. OpenWeather / Google Places stay egress-only — calling the client-token route for them returns `client_token_not_exposable`.

**Mapbox platform pool env (orchestrator):** `MAPBOX_ACCESS_TOKEN` (preferred) or `MAPBOX_POOL_TOKEN` (same name as module-runtime egress). Plan coverage `PLAN_POOL` / UI “pool included” only grants entitlement — without one of those env vars the API returns `client_token_unavailable`.

Legacy alias (deprecated): `GET /api/v1/workspace/mapbox-access-token` → same Mapbox resolution.

## Author checklist

- [ ] `credential_provider_id` matches an orchestrator `CredentialProvider` id
- [ ] Both pool and BYOK capabilities declared when BYOK is supported (or document why not)
- [ ] Guest / host UX when neither pool nor BYOK is available (empty state + i18n fallback keys)
- [ ] After publish, registry `credentialBindings` for the module is non-empty
- [ ] Do **not** hardcode module → provider maps in the dashboard; declare in the module

## Optional host catalog

If you maintain a hand-written `portaki.module.json`, mirror `connectors` and `capabilities` there too (or rely on SDK-only publish). Dual-layer publish (`vnd.portaki.manifest` + `vnd.portaki.sdk.manifest`) is for host catalogs that are **not** SDK-shaped; credential fields must still be present on the host catalog layer.

## Reference

- Official example: [`portaki-modules/modules/weather`](https://github.com/PortakiApp/portaki-modules/tree/main/modules/weather)
- Capability constants: `crates/portaki-sdk/src/capability.rs`
- Macros: `#[custom_connector]`, `#[connector_op]`, `#[capability]`
