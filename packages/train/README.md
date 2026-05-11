# Trains (`@portakiapp/module-train`)

> **Horaires et informations trains au départ de la gare configurée** — s’appuie sur l’écosystème Navitia / données voyageurs.

## Public cible

Voyageurs qui consultent l’app guest pour préparer un trajet depuis le logement (gare la plus pertinente configurée côté propriété).

## Ce que ça apporte

- Vue dédiée « Trains » dans la navigation guest.
- Contextualisation avec le **code gare** (`trainStationCode`) porté par la fiche propriété.
- Base prête pour brancher les API guest Navitia / SNCF existantes.

## Fiche technique

| Champ | Valeur |
|--------|--------|
| **Package npm** | `@portakiapp/module-train` |
| **Identifiant `id`** | `train` |
| **Slot navigation** | `section` |
| **Icône** | `train` |
| **Visibilité** | Toujours affiché (pas de `visibleOnStatus`) |
| **Carte / carte overlay** | Non |

## Intégration Portaki

Le module exporte un **default export** créé avec `definePortakiModule`. Le rendu reçoit `property.trainStationCode` pour afficher ou masquer le détail selon la configuration.

## Données & API

Données voyageurs via les endpoints guest déjà prévus (`GET` Navitia / hub SNCF selon votre infra). Aujourd’hui l’UI est un gabarit ; branchez vos appels dans `TrainSection`.

## Développement local

Depuis la racine du monorepo :

```bash
pnpm install
```

Ce package dépend de **`@portakiapp/module-sdk`** (publié depuis [portaki-sdk](https://github.com/PortakiApp/portaki-sdk)).

## Licence

AGPL-3.0 — voir le `package.json`.
