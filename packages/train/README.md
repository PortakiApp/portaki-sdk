<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/PortakiApp/portaki-sdk/develop/docs/assets/portaki-wordmark-light.svg">
  <img src="https://portaki.app/portaki-wordmark.svg" width="160" height="44" alt="Portaki" />
</picture>

# 🚆 Module Trains

### `@portaki/module-train`

[![npm](https://img.shields.io/npm/v/@portaki/module-train?label=npm&logo=npm&color=CB3837)](https://www.npmjs.com/package/@portaki/module-train)
[![license](https://img.shields.io/badge/license-AGPL--3.0-blue)](https://opensource.org/licenses/AGPL-3.0)
[![SDK](https://img.shields.io/badge/built%20with-%40portaki%2Fmodule--sdk-181717?logo=github)](https://github.com/PortakiApp/portaki-sdk)

*Horaires & infos voyageurs au départ de la gare configurée*

</div>

---

> 🎯 **En une phrase** — Donne aux voyageurs une vue **Trains** branchée sur **Navitia / données voyageurs**, contextualisée avec le **code gare** de la propriété.

## 👥 Pour qui ?

| Persona | Besoin |
|---------|--------|
| 🧳 **Voyageurs** | Préparer un trajet depuis le logement |
| 🏠 **Hôte** | Afficher la bonne gare (`trainStationCode`) |

## ✨ Ce que le module apporte

- [x] Entrée de navigation dédiée **« Trains »**
- [x] Liaison avec **`property.trainStationCode`**
- [x] Base UI prête pour tes appels **Navitia / SNCF** guest

---

## 🧭 Fiche technique

| Clé | Valeur |
|-----|--------|
| 📦 **npm** | `@portaki/module-train` |
| 🆔 **`id`** | `train` |
| 📍 **Slot nav** | `section` |
| 🎨 **Icône** | `train-front` |
| 📄 **Manifeste** | [`portaki.module.json`](./portaki.module.json) |
| 👁️ **Visibilité** | Toujours affiché |
| 🗺️ **Carte** | Non |

---

## 🔌 Intégration Portaki

Export **default** via `definePortakiModule`. Le rendu utilise `property.trainStationCode` pour adapter l’affichage.

## 📡 Données & API

Branche tes endpoints guest (Navitia / hub SNCF). Le composant **`TrainSection`** est le point d’accroche pour tes `fetch`.

---

## 🛠️ Développement local

```bash
# depuis la racine du monorepo portaki-sdk
pnpm install
```

Dépend de **`@portaki/module-sdk`** → dépôt [**portaki-sdk**](https://github.com/PortakiApp/portaki-sdk).

---

## 📄 Licence

**AGPL-3.0** — voir `package.json`.
