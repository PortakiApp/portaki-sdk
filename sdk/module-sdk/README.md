<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/PortakiApp/portaki-sdk/develop/docs/assets/portaki-wordmark-light.svg">
    <img src="https://portaki.app/portaki-wordmark.svg" width="160" height="44" alt="Portaki" />
  </picture>
</p>

<h1 align="center">@portaki/module-sdk</h1>

<p align="center">
  <strong>SDK officiel npm</strong> — construire des <strong>modules invités</strong> React pour <a href="https://portaki.app">Portaki</a><br/>
  <sub>TypeScript · ESM · <code>definePortakiModule</code></sub>
</p>

<p align="center">
  <a href="https://www.npmjs.com/package/@portaki/module-sdk"><img src="https://img.shields.io/npm/v/@portaki/module-sdk?logo=npm&color=CB3837&label=npm" alt="npm version"></a>
  <a href="https://github.com/PortakiApp/portaki-sdk/blob/develop/sdk/module-sdk/LICENSE"><img src="https://img.shields.io/badge/license-MIT-22c55e" alt="License MIT"></a>
  <a href="https://github.com/PortakiApp/portaki-sdk"><img src="https://img.shields.io/badge/source-portaki--sdk-181717?logo=github" alt="GitHub"></a>
</p>

---

## Installation

```bash
npm install @portaki/module-sdk react
# ou
pnpm add @portaki/module-sdk react
```

**Peer** : `react` ≥ 18.

---

## Démarrage rapide

Expose un **export default** depuis le point d’entrée de ton paquet npm (comme les modules `@portaki/module-*` du monorepo) :

```tsx
import { definePortakiModule } from "@portaki/module-sdk";

export default definePortakiModule({
  id: "hello",
  label: { fr: "Bonjour", en: "Hello" },
  icon: "sparkles",
  navSlot: "section",
  render: ({ property, stay, lang }) => (
    <section style={{ padding: "1rem" }}>
      <h2>{lang === "fr" ? "Votre séjour" : "Your stay"}</h2>
      <p>{property.name}</p>
    </section>
  ),
});
```

`definePortakiModule` complète les champs optionnels (`description`, `version`, `navSlot`, …) avec des valeurs par défaut cohérentes pour le runtime Portaki.

---

## API publique

| Export | Rôle |
|--------|------|
| **`definePortakiModule`** | Fabrique une définition de module typée (`PortakiModuleDefinition`). |
| **`useTracking`** | Hook optionnel pour instrumentation / analytics côté module. |
| **Composants** | `ModuleSection`, `ModuleCard`, `ModuleLoading`, `ModuleError`, `ModuleEmpty`, `CopyButton`, `ExternalLink`, `WazeButton`, `GoogleMapsButton`, `ModuleConfigAlert` — blocs UI alignés sur la page invité. |
| **Types** | `ModuleContext`, `StayData`, `PropertyData`, `NavSlot`, `StayStatus`, schéma de config module (`ModuleConfigSchema`), etc. |

Manifest JSON des modules : schéma **`module.v1`** dans le dépôt [portaki-sdk](https://github.com/PortakiApp/portaki-sdk) (`schema/module.v1.json`).

---

## Liens

| Ressource | URL |
|-----------|-----|
| Site & marque | [portaki.app](https://portaki.app) |
| Monorepo (Java, modules, CI) | [github.com/PortakiApp/portaki-sdk](https://github.com/PortakiApp/portaki-sdk) |
| Paquet npm | [npmjs.com/package/@portaki/module-sdk](https://www.npmjs.com/package/@portaki/module-sdk) |
| Guide développeur | [Getting started](https://github.com/PortakiApp/portaki-sdk/blob/develop/docs/getting-started.md) |
| SDK Java (Maven) | [sdk/java](https://github.com/PortakiApp/portaki-sdk/tree/develop/sdk/java) |

---

## Build local

```bash
cd sdk/module-sdk
npm ci
npm run build
```

Les types et le JS compilé sortent dans `dist/`.

---

## Version Java (Maven)

Backend des modules : artefact **`app.portaki:portaki-module-sdk`** — [README Java](https://github.com/PortakiApp/portaki-sdk/blob/develop/sdk/java/README.md).

---

## Licence

**MIT** — voir le champ `license` du `package.json`.
