# Portaki SDK

Bibliothèques officielles pour développer des **modules invités** Portaki : SDK **JavaScript / React**, SDK **Java**, et les **paquets** `@portakiapp/module-*` (dossier `packages/`) dans un même dépôt monorepo (`pnpm`).

[![CI](https://github.com/portaki/portaki-sdk/actions/workflows/ci-verify.yml/badge.svg?branch=develop)](https://github.com/portaki/portaki-sdk/actions/workflows/ci-verify.yml)

---

## Arborescence

| Chemin | Contenu |
|--------|---------|
| [`sdk/javascript/`](sdk/javascript/) | `@portakiapp/module-sdk` — types et `definePortakiModule` |
| [`sdk/java/`](sdk/java/) | `app.portaki:portaki-module-sdk` — annotations backend |
| [`packages/`](packages/) | Paquets invités publiés sous `@portakiapp/module-*` |

À la racine : `pnpm-workspace.yaml` et `package.json` pour lier le workspace (`workspace:*` vers le SDK JS).

---

## SDK JavaScript (`@portakiapp/module-sdk`)

```bash
npm install @portakiapp/module-sdk react
```

```tsx
import { definePortakiModule } from "@portakiapp/module-sdk";

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

## SDK Java (`app.portaki:portaki-module-sdk`)

```xml
<dependency>
  <groupId>app.portaki</groupId>
  <artifactId>portaki-module-sdk</artifactId>
  <version>0.1.0-SNAPSHOT</version>
</dependency>
```

---

## Développement local

**SDK JavaScript**

```bash
cd sdk/javascript
npm ci
npm run build
```

**SDK Java**

```bash
cd sdk/java
mvn verify
```

**Workspace packages (pnpm)**

```bash
pnpm install
pnpm lint
```

Les paquets sous `packages/` résolvent `@portakiapp/module-sdk` via `workspace:*` (pas besoin de publication npm locale).

---

## CI/CD

Les workflows utilisent des **noms en slug** (`ci-verify`, `publish-npm-sdk`, …), des **jobs** stables (`sdk_javascript`, `workspace_packages`, …) et des **étapes** homogènes (`Checkout`, `Setup Node.js`, `Install dependencies`, …). Ils ne se déclenchent que lorsque les chemins pertinents changent (`paths` + filtre dans `ci-verify`).

| Workflow | Rôle |
|----------|------|
| [`ci-verify.yml`](.github/workflows/ci-verify.yml) | Build SDK JS, `mvn verify` SDK Java, lint `packages/`, backend pre-arrival si touché |
| [`publish-npm-sdk.yml`](.github/workflows/publish-npm-sdk.yml) | Publie **`@portakiapp/module-sdk`** sur **npmjs** (`NPM_TOKEN`) |
| [`publish-maven-sdk.yml`](.github/workflows/publish-maven-sdk.yml) | Déploie le JAR SDK vers **Maven Central** (OSSRH : `OSSRH_USERNAME`, `OSSRH_TOKEN`) |
| [`publish-npm-packages.yml`](.github/workflows/publish-npm-packages.yml) | Publication **manuelle** des `@portakiapp/module-*` (`workflow_dispatch`) |

**Publier sur npmjs** : créer un token npm avec droit **publish** sur le scope **`@portakiapp`**, ajouter le secret **`NPM_TOKEN`** au dépôt GitHub, puis lancer les workflows ou pousser sur `develop` — détail pas à pas dans **[docs/deployment.md](docs/deployment.md)**.

Guide d’utilisation des API : **[docs/getting-started.md](docs/getting-started.md)**.

---

## Licence

MIT — voir les champs `license` des paquets individuels (modules souvent AGPL-3.0).
