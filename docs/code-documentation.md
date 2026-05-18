# Documentation du code — conventions et génération

Documentation **institutionnelle** du SDK Portaki : commentaires source alignés sur **Doxygen**, génération **TypeDoc** pour la référence API TypeScript, et site **VitePress** pour les guides.

## Conventions dans le code

### En-tête de fichier

```typescript
/**
 * @file define-module.ts
 * @brief Module authoring entry point (UI, schema, gateway handlers, extensions).
 * @copyright Portaki — SPDX-License-Identifier: MIT
 * @addtogroup module_authoring Module authoring API
 * @{
 */
```

### Symboles publics

| Balise | Usage |
|--------|--------|
| `@brief` | Une ligne — résumé affiché dans l’index |
| `@details` / `@remarks` | Comportement, contraintes plateforme |
| `@param` | Paramètres de fonction |
| `@returns` | Valeur de retour |
| `@throws` | Erreurs contractuelles |
| `@example` | Extrait d’usage (module auteur) |
| `@see` | Liens vers manifeste, runtime, autre symbole |
| `@defgroup` / `@addtogroup` | Regroupement logique (schema, gateway, extensions) |

Les types exportés documentent chaque champ avec `/** … */` au-dessus de la propriété.

### Périmètre documenté

| Package | Rôle |
|---------|------|
| `@portaki/sdk` | `defineModule`, schema DSL, gateway (`ctx.db`), UI, extensions hôte |
| `@portaki/cli` | `portaki build`, bundles `.portaki/` |
| `@portaki/sdk-test-support` | Fixtures Vitest, validation manifeste, rendu invité/hôte |
| `examples/rules` | Module de référence pour auteurs |

## TypeDoc (référence API — recommandé pour TypeScript)

```bash
pnpm docs:api
```

Sortie : `docs-site/public/api/` (lien « API » dans la barre VitePress).

Configuration : `typedoc.json` à la racine du monorepo.

## Doxygen (standard entreprise)

```bash
pnpm docs:doxygen
```

Sortie : `build/doxygen/html/index.html`.

Configuration : `Doxyfile` (entrées `packages/sdk/src`, `packages/cli/src`, `packages/sdk-test-support/src`).

> Doxygen interprète les blocs JSDoc/TSDoc ; pour une navigation TypeScript optimale, privilégier TypeDoc en complément.

## Site de guides (VitePress)

```bash
pnpm docs:dev      # http://localhost:5173
pnpm docs:build    # docs-site/.vitepress/dist
```

Contenu : `docs-site/src/` + guides importés depuis `docs/*.md`.

## Revue avant release

1. `pnpm docs:api` sans warning bloquant.
2. `pnpm docs:build` — site statique vert.
3. Vérifier que les symboles publics exportés par `@portaki/sdk` ont un `@brief`.
