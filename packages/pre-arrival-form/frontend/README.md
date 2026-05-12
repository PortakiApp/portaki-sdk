<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/PortakiApp/portaki-sdk/develop/docs/assets/portaki-wordmark-light.svg">
  <img src="https://portaki.app/portaki-wordmark.svg" width="160" height="44" alt="Portaki" />
</picture>

# 📋 Pré-arrivée · Package npm

### `@portaki/module-pre-arrival-form`

[![npm](https://img.shields.io/npm/v/@portaki/module-pre-arrival-form?label=npm&logo=npm&color=CB3837)](https://www.npmjs.com/package/@portaki/module-pre-arrival-form)
[![license](https://img.shields.io/badge/license-AGPL--3.0-blue)](https://opensource.org/licenses/AGPL-3.0)
[![Module complet](https://img.shields.io/badge/doc-module%20full--stack-6366f1)](../README.md)

*Partie **React** publiée sur npm — le backend Java vit dans [`../README.md`](../README.md)*

</div>

---

> 🎯 **En une phrase** — Export **`definePortakiModule`** pour le formulaire lié au séjour ; masquage auto hors statuts **`PRE_ARRIVAL`** / **`UPCOMING`**.

## 📌 À savoir

| Sujet | Détail |
|-------|--------|
| 🎯 **Public** | Identique au module complet — ici on documente **uniquement le paquet UI** |
| 📦 **Publication** | Ce dossier est ce qui est publié sous **`@portaki/module-pre-arrival-form`** |

---

## ✨ Fonctionnalités

- [x] **Default export** `definePortakiModule`
- [x] Masquage si le séjour n’est pas aux statuts attendus
- [x] Composant **`PreArrivalForm`** · props **`stayId`**, **`lang`**

---

## 🧭 Fiche technique

| Clé | Valeur |
|-----|--------|
| 📦 **npm** | `@portaki/module-pre-arrival-form` |
| 🆔 **`id`** | `pre-arrival-form` |
| 📍 **Slot nav** | `section` |
| 🎨 **Icône** | `clipboard-list` |
| 📄 **Manifeste** | [`../portaki.module.json`](../portaki.module.json) |
| 👁️ **Visibilité** | `visibleOnStatus: ['PRE_ARRIVAL', 'UPCOMING']` |
| 🗺️ **Carte** | Non |

---

## 🔌 Intégration Portaki

Import du default export comme les autres **`@portaki/module-*`**.

## 📡 Données & API

Brancher chargement / soumission sur tes routes guest (étapes du formulaire).

---

## 🛠️ Développement local

```bash
pnpm install   # racine du monorepo portaki-sdk
```

🔧 **`@portaki/module-sdk`** → [**portaki-sdk**](https://github.com/PortakiApp/portaki-sdk)

---

## 📄 Licence

**AGPL-3.0** — voir `package.json`.
