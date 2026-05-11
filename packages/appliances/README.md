<div align="center">

# 🔌 Module Appareils

### `@portaki/module-appliances`

[![npm](https://img.shields.io/npm/v/@portaki/module-appliances?label=npm&logo=npm&color=CB3837)](https://www.npmjs.com/package/@portaki/module-appliances)
[![license](https://img.shields.io/badge/license-AGPL--3.0-blue)](https://opensource.org/licenses/AGPL-3.0)
[![SDK](https://img.shields.io/badge/built%20with-%40portaki%2Fmodule--sdk-181717?logo=github)](https://github.com/PortakiApp/portaki-sdk)

*Four, lave-linge, chauffage… guide éditorial par équipement*

</div>

---

> 🎯 **En une phrase** — Aide les voyageurs à **utiliser les équipements** avec consignes courtes et contenu enrichissable.

## 👥 Pour qui ?

| Persona | Besoin |
|---------|--------|
| 🏠 **Voyageurs** | Mode d’emploi express sur place |
| 📝 **Contenu** | Liste d’appareils + médias par propriété |

## ✨ Ce que le module apporte

- [x] Section **« Appareils »** dans la nav guest
- [x] Structure pour guide riche (**TipTap**, médias, FAQ courte)

---

## 🧭 Fiche technique

| Clé | Valeur |
|-----|--------|
| 📦 **npm** | `@portaki/module-appliances` |
| 🆔 **`id`** | `appliances` |
| 📍 **Slot nav** | `section` |
| 🎨 **Icône** | `plug` |
| 👁️ **Visibilité** | Toujours affiché |
| 🗺️ **Carte** | Non |

---

## 🔌 Intégration Portaki

Branche CMS / JSON propriété dans le JSX pour remplacer la démo.

## 📡 Données & API

Contenus **par propriété** : liste d’appareils, notices, liens externes.

---

## 🛠️ Développement local

```bash
pnpm install   # racine du monorepo portaki-sdk
```

Dépend de **`@portaki/module-sdk`** → [**portaki-sdk**](https://github.com/PortakiApp/portaki-sdk).

---

## 📄 Licence

**AGPL-3.0** — voir `package.json`.
