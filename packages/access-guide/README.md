<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/PortakiApp/portaki-sdk/develop/docs/assets/portaki-wordmark-light.svg">
  <img src="https://portaki.app/portaki-wordmark.svg" width="160" height="44" alt="Portaki" />
</picture>

# Accès & parking

### `@portaki/module-access-guide`

[![npm](https://img.shields.io/npm/v/@portaki/module-access-guide?label=npm&logo=npm&color=CB3837)](https://www.npmjs.com/package/@portaki/module-access-guide)
[![license](https://img.shields.io/badge/license-AGPL--3.0-blue)](https://opensource.org/licenses/AGPL-3.0)
[![SDK](https://img.shields.io/badge/built%20with-%40portaki%2Fmodule--sdk-181717?logo=github)](https://github.com/PortakiApp/portaki-sdk)

*Du parking à la porte : étapes, carte, vidéo*

</div>

---

> **En une phrase** — **`steps_json`** liste les étapes (`kind`: parking, door, elevator, other) avec titres et détails FR/EN ; liens **carte** / **vidéo** et **note globale** optionnels.

## Fiche technique

| Clé | Valeur |
|-----|--------|
| **npm** | `@portaki/module-access-guide` |
| **`id`** | `access-guide` |
| **Slot nav** | `section` |
| **Icône** | `car-front` |
| **Manifeste** | [`portaki.module.json`](./portaki.module.json) |

---

## Champs hôte

| Champ | Rôle |
|--------|------|
| `steps_json` | JSON obligatoire — voir exemple dans le manifeste. |
| `parking_map_url` | Carte / Google Maps / Street View. |
| `arrival_video_url` | YouTube, Loom, etc. |
| `global_note` | Note libre (sonnette, digicode général, …). |

Schéma aligné sur **`official-modules/access-guide.json`**.

---

## Développement local

```bash
cd portaki-sdk && pnpm install && pnpm run validate:modules
```

---

## Licence

**AGPL-3.0** — voir `package.json`.
