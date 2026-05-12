<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/PortakiApp/portaki-sdk/develop/docs/assets/portaki-wordmark-light.svg">
  <img src="https://portaki.app/portaki-wordmark.svg" width="160" height="44" alt="Portaki" />
</picture>

# ⚖️ Module Règlement intérieur

### `@portaki/module-rules`

[![npm](https://img.shields.io/npm/v/@portaki/module-rules?label=npm&logo=npm&color=CB3837)](https://www.npmjs.com/package/@portaki/module-rules)
[![license](https://img.shields.io/badge/license-AGPL--3.0-blue)](https://opensource.org/licenses/AGPL-3.0)
[![SDK](https://img.shields.io/badge/built%20with-%40portaki%2Fmodule--sdk-181717?logo=github)](https://github.com/PortakiApp/portaki-sdk)

*House rules accessibles, contenu riche (TipTap / CMS)*

</div>

---

> 🎯 **En une phrase** — Une section **Règlement** claire pour bruit, fumeur, tri… avec **`data-module="rules"`** pour style & analytics.

## 👥 Pour qui ?

| Persona | Besoin |
|---------|--------|
| 🛋️ **Voyageurs** | Lire les règles avant / pendant le séjour |
| ✍️ **Ops / CMS** | Pousser du HTML ou TipTap sanitizé |

## ✨ Ce que le module apporte

- [x] Section **House rules** / **Règlement**
- [x] Hook **`data-module="rules"`**
- [x] Gabarit prêt pour **TipTap** ou HTML back-office

---

## 🧭 Fiche technique

| Clé | Valeur |
|-----|--------|
| 📦 **npm** | `@portaki/module-rules` |
| 🆔 **`id`** | `rules` |
| 📍 **Slot nav** | `section` |
| 🎨 **Icône** | `scroll-text` |
| 📄 **Manifeste** | [`portaki.module.json`](./portaki.module.json) |
| 👁️ **Visibilité** | Toujours affiché |
| 🗺️ **Carte** | Non |

---

## 🔌 Intégration Portaki

Remplace le placeholder par la chaîne / document fourni par ton API guest ou CMS.

## 📡 Données & API

Prévoir un endpoint (ou champ `property`) avec document structuré ou **HTML sanitisé**.

---

## 🛠️ Développement local

```bash
pnpm install   # racine du monorepo portaki-sdk
```

Dépend de **`@portaki/module-sdk`** → [**portaki-sdk**](https://github.com/PortakiApp/portaki-sdk).

---

## 📄 Licence

**AGPL-3.0** — voir `package.json`.
