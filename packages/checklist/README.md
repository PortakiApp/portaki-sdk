<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/PortakiApp/portaki-sdk/develop/docs/assets/portaki-wordmark-light.svg">
  <img src="https://portaki.app/portaki-wordmark.svg" width="160" height="44" alt="Portaki" />
</picture>

# ✅ Module Checklist départ

### `@portaki/module-checklist`

[![npm](https://img.shields.io/npm/v/@portaki/module-checklist?label=npm&logo=npm&color=CB3837)](https://www.npmjs.com/package/@portaki/module-checklist)
[![license](https://img.shields.io/badge/license-AGPL--3.0-blue)](https://opensource.org/licenses/AGPL-3.0)
[![SDK](https://img.shields.io/badge/built%20with-%40portaki%2Fmodule--sdk-181717?logo=github)](https://github.com/PortakiApp/portaki-sdk)

*Liste de départ · persistance guest · FR / EN*

</div>

---

> 🎯 **En une phrase** — Accompagne la **sortie du séjour** avec une checklist pilotée par la config propriété et les **`POST`** guest.

## 👥 Pour qui ?

| Persona | Besoin |
|---------|--------|
| 🧳 **Voyageurs** | Ne rien oublier (clés, déchets, linge…) |
| 🔐 **Produit** | Afficher seulement pour séjour **`ACTIVE`** |

## ✨ Ce que le module apporte

- [x] Visible si **`visibleOnStatus: ['ACTIVE']`**
- [x] Items **FR / EN** via `property.checklistItems`
- [x] Hooks pour **`POST`** par item (`stayId`, `itemId`)

---

## 🧭 Fiche technique

| Clé | Valeur |
|-----|--------|
| 📦 **npm** | `@portaki/module-checklist` |
| 🆔 **`id`** | `checklist` |
| 📍 **Slot nav** | `section` |
| 🎨 **Icône** | `list-checks` |
| 📄 **Manifeste** | [`portaki.module.json`](./portaki.module.json) |
| 👁️ **Visibilité** | `visibleOnStatus: ['ACTIVE']` |
| 🗺️ **Carte** | Non |

---

## 🔌 Intégration Portaki

Sans **`stay`**, le module ne rend rien. Sinon **`ChecklistSection`** reçoit `stay.id` et `property.checklistItems`.

## 📡 Données & API

Persistance type **`POST .../checklist/{itemId}`** côté guest — adapte les routes une fois figées.

---

## 🛠️ Développement local

```bash
pnpm install   # racine du monorepo portaki-sdk
```

Dépend de **`@portaki/module-sdk`** → [**portaki-sdk**](https://github.com/PortakiApp/portaki-sdk).

---

## 📄 Licence

**AGPL-3.0** — voir `package.json`.
