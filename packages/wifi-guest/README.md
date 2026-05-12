<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/PortakiApp/portaki-sdk/develop/docs/assets/portaki-wordmark-light.svg">
  <img src="https://portaki.app/portaki-wordmark.svg" width="160" height="44" alt="Portaki" />
</picture>

# Wi‑Fi invité

### `@portaki/module-wifi-guest`

[![npm](https://img.shields.io/npm/v/@portaki/module-wifi-guest?label=npm&logo=npm&color=CB3837)](https://www.npmjs.com/package/@portaki/module-wifi-guest)
[![license](https://img.shields.io/badge/license-AGPL--3.0-blue)](https://opensource.org/licenses/AGPL-3.0)
[![SDK](https://img.shields.io/badge/built%20with-%40portaki%2Fmodule--sdk-181717?logo=github)](https://github.com/PortakiApp/portaki-sdk)

*SSID, mot de passe et aide à la connexion — réseau dédié voyageurs*

</div>

---

> **En une phrase** — Affiche le **réseau invité** sur la page séjour : le mot de passe est **déchiffré côté serveur** puis montré à l’invité ; prévoir un SSID / mot de passe **distincts** du réseau personnel.

## Fiche technique

| Clé | Valeur |
|-----|--------|
| **npm** | `@portaki/module-wifi-guest` |
| **`id`** | `wifi-guest` |
| **Slot nav** | `section` |
| **Icône** | `wifi` |
| **Manifeste** | [`portaki.module.json`](./portaki.module.json) |

---

## Configuration (hôte)

| Champ | Rôle |
|--------|------|
| `ssid` | Nom du réseau (SSID) — voir libellés FR/EN dans le manifeste. |
| `password` | Secret chiffré au repos ; visible uniquement sur la page invité. |
| `band_hint` | Optionnel — 2,4 / 5 GHz, mesh, etc. |
| `connection_steps` | Optionnel — portail captif, WPS, consignes longues. |

Aligné sur le schéma API **`official-modules/wifi-guest.json`** (Portaki API).

---

## Sécurité

Ne réutilisez pas le mot de passe du réseau personnel. Préférez VLAN ou SSID « Guest » sur le routeur.

---

## Développement local

```bash
cd portaki-sdk && pnpm install && pnpm run validate:modules
```

---

## Licence

**AGPL-3.0** — voir `package.json`.
