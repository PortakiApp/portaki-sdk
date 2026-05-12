<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/PortakiApp/portaki-sdk/develop/docs/assets/portaki-wordmark-light.svg">
  <img src="https://portaki.app/portaki-wordmark.svg" width="160" height="44" alt="Portaki" />
</picture>

# 🔌 Module Appareils

### `@portaki/module-appliances`

[![npm](https://img.shields.io/npm/v/@portaki/module-appliances?label=npm&logo=npm&color=CB3837)](https://www.npmjs.com/package/@portaki/module-appliances)
[![license](https://img.shields.io/badge/license-AGPL--3.0-blue)](https://opensource.org/licenses/AGPL-3.0)
[![SDK](https://img.shields.io/badge/built%20with-%40portaki%2Fmodule--sdk-181717?logo=github)](https://github.com/PortakiApp/portaki-sdk)

*Four, lave-linge, chauffage… guide éditorial par équipement*

</div>

---

> 🎯 **En une phrase** — Aide les voyageurs à **utiliser les équipements** avec consignes courtes et contenu enrichissable.

## 👥 Pour qui ?

| Persona | Besoin |
|---------|--------|
| 🏠 **Voyageurs** | Mode d’emploi express sur place |
| 📝 **Contenu** | Liste d’appareils + médias par propriété |

## ✨ Ce que le module apporte

- [x] Section **« Appareils »** dans la nav guest
- [x] Structure pour guide riche (**TipTap**, médias, FAQ courte)

---

## 🧭 Fiche technique

| Clé | Valeur |
|-----|--------|
| 📦 **npm** | `@portaki/module-appliances` |
| 🆔 **`id`** | `appliances` |
| 📍 **Slot nav** | `section` |
| 🎨 **Icône** | `plug` |
| 📄 **Manifeste** | [`../portaki.module.json`](../portaki.module.json) |
| 👁️ **Visibilité** | Toujours affiché |
| 🗺️ **Carte** | Non |

---

## 🔌 Intégration Portaki

Branche CMS / JSON propriété dans le JSX pour remplacer la démo.

## 📡 Données & API

Contenus **par propriété** : le configurateur hôte enregistre `devices_json` (tableau JSON) et `safety_notice` (optionnel). Chaque appareil supporte `title` {fr,en}, `tips`, `manualUrl`, `icon`.

### Exemple `devices_json`

```json
[
  {
    "id": "lave-vaisselle",
    "title": { "fr": "Lave-vaisselle", "en": "Dishwasher" },
    "tips": { "fr": "Programme Éco — pastilles dans le tiroir.", "en": "Eco cycle — tabs in the drawer." },
    "manualUrl": "",
    "icon": "utensils"
  }
]
```

La page invité reçoit la config déchiffrée via `GET /api/v1/guest/{slug}/{code}` (`moduleConfigs`).

---

## 🛠️ Développement local

```bash
pnpm install   # racine du monorepo portaki-sdk
```

Dépend de **`@portaki/module-sdk`** → [**portaki-sdk**](https://github.com/PortakiApp/portaki-sdk).

---

## 📄 Licence

**AGPL-3.0** — voir `package.json`.
