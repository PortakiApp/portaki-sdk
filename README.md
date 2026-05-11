# Portaki SDK

Bibliothèques officielles pour développer des **modules invités** Portaki : SDK **JavaScript / React**, SDK **Java**, et les **paquets** `@portakiapp/module-*` (dossier `packages/`) dans un même dépôt monorepo (`pnpm`).

[![CI](https://github.com/portaki/portaki-sdk/actions/workflows/ci.yml/badge.svg?branch=develop)](https://github.com/portaki/portaki-sdk/actions/workflows/ci.yml)

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

Les workflows ne se déclenchent que lorsque des fichiers pertinents changent (voir les filtres `paths` et les jobs conditionnels dans [`.github/workflows/ci.yml`](.github/workflows/ci.yml)).

| Workflow | Rôle |
|----------|------|
| [`.github/workflows/ci.yml`](.github/workflows/ci.yml) | Build SDK JS, `mvn verify` SDK Java, lint `packages/`, backend pre-arrival si touché |
| [`.github/workflows/publish-npm.yml`](.github/workflows/publish-npm.yml) | Publie **`@portakiapp/module-sdk`** sur **npmjs** (`NPM_TOKEN`) |
| [`.github/workflows/publish-maven.yml`](.github/workflows/publish-maven.yml) | Déploie le JAR SDK vers **Maven Central** via **OSSRH** (`OSSRH_USERNAME`, `OSSRH_TOKEN`) |
| [`.github/workflows/publish-modules-npm.yml`](.github/workflows/publish-modules-npm.yml) | Publication manuelle des packages `@portakiapp/module-*` sur npmjs |

Détails des secrets et des tags de release : **[docs/deployment.md](docs/deployment.md)**.

Guide d’utilisation des API : **[docs/getting-started.md](docs/getting-started.md)**.

---

## Licence

MIT — voir les champs `license` des paquets individuels (modules souvent AGPL-3.0).
