# Appareils (`@portakiapp/module-appliances`)

> **Guide des équipements du logement** — four, lave-linge, chauffage, etc., avec contenu éditorial.

## Public cible

Voyageurs qui utilisent les équipements sur place et ont besoin d’instructions courtes et fiables.

## Ce que ça apporte

- Section « Appareils » dans la navigation guest.
- Structure prête pour un guide riche (TipTap, médias, FAQ courte par appareil).

## Fiche technique

| Champ | Valeur |
|--------|--------|
| **Package npm** | `@portakiapp/module-appliances` |
| **Identifiant `id`** | `appliances` |
| **Slot navigation** | `section` |
| **Icône** | `plug` |
| **Visibilité** | Toujours affiché |
| **Carte / carte overlay** | Non |

## Intégration Portaki

Branchez votre source de vérité (CMS, JSON propriété) dans le JSX du module pour remplacer le texte de démonstration.

## Données & API

Prévoir des contenus par propriété (liste d’appareils, notices, liens).

## Développement local

Depuis la racine du monorepo :

```bash
pnpm install
```

Ce package dépend de **`@portakiapp/module-sdk`** (publié depuis [portaki-sdk](https://github.com/PortakiApp/portaki-sdk)).

## Licence

AGPL-3.0 — voir le `package.json`.
