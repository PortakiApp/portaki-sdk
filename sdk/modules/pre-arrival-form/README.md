# Formulaire avant arrivée (`pre-arrival-form`)

> **Collecter les informations utiles avant l’entrée dans les lieux** — module **full stack** : package npm React + service Java pour les événements de séjour.

## Public cible

Voyageurs en phase **pré-arrivée** ou **à venir**, et équipes ops qui déclenchent les flux à partir des événements métier (`StayCreatedEvent`, etc.).

## Ce que ça apporte

- Formulaire contextualisé par **`stayId`** dans l’app guest.
- Affichage limité aux statuts **`PRE_ARRIVAL`** et **`UPCOMING`**.
- Backend Java prêt à consommer les événements Axon et exposer les commandes nécessaires (voir `backend/`).

## Fiche technique

| Champ | Valeur |
|--------|--------|
| **Package npm (UI)** | `@portakiapp/module-pre-arrival-form` (dossier `frontend/`) |
| **Identifiant `id`** | `pre-arrival-form` |
| **Slot navigation** | `section` |
| **Icône** | `clipboard-list` |
| **Visibilité** | `visibleOnStatus: ['PRE_ARRIVAL', 'UPCOMING']` |
| **Backend** | `backend/` — Maven, module `portaki-module-sdk` Java, Spring |

## Intégration Portaki

- **Front** : même pattern que les autres modules (`definePortakiModule`, export par défaut). Sans `stay`, le rendu est vide.
- **Back** : référencer `PreArrivalFormModule` et les handlers prévus dans votre gateway Axon / bus.

## Données & API

Brancher les endpoints guest pour soumission du formulaire ; aligner les DTO avec le backend Java du dossier `backend/`.

## Développement local

### Frontend (npm)

À la racine du monorepo :

```bash
pnpm install
```

README détaillé du package : [frontend/README.md](./frontend/README.md).

### Backend (Java)

```bash
cd backend
mvn -q compile
```

Ce package dépend de **`@portakiapp/module-sdk`** côté frontend (publié depuis [portaki-sdk](https://github.com/PortakiApp/portaki-sdk)).

## Licence

AGPL-3.0 — packages npm et composants Java selon les métadonnées du dépôt.
