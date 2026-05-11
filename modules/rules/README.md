# Règlement intérieur (`@portakiapp/module-rules`)

> **Rendre accessibles les règles de la maison** — contenu riche (TipTap / éditeur) à connecter à votre CMS ou API.

## Public cible

Voyageurs qui doivent connaître le règlement (bruit, fumeur, tri, etc.) avant ou pendant le séjour.

## Ce que ça apporte

- Section claire **House rules** / **Règlement** dans l’app guest.
- Emplacement `data-module="rules"` pour le style et l’analytics.
- Base prête pour brancher le contenu TipTap ou le HTML issu de votre back-office.

## Fiche technique

| Champ | Valeur |
|--------|--------|
| **Package npm** | `@portakiapp/module-rules` |
| **Identifiant `id`** | `rules` |
| **Slot navigation** | `section` |
| **Icône** | `scale` |
| **Visibilité** | Toujours affiché |
| **Carte / carte overlay** | Non |

## Intégration Portaki

Le module ne dépend pas encore d’une source de contenu : remplacez le texte factice par la chaîne / document fourni par l’API guest.

## Données & API

Prévoir un endpoint (ou un champ `property`) transportant le document de règlement structuré ou HTML sanitisé.

## Développement local

Depuis la racine du monorepo :

```bash
pnpm install
```

Ce package dépend de **`@portakiapp/module-sdk`** (publié depuis [portaki-sdk](https://github.com/PortakiApp/portaki-sdk)).

## Licence

AGPL-3.0 — voir le `package.json`.
