<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/PortakiApp/portaki-sdk/develop/docs/assets/portaki-wordmark-light.svg">
    <img src="https://portaki.app/portaki-wordmark.svg" width="177" height="48" alt="Portaki" />
  </picture>
</p>

<h1 align="center">Portaki SDK</h1>

<p align="center">
  <strong>Monorepo officiel</strong> — SDK <strong>JavaScript / React</strong>, SDK <strong>Java</strong>, schéma manifestes ; modules invités <code>@portaki/module-*</code> dans <a href="https://github.com/PortakiApp/portaki-modules">portaki-modules</a><br/>
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
| [`sdk/javascript/`](sdk/javascript/) | **`@portaki/module-sdk`** `0.3.2` — types, `definePortakiModule`, build TypeScript |
| [`sdk/javascript/README.md`](sdk/javascript/README.md) | **README npm** dédié au paquet publié |
| [`sdk/java/`](sdk/java/) | **`app.portaki:portaki-module-sdk`** `0.3.0-SNAPSHOT` — annotations & modèle JVM |
| [portaki-modules](https://github.com/PortakiApp/portaki-modules) | Sources & publication npm des **`@portaki/module-*`** (dossier `modules/`) |

Les apps (ex. **portaki-web**) déclarent **`@portaki/module-sdk`** et **`@portaki/module-*`** en **semver** depuis le registre public.

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

Le dépôt contient le **SDK JS (MIT)** et le **SDK Java** ; les modules invités AGPL sont dans **[portaki-modules](https://github.com/PortakiApp/portaki-modules)** — voir le champ `license` de chaque paquet.
