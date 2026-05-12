<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/PortakiApp/portaki-sdk/develop/docs/assets/portaki-wordmark-light.svg">
  <img src="https://portaki.app/portaki-wordmark.svg" width="160" height="44" alt="Portaki" />
</picture>

# 📅 Module Événements

### `@portaki/module-events`

[![npm](https://img.shields.io/npm/v/@portaki/module-events?label=npm&logo=npm&color=CB3837)](https://www.npmjs.com/package/@portaki/module-events)
[![license](https://img.shields.io/badge/license-AGPL--3.0-blue)](https://opensource.org/licenses/AGPL-3.0)
[![SDK](https://img.shields.io/badge/built%20with-%40portaki%2Fmodule--sdk-181717?logo=github)](https://github.com/PortakiApp/portaki-sdk)

*Annonce locale + hooks carte pour l’app guest*

</div>

---

> 🎯 **En une phrase** — Section **Événements** avec support **carte** (`mapOverlay`, marqueurs async) pour tout ce qui bouge autour du logement.

## 👥 Pour qui ?

| Persona | Besoin |
|---------|--------|
| 🧭 **Voyageurs** | Activités, événements, repères sur la carte |
| 🗺️ **Produit** | Overlays carte cohérents avec les autres modules |

## ✨ Ce que le module apporte

- [x] Entrée menu **« Événements »**
- [x] **`mapOverlay: true`** + hook **`mapMarkers`** (async)
- [x] **`EventsSection`** prêt à être branché sur tes données

---

## 🧭 Fiche technique

| Clé | Valeur |
|-----|--------|
| 📦 **npm** | `@portaki/module-events` |
| 🆔 **`id`** | `events` |
| 📍 **Slot nav** | `section` |
| 🎨 **Icône** | `calendar-days` |
| 📄 **Manifeste** | [`portaki.module.json`](./portaki.module.json) |
| 👁️ **Visibilité** | Toujours affiché |
| 🗺️ **Carte** | Oui — overlay + marqueurs typés côté app |

---

## 🔌 Intégration Portaki

`render` reçoit `property.id`. Implémente **`mapMarkers`** selon ton API d’événements.

## 📡 Données & API

Endpoints guest listant les événements (par `propertyId` ou zone). La structure UI est là ; branche les flux réels.

---

## 🛠️ Développement local

```bash
pnpm install   # racine du monorepo portaki-sdk
```

Dépend de **`@portaki/module-sdk`** → [**portaki-sdk**](https://github.com/PortakiApp/portaki-sdk).

---

## 📄 Licence

**AGPL-3.0** — voir `package.json`.
