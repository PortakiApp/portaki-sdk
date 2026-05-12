<p align="center">
  <a href="https://portaki.app" title="Portaki">
    <img
      src="https://portaki.app/portaki-wordmark.svg"
      width="177"
      height="48"
      alt="Portaki"
    >
  </a>
</p>

<h1 align="center">Portaki SDK</h1>

<p align="center">
  <strong>Monorepo officiel</strong> — SDK <strong>JavaScript / React</strong>, SDK <strong>Java</strong>, et modules invités <code>@portaki/module-*</code><br/>
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
| [`sdk/javascript/`](sdk/javascript/) | **`@portaki/module-sdk`** `0.3.0` — types, `definePortakiModule`, build TypeScript |
| [`sdk/javascript/README.md`](sdk/javascript/README.md) | **README npm** dédié au paquet publié |
| [`sdk/java/`](sdk/java/) | **`app.portaki:portaki-module-sdk`** `0.3.0-SNAPSHOT` — annotations & modèle JVM |
| [`packages/`](packages/) | Modules invités publiés **`@portaki/module-*`** |

Les paquets sous `packages/` déclarent `@portaki/module-sdk` en **`workspace:^`**. En local, pnpm relie au dossier `sdk/javascript`. Au **`pnpm publish`**, la dépendance workspace est réécrite vers une semver publiable (ex. **`^0.3.0`**).

---

## SDK JavaScript — `@portaki/module-sdk`

```bash
npm install @portaki/module-sdk react
```

| | |
|:---|:---|
| **npm** | [npmjs.com/package/@portaki/module-sdk](https://www.npmjs.com/package/@portaki/module-sdk) |
| **README paquet** | [sdk/javascript/README.md](sdk/javascript/README.md) |

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
  <version>0.3.0-SNAPSHOT</version>
</dependency>
```

---

## Développement local

| Zone | Commandes |
|------|-------------|
| **SDK JS** | `cd sdk/javascript` → `npm ci` → `npm run build` |
| **SDK Java** | `cd sdk/java` → `mvn verify` |
| **Workspace** | racine : `pnpm install` → `pnpm lint` |

---

## CI / CD

| Workflow | Rôle |
|----------|------|
| [`ci-verify.yml`](.github/workflows/ci-verify.yml) | Build SDK JS, `mvn verify` Java, lint `packages/` |
| [`publish-npm-sdk.yml`](.github/workflows/publish-npm-sdk.yml) | Publie **`@portaki/module-sdk`** sur npmjs |
| [`publish-maven-sdk.yml`](.github/workflows/publish-maven-sdk.yml) | JAR SDK vers Maven Central |
| [`publish-npm-packages.yml`](.github/workflows/publish-npm-packages.yml) | Publication manuelle des `@portaki/module-*` |

Détail publication : **[docs/deployment.md](docs/deployment.md)** · guide **[docs/getting-started.md](docs/getting-started.md)**

---

## Licence

Le dépôt mélange **MIT** (SDK JS) et **AGPL-3.0** (plusieurs modules sous `packages/`) — voir le champ `license` de chaque paquet.
