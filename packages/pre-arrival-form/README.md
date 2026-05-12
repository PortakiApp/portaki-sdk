<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/PortakiApp/portaki-sdk/develop/docs/assets/portaki-wordmark-light.svg">
  <img src="https://portaki.app/portaki-wordmark.svg" width="160" height="44" alt="Portaki" />
</picture>

# 📋 Module Pré-arrivée · Full stack

### `@portaki/module-pre-arrival-form`

[![Frontend npm](https://img.shields.io/npm/v/@portaki/module-pre-arrival-form?label=%40portaki%2Fmodule-pre-arrival-form&logo=npm&color=CB3837)](https://www.npmjs.com/package/@portaki/module-pre-arrival-form)
[![license](https://img.shields.io/badge/license-AGPL--3.0-blue)](https://opensource.org/licenses/AGPL-3.0)
[![SDK](https://img.shields.io/badge/monorepo-portaki--sdk-181717?logo=github)](https://github.com/PortakiApp/portaki-sdk)

*React **+** backend Java · événements séjour · Axon-ready*

</div>

---

> 🎯 **En une phrase** — Collecte les infos **avant l’entrée dans les lieux** : UI guest (`definePortakiModule`) + **`backend/`** Java pour les événements métier.

## 👥 Pour qui ?

| Persona | Rôle |
|---------|------|
| 🧳 **Voyageurs** | Statuts **`PRE_ARRIVAL`** · **`UPCOMING`** |
| ⚙️ **Ops / backend** | Réactions aux events (`StayCreatedEvent`, …) |

## ✨ Ce que le module apporte

| Couche | Détail |
|--------|--------|
| 🖥️ **Front** | Formulaire contextualisé par **`stayId`** |
| ☕ **Back** | Module Maven prêt à consommer **Axon** / bus interne |
| 🔒 **UX** | Masqué hors statuts configurés |

---

## 🧭 Fiche technique

| Clé | Valeur |
|-----|--------|
| 📦 **npm (UI)** | `@portaki/module-pre-arrival-form` → dossier [`frontend/`](./frontend/) |
| 🆔 **`id`** | `pre-arrival-form` |
| 📍 **Slot nav** | `section` |
| 🎨 **Icône** | `clipboard-list` |
| 📄 **Manifeste** | [`portaki.module.json`](./portaki.module.json) |
| 👁️ **Visibilité** | `visibleOnStatus: ['PRE_ARRIVAL', 'UPCOMING']` |
| ☕ **Backend** | [`backend/`](./backend/) — Maven, `portaki-module-sdk` Java, Spring |

---

## 🔌 Intégration Portaki

1. **Front** — même pattern que les autres `@portaki/module-*` · sans `stay`, rendu vide.
2. **Back** — référencer **`PreArrivalFormModule`** et les handlers prévus (gateway / Axon).

## 📡 Données & API

Aligner les DTO front ↔ routes guest ↔ handlers **`backend/`**.

---

## 🛠️ Développement local

### Frontend

```bash
pnpm install    # racine portaki-sdk
```

📎 Détail du paquet npm → **[frontend/README.md](./frontend/README.md)**

### Backend

```bash
cd backend && mvn -q compile
```

---

## 📄 Licence

**AGPL-3.0** — métadonnées npm & dépôt Java selon les manifests du monorepo.
