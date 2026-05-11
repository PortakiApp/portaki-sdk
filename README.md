<div align="center">

# 🧩 Portaki SDK

**Modules invités · React · Java · monorepo `pnpm`**

[![CI](https://github.com/PortakiApp/portaki-sdk/actions/workflows/ci-verify.yml/badge.svg?branch=develop)](https://github.com/PortakiApp/portaki-sdk/actions/workflows/ci-verify.yml)
[![npm org](https://img.shields.io/badge/npm-%40portaki-CB3837?logo=npm)](https://www.npmjs.com/org/portaki)
[![pnpm](https://img.shields.io/badge/pnpm-workspace-f69220?logo=pnpm)](https://pnpm.io/workspaces)
[![Docs](https://img.shields.io/badge/docs-deployment-0366d6)](docs/deployment.md)

</div>

> 📦 Bibliothèques officielles pour construire des **modules guest** [Portaki](https://github.com/PortakiApp) : SDK **JavaScript / React**, SDK **Java**, et paquets **`@portaki/module-*`** dans un seul dépôt.

---

## 📂 Ce que tu trouves ici

| 🗂️ Chemin | 📌 Contenu |
|-----------|------------|
| [`sdk/javascript/`](sdk/javascript/) | **`@portaki/module-sdk`** — types, `definePortakiModule`, build TypeScript |
| [`sdk/java/`](sdk/java/) | **`app.portaki:portaki-module-sdk`** — annotations & modèle côté JVM |
| [`packages/`](packages/) | Modules guest publiés sous **`@portaki/module-*`** |

<details>
<summary><strong>🔗 Workspace <code>pnpm</code></strong> (clique pour déplier)</summary>

Les paquets sous `packages/` déclarent `@portaki/module-sdk` en **`workspace:^`**. En local, pnpm relie au dossier `sdk/javascript`. Au **`pnpm publish`**, cette dépendance est réécrite vers une semver publiable (ex. **`^0.2.2`** tant que le SDK est en `0.2.x`).

</details>

---

## ⚛️ SDK JavaScript — `@portaki/module-sdk`

| | |
|:---|:---|
| **Installer** | `npm install @portaki/module-sdk react` |
| **Registry** | [npmjs — `@portaki/module-sdk`](https://www.npmjs.com/package/@portaki/module-sdk) |

**Exemple minimal :**

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

## ☕ SDK Java — `app.portaki:portaki-module-sdk`

```xml
<dependency>
  <groupId>app.portaki</groupId>
  <artifactId>portaki-module-sdk</artifactId>
  <version>0.2.2-SNAPSHOT</version>
</dependency>
```

---

## 🛠️ Développement local

| Stack | Commandes |
|-------|-----------|
| **SDK JS** | `cd sdk/javascript` → `npm ci` → `npm run build` |
| **SDK Java** | `cd sdk/java` → `mvn verify` |
| **Workspace** | à la racine : `pnpm install` → `pnpm lint` |

---

## 🚀 CI / CD

Les workflows utilisent des **slugs** stables (`ci-verify`, `publish-npm-sdk`, …) et des chemins filtrés (`paths` + [`paths-filter`](https://github.com/dorny/paths-filter) dans `ci-verify`).

| Workflow | 🎯 Rôle |
|----------|---------|
| [`ci-verify.yml`](.github/workflows/ci-verify.yml) | Build SDK JS, `mvn verify` Java, lint `packages/`, backend pre-arrival si touché |
| [`publish-npm-sdk.yml`](.github/workflows/publish-npm-sdk.yml) | Publie **`@portaki/module-sdk`** sur **npmjs** (Trusted Publishing / OIDC) |
| [`publish-maven-sdk.yml`](.github/workflows/publish-maven-sdk.yml) | JAR SDK vers **Maven Central** (`OSSRH_USERNAME`, `OSSRH_TOKEN`) |
| [`publish-npm-packages.yml`](.github/workflows/publish-npm-packages.yml) | Publication **manuelle** des `@portaki/module-*` |

> **☁️ npmjs** — Bump la **`version`** dans les `package.json` concernés, configure [Trusted Publishing](https://docs.npmjs.com/trusted-publishers), puis lance les workflows. Détail : **[docs/deployment.md](docs/deployment.md)**.

📘 Guide API : **[docs/getting-started.md](docs/getting-started.md)**

---

## 📜 Licence

Le dépôt mélange **MIT** (SDK JS par défaut) et **AGPL-3.0** (plusieurs modules sous `packages/`) — voir le champ `license` de chaque paquet.
