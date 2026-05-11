# Formulaire avant arrivée — package npm (`@portakiapp/module-pre-arrival-form`)

> **Partie UI du module pré-arrivée** — export npm consommé par l’application guest ; le backend Java est documenté dans [../README.md](../README.md).

## Public cible

Même que le module complet : voyageurs en amont du séjour ; cette page documente uniquement le **package React** publié sur npm.

## Ce que ça apporte

- Export par défaut `definePortakiModule` pour le formulaire lié au séjour.
- Masquage automatique lorsque le séjour n’est pas aux statuts configurés.

## Fiche technique

| Champ | Valeur |
|--------|--------|
| **Package npm** | `@portakiapp/module-pre-arrival-form` |
| **Identifiant `id`** | `pre-arrival-form` |
| **Slot navigation** | `section` |
| **Icône** | `clipboard-list` |
| **Visibilité** | `visibleOnStatus: ['PRE_ARRIVAL', 'UPCOMING']` |
| **Carte / carte overlay** | Non |

## Intégration Portaki

Importer le default export comme pour les autres `@portakiapp/module-*`. Le composant `PreArrivalForm` reçoit `stayId` et `lang`.

## Données & API

À connecter aux routes guest de soumission / chargement d’étapes du formulaire.

## Développement local

Depuis la racine du monorepo :

```bash
pnpm install
```

Voir aussi le dépôt **[portaki-sdk](https://github.com/PortakiApp/portaki-sdk)** pour `@portakiapp/module-sdk`.

## Licence

AGPL-3.0 — voir le `package.json`.
