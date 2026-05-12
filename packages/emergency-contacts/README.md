<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/PortakiApp/portaki-sdk/develop/docs/assets/portaki-wordmark-light.svg">
  <img src="https://portaki.app/portaki-wordmark.svg" width="160" height="44" alt="Portaki" />
</picture>

# Urgences & utiles

### `@portaki/module-emergency-contacts`

[![npm](https://img.shields.io/npm/v/@portaki/module-emergency-contacts?label=npm&logo=npm&color=CB3837)](https://www.npmjs.com/package/@portaki/module-emergency-contacts)
[![license](https://img.shields.io/badge/license-AGPL--3.0-blue)](https://opensource.org/licenses/AGPL-3.0)
[![SDK](https://img.shields.io/badge/built%20with-%40portaki%2Fmodule--sdk-181717?logo=github)](https://github.com/PortakiApp/portaki-sdk)

*Numéros d’urgence, pharmacie, ligne hôte — liens `tel:`*

</div>

---

> **En une phrase** — **`contacts_json`** structure les lignes (libellé FR/EN, téléphone, catégorie, note) ; **`host_visible_phone`** ajoute une ligne hôte en texte simple.

## Fiche technique

| Clé | Valeur |
|-----|--------|
| **npm** | `@portaki/module-emergency-contacts` |
| **`id`** | `emergency-contacts` |
| **Slot nav** | `section` |
| **Icône** | `phone` |
| **Manifeste** | [`portaki.module.json`](./portaki.module.json) |

---

## Rappel réglementaire

En cas d’urgence vitale, le **112** (UE) ou le numéro local approprié — le module rappelle ce message en page invité.

---

## Champs hôte

| Champ | Rôle |
|--------|------|
| `contacts_json` | Tableau JSON (voir manifeste + API). |
| `host_visible_phone` | Téléphone hôte affiché en complément. |

Schéma aligné sur **`official-modules/emergency-contacts.json`**.

---

## Développement local

```bash
cd portaki-sdk && pnpm install && pnpm run validate:modules
```

---

## Licence

**AGPL-3.0** — voir `package.json`.
