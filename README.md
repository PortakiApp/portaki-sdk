<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/PortakiApp/portaki-sdk/develop/docs/assets/portaki-wordmark-light.svg">
    <img src="https://portaki.app/portaki-wordmark.svg" width="177" height="48" alt="Portaki" />
  </picture>
</p>

<h1 align="center">Portaki SDK</h1>

<p align="center">
  <strong>Monorepo officiel</strong> — SDK <strong>JavaScript / React</strong>, SDK <strong>Java</strong> ; manifestes et modules invités <code>@portaki/module-*</code> dans <a href="https://github.com/PortakiApp/portaki-modules">portaki-modules</a><br/>
  <sub>pnpm · CI GitHub · publication npm &amp; Maven</sub>
</p>

<p align="center">
  <a href="https://github.com/PortakiApp/portaki-sdk/actions/workflows/ci-verify.yml"><img src="https://github.com/PortakiApp/portaki-sdk/actions/workflows/ci-verify.yml/badge.svg?branch=develop" alt="CI"></a>
  <a href="https://www.npmjs.com/org/portaki"><img src="https://img.shields.io/badge/npm-%40portaki-CB3837?logo=npm" alt="npm @portaki"></a>
  <a href="https://pnpm.io/workspaces"><img src="https://img.shields.io/badge/pnpm-workspace-f69220?logo=pnpm" alt="pnpm"></a>
  <a href="./docs/deployment.md"><img src="https://img.shields.io/badge/docs-deployment-0366d6" alt="Docs deployment"></a>
</p>

---

## Aperçu

| Chemin | Contenu |
|--------|---------|
| [`sdk/guest/`](sdk/guest/) | **`@portaki/sdk`** — pont invité : hooks, HMAC, Route Handlers Next (`/api/portaki/*`), slots |
| [`sdk/module-sdk/`](sdk/module-sdk/) | **`@portaki/module-sdk`** — `definePortakiModule` (modules invités npm) |
| [`sdk/java/`](sdk/java/) | **`app.portaki:portaki-module-sdk`** (Maven) — gateway + annotations hôte JVM |
| [`sdk/module-sdk/README.md`](sdk/module-sdk/README.md) | **README npm** du paquet **`@portaki/module-sdk`** |
| [portaki-modules](https://github.com/PortakiApp/portaki-modules) | Sources & publication npm des **`@portaki/module-*`** (dossier `modules/`) |

Les apps (ex. **portaki-web**) déclarent **`@portaki/module-sdk`** / **`@portaki/sdk`** et **`@portaki/module-*`** en **semver** (ou `file:` pour le pont en dev monorepo) depuis le registre public.

---

## Pont invité — `@portaki/sdk` (`sdk/guest/`)

- **Installation** : `pnpm add @portaki/sdk` (dans ce monorepo : `"@portaki/sdk": "file:../portaki-sdk/sdk/guest"` — exécuter d’abord `pnpm --filter @portaki/sdk run build` dans **portaki-sdk**, le dossier **`dist/`** n’est pas versionné).
- **Provider** : `PortakiProvider` reçoit le contexte métier + `hmacKeyMaterialB64` (dérivé côté serveur avec `deriveModuleHmacKeyMaterialB64(MODULE_HMAC_SECRET, moduleId, stayId)` — aligné sur la vérification Route Handler).
- **Hooks** : `usePortakiContext`, `usePortakiConfig`, `usePortakiQuery`, `usePortakiCommand`, `usePortakiModuleQuery`.
- **Serveur** : `import { verifyHmacToken, deriveModuleHmacKeyMaterialB64, portakiServerQuery } from '@portaki/sdk/server'`.
- **Schéma manifeste** : `schema/module.v1.json` (scopes, queries, commands, events, audit, slots).

---

## SDK JavaScript — `@portaki/module-sdk`

```bash
npm install @portaki/module-sdk react
```

| | |
|:---|:---|
| **npm** | [npmjs.com/package/@portaki/module-sdk](https://www.npmjs.com/package/@portaki/module-sdk) |
| **README paquet** | [sdk/module-sdk/README.md](sdk/module-sdk/README.md) |

```tsx
import { definePortakiModule } from "@portaki/module-sdk";

export default definePortakiModule({
  id: "example",
  label: { fr: "Exemple", en: "Example" },
  icon: "sparkles",
  navSlot: "section",
  render: ({ property, stay, lang }) => (
    <section>
      <h2>{lang === "fr" ? property.id : property.id}</h2>
    </section>
  ),
});
```

---

## SDK Java — `app.portaki:portaki-module-sdk`

```xml
<dependency>
  <groupId>app.portaki</groupId>
  <artifactId>portaki-module-sdk</artifactId>
  <version>0.4.0</version>
</dependency>
```

### Backend hôte (monolithe → microservice)

Le paquet **`app.portaki.sdk.module.backend`** définit le contrat stable pour tout travail serveur rattaché à un module hôte (jobs planifiés, actions explicites, webhooks, …) — **sans logique métier** d’un module précis :

| Type | Rôle |
|------|------|
| `ModuleHostContext` | `tenantId`, `propertyId`, `moduleId` — périmètre après contrôle d’accès API |
| `HostModuleAction` | Action nommée (ex. `SYNC`) — extensible pour webhooks / jobs |
| `HostModuleRunResult` | Agrégat générique (succès / échecs / total / résumé + JSON config plat optionnel après action) |
| `PortakiHostModuleBackend` | Implémentation par `moduleId` |
| `PortakiHostModuleBackendRegistry` | Résolution `moduleId` → backend (beans Spring aujourd’hui, client HTTP vers microservice demain) |
| `app.portaki.sdk.module.backend.run.*` | Orchestration **in-process** d’un run module (`ModuleRunContext`, `ModuleRunPipeline`, `ModuleRunListener`) — sans saga applicative ni Axon |
| `@PortakiHostModule` | Marqueur sur les implémentations pour découverte / conventions |

L’API cœur peut **router** vers une implémentation in-process ou vers un **microservice modules** sans changer les types consommés par les modules « officiels ».

---

## Développement local

| Zone | Commandes |
|------|-------------|
| **@portaki/sdk** (guest) | À la racine : `pnpm install` puis `pnpm --filter @portaki/sdk run build` |
| **@portaki/module-sdk** | `cd sdk/module-sdk` → `npm ci` → `npm run build` |
| **SDK Java** | `cd sdk/java` → `mvn verify` |
| **Racine pnpm** | `pnpm install` (workspace minimal : SDK JS uniquement) |

---

## CI / CD

| Workflow | Rôle |
|----------|------|
| [`ci-verify.yml`](.github/workflows/ci-verify.yml) | Build SDK JS et `mvn verify` Java |
| [`publish-npm-sdk.yml`](.github/workflows/publish-npm-sdk.yml) | Publie **`@portaki/module-sdk`** sur npmjs |
| [`publish-maven-sdk.yml`](.github/workflows/publish-maven-sdk.yml) | JAR SDK vers Maven Central |

Les **`@portaki/module-*`** invités sont publiés depuis **[portaki-modules](https://github.com/PortakiApp/portaki-modules)**.

Détail publication : **[docs/deployment.md](docs/deployment.md)** · guide **[docs/getting-started.md](docs/getting-started.md)**

---

## Licence

Le dépôt contient **`@portaki/module-sdk`** (MIT), le pont **`@portaki/sdk`** (AGPL — `sdk/guest/`), et le **SDK Java** (Apache 2.0) ; les modules invités AGPL sont dans **[portaki-modules](https://github.com/PortakiApp/portaki-modules)** — voir le champ `license` de chaque paquet.
