<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/PortakiApp/portaki-sdk/develop/docs/assets/portaki-wordmark-light.svg">
  <img src="https://portaki.app/portaki-wordmark.svg" width="160" height="44" alt="Portaki" />
</picture>

# Météo

### `@portaki/module-weather`

[![npm](https://img.shields.io/npm/v/@portaki/module-weather?label=npm&logo=npm&color=CB3837)](https://www.npmjs.com/package/@portaki/module-weather)
[![license](https://img.shields.io/badge/license-AGPL--3.0-blue)](https://opensource.org/licenses/AGPL-3.0)
[![SDK](https://img.shields.io/badge/built%20with-%40portaki%2Fmodule--sdk-181717?logo=github)](https://github.com/PortakiApp/portaki-sdk)

*Prévisions locales (Open-Meteo, sans clé API)*

</div>

---

> **En une phrase** — Affiche **1 à 7 jours** de températures (min/max) via **Open-Meteo** ; coordonnées = champs module **ou** `property.lat` / `property.lng` passés par l’app invité.

## Fiche technique

| Clé | Valeur |
|-----|--------|
| **npm** | `@portaki/module-weather` |
| **`id`** | `weather` |
| **Slot nav** | `section` |
| **Icône** | `cloud-sun` |
| **Manifeste** | [`portaki.module.json`](./portaki.module.json) |

---

## Champs hôte

| Champ | Rôle |
|--------|------|
| `latitude` / `longitude` | Optionnels — sinon coordonnées du bien. |
| `location_label` | Libellé affiché au-dessus des jours. |
| `forecast_days` | 1 à 7 (défaut 3). |
| `intro` | Texte libre au-dessus des températures. |

Données **Open-Meteo** (CC BY 4.0). Schéma aligné sur **`official-modules/weather.json`**.

---

## Développement local

```bash
cd portaki-sdk && pnpm install && pnpm run validate:modules
```

---

## Licence

**AGPL-3.0** — voir `package.json`.
