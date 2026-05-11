# Événements (`@portakiapp/module-events`)

> **Annoncer et localiser ce qui se passe autour du logement** — section dédiée + point d’accroche pour la carte.

## Public cible

Voyageurs en recherche d’activités, événements locaux, ou repères sur la carte pendant le séjour.

## Ce que ça apporte

- Entrée de menu « Événements » cohérente avec les autres modules.
- Préparation d’**overlays carte** via `mapOverlay` et le hook `mapMarkers` (à alimenter côté data).
- Contenu extensible dans `EventsSection` (listes, liens, partenaires).

## Fiche technique

| Champ | Valeur |
|--------|--------|
| **Package npm** | `@portakiapp/module-events` |
| **Identifiant `id`** | `events` |
| **Slot navigation** | `section` |
| **Icône** | `calendar` |
| **Visibilité** | Toujours affiché |
| **Carte / carte overlay** | Oui — `mapOverlay: true` ; `mapMarkers` async (retour typé côté app) |

## Intégration Portaki

`render` reçoit `property.id` pour charger les contenus. La fonction `mapMarkers` est prête à retourner des marqueurs ; implémentez la récupération d’événements selon votre API.

## Données & API

Brancher les endpoints guest listant les événements (par `propertyId` ou zone). Le composant actuel sert de structure d’accueil.

## Développement local

Depuis la racine du monorepo :

```bash
pnpm install
```

Ce package dépend de **`@portakiapp/module-sdk`** (publié depuis [portaki-sdk](https://github.com/PortakiApp/portaki-sdk)).

## Licence

AGPL-3.0 — voir le `package.json`.
