<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/PortakiApp/portaki-sdk/develop/docs/assets/portaki-wordmark-light.svg">
  <img src="https://portaki.app/portaki-wordmark.svg" width="160" height="44" alt="Portaki" />
</picture>

# Bons plans du coin

### `@portaki/module-local-guide`

[![npm](https://img.shields.io/npm/v/@portaki/module-local-guide?label=npm&logo=npm&color=CB3837)](https://www.npmjs.com/package/@portaki/module-local-guide)
[![license](https://img.shields.io/badge/license-AGPL--3.0-blue)](https://opensource.org/licenses/AGPL-3.0)
[![SDK](https://img.shields.io/badge/built%20with-%40portaki%2Fmodule--sdk-181717?logo=github)](https://github.com/PortakiApp/portaki-sdk)

*Boulangerie, plage, parking, urgences — liens Maps et notes courtes*

</div>

---

> **En une phrase** — Liste **curatée** par l’hôte : `spots_json` (id, titre FR/EN, URL, catégorie, note) + **`disclaimer`** optionnel.

## Fiche technique

| Clé | Valeur |
|-----|--------|
| **npm** | `@portaki/module-local-guide` |
| **`id`** | `local-guide` |
| **Slot nav** | `section` |
| **Icône** | `map-pin` |
| **Manifeste** | [`portaki.module.json`](./portaki.module.json) |

---

## Champs hôte

| Champ | Rôle |
|--------|------|
| `spots_json` | Tableau d’entrées (voir exemple FR/EN dans le manifeste). |
| `disclaimer` | Avertissement (horaires indicatifs, etc.). |

Schéma aligné sur **`official-modules/local-guide.json`**.

---

## Développement local

```bash
cd portaki-sdk && pnpm install && pnpm run validate:modules
```

---

## Licence

**AGPL-3.0** — voir `package.json`.
