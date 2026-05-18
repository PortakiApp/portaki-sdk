# @portaki/sdk-test-support

Package de **tests unitaires** pour les modules npm `@portaki/module-*` : fixtures, validation du manifeste catalogue, rendu invité/hôte avec Testing Library.

## Installation

```json
{
  "devDependencies": {
    "@portaki/sdk": "^2.0.0",
    "@portaki/sdk-test-support": "^2.0.0",
    "vitest": "^3.0.5",
    "jsdom": "^26.0.0"
  }
}
```

## Vitest preset

```ts
// vitest.config.ts
import { defineConfig, mergeConfig } from 'vitest/config'
import portakiModuleVitestConfig from '@portaki/sdk-test-support/vitest'

export default mergeConfig(
  portakiModuleVitestConfig(import.meta.url),
  defineConfig({ /* overrides module-specific */ }),
)
```

## Fixtures

| Symbole | Rôle |
|---------|------|
| `FIXTURE_STAY` | Séjour invité type actif |
| `FIXTURE_PROPERTY` | Logement avec checklist |
| `createMockModuleContext()` | Contexte `ModuleContext` pour `render()` |
| `createMockHostModuleContext()` | Contexte hôte pour `renderHost()` |
| `createSpyTrack()` | Capture des événements analytics |

## Validation manifeste

```ts
import { validateSiblingManifest } from '@portaki/sdk-test-support'

const result = validateSiblingManifest(import.meta.url)
expect(result.valid).toBe(true)
```

Valide `portaki.module.json` contre `schema/module.v1.json` (copie embarquée dans le package).

## Rendu composants

```ts
import { renderGuestModule, assertGuestSurface } from '@portaki/sdk-test-support'
import module from '../src/index'

const { container } = renderGuestModule(module, { lang: 'fr' })
assertGuestSurface(container)
```

## Référence API

Générée par TypeDoc : voir [API — sdk-test-support](/api/modules/sdk-test-support_src.html) (après `pnpm docs:api`).
