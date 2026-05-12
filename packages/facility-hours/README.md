<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/PortakiApp/portaki-sdk/develop/docs/assets/portaki-wordmark-light.svg">
  <img src="https://portaki.app/portaki-wordmark.svg" width="160" height="44" alt="Portaki" />
</picture>

# Horaires & accès

### `@portaki/module-facility-hours`

[![npm](https://img.shields.io/npm/v/@portaki/module-facility-hours?label=npm&logo=npm&color=CB3837)](https://www.npmjs.com/package/@portaki/module-facility-hours)
[![license](https://img.shields.io/badge/license-AGPL--3.0-blue)](https://opensource.org/licenses/AGPL-3.0)
[![SDK](https://img.shields.io/badge/built%20with-%40portaki%2Fmodule--sdk-181717?logo=github)](https://github.com/PortakiApp/portaki-sdk)

*Piscine, spa, salle de sport — horaires par équipement et par logement*

</div>

---

> **En une phrase** — Un **JSON par logement** : chaque équipement a un titre FR/EN, des **lignes d’horaires** et une **note** optionnelle (bracelet, âge minimum, etc.).

## Fiche technique

| Clé | Valeur |
|-----|--------|
| **npm** | `@portaki/module-facility-hours` |
| **`id`** | `facility-hours` |
| **Slot nav** | `section` |
| **Icône** | `waves` |
| **Manifeste** | [`portaki.module.json`](./portaki.module.json) |

---

## Champs hôte

| Champ | Rôle |
|--------|------|
| `facilities_json` | Tableau : `id`, `title` {fr,en}, `lines` [{fr,en}, …], `note` {fr,en} optionnel. |
| `general_note` | Texte libre sous la liste (fermeture hiver, travaux). |

Schéma aligné sur **`official-modules/facility-hours.json`**.

---

## Développement local

```bash
cd portaki-sdk && pnpm install && pnpm run validate:modules
```

---

## Licence

**AGPL-3.0** — voir `package.json`.
