# Checklist départ (`@portakiapp/module-checklist`)

> **Accompagner le voyageur pour une sortie sans oubli** — liste issue de la configuration propriété et persistance via l’API guest.

## Public cible

Voyageurs en fin de séjour qui doivent valider les étapes de départ (clés, déchets, linge, etc.).

## Ce que ça apporte

- Affichage conditionnel : réservé aux séjours au statut **`ACTIVE`** (`visibleOnStatus`).
- Libellés **FR / EN** par item (`checklistItems` sur la propriété).
- Point d’accroche pour `POST` guest par item (`stayId` + `itemId`).

## Fiche technique

| Champ | Valeur |
|--------|--------|
| **Package npm** | `@portakiapp/module-checklist` |
| **Identifiant `id`** | `checklist` |
| **Slot navigation** | `section` |
| **Icône** | `check-square` |
| **Visibilité** | `visibleOnStatus: ['ACTIVE']` |
| **Carte / carte overlay** | Non |

## Intégration Portaki

Si `stay` est absent, le module ne rend rien. Sinon `ChecklistSection` reçoit `stay.id` et la liste `property.checklistItems`.

## Données & API

Persistance décrite côté produit comme `POST .../checklist/{itemId}` sur le contexte guest. Adaptez les appels dans le composant lorsque les routes sont figées.

## Développement local

Depuis la racine du monorepo :

```bash
pnpm install
```

Ce package dépend de **`@portakiapp/module-sdk`** (publié depuis [portaki-sdk](https://github.com/PortakiApp/portaki-sdk)).

## Licence

AGPL-3.0 — voir le `package.json`.
