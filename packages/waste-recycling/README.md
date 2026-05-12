<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/PortakiApp/portaki-sdk/develop/docs/assets/portaki-wordmark-light.svg">
  <img src="https://portaki.app/portaki-wordmark.svg" width="160" height="44" alt="Portaki" />
</picture>

# Tri & déchets

### `@portaki/module-waste-recycling`

[![npm](https://img.shields.io/npm/v/@portaki/module-waste-recycling?label=npm&logo=npm&color=CB3837)](https://www.npmjs.com/package/@portaki/module-waste-recycling)
[![license](https://img.shields.io/badge/license-AGPL--3.0-blue)](https://opensource.org/licenses/AGPL-3.0)
[![SDK](https://img.shields.io/badge/built%20with-%40portaki%2Fmodule--sdk-181717?logo=github)](https://github.com/PortakiApp/portaki-sdk)

*Bac par bac : où jeter quoi, jours de collecte*

</div>

---

> **En une phrase** — **`bins_json`** décrit chaque bac (titre FR/EN + lignes « quoi jeter ») ; **`collection_schedule`** regroupe jours de passage, liens mairie, consignes libres.

## Fiche technique

| Clé | Valeur |
|-----|--------|
| **npm** | `@portaki/module-waste-recycling` |
| **`id`** | `waste-recycling` |
| **Slot nav** | `section` |
| **Icône** | `recycle` |
| **Manifeste** | [`portaki.module.json`](./portaki.module.json) |

---

## Champs hôte

| Champ | Rôle |
|--------|------|
| `bins_json` | Tableau : `id`, `title` {fr,en}, `items` [{fr,en}, …]. |
| `collection_schedule` | Texte libre : jours de collecte, calendrier communal. |

Schéma aligné sur **`official-modules/waste-recycling.json`**.

---

## Développement local

```bash
cd portaki-sdk && pnpm install && pnpm run validate:modules
```

---

## Licence

**AGPL-3.0** — voir `package.json`.
