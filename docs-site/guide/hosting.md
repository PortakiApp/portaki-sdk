# Hébergement de la documentation Portaki SDK

Comparatif des plateformes pour une doc **hébergée**, versionnée avec le dépôt, et déployable sur **Vercel** (ou équivalent).

## Recommandation Portaki

| Besoin | Outil | Hébergement |
|--------|--------|-------------|
| Guides + référence API depuis le code | **VitePress** (ce dépôt, `docs-site/`) + **TypeDoc** | Vercel (statique) |
| Commentaires source « institutionnels » | **TSDoc / JSDoc** (compatible **Doxygen**) | Génération locale / CI |
| Édition non technique, espaces d’équipe | GitBook, Notion, Mintlify | SaaS (+ domaine custom) |

Le site **`docs-site/`** de ce monorepo est la cible officielle pour Vercel. Les guides existants sous `docs/` y sont intégrés.

## Alternatives à GitBook

### Hébergeables sur Vercel (statique ou Next.js)

| Plateforme | Stack | Points forts | Limites |
|------------|--------|--------------|---------|
| **[VitePress](https://vitepress.dev/)** | Vue + Vite | Rapide, Markdown, search intégré, idéal monorepo | Moins « WYSIWYG » que GitBook |
| **[Nextra](https://nextra.site/)** | Next.js | App Router, très intégré écosystème Vercel | Config plus lourde |
| **[Fumadocs](https://fumadocs.dev/)** | Next.js | Moderne, MDX, composants doc | Courbe d’apprentissage |
| **[Starlight](https://starlight.astro.build/)** | Astro | Excellent pour docs techniques, i18n | Pas Next (mais Vercel OK) |
| **[Docusaurus](https://docusaurus.io/)** | React | Mature, versioning, blog | Bundle plus gros |
| **TypeDoc** (HTML) | — | API 100 % depuis TypeScript | Guides à côté (VitePress) |
| **Doxygen** (HTML) | — | Standard entreprise, `@brief` / `@defgroup` | TS moins natif que TypeDoc |

### SaaS (domaine custom, hors build Vercel)

| Plateforme | Notes |
|------------|--------|
| **[GitBook](https://www.gitbook.com/)** | Sync GitHub, bel rendu, espaces équipe ; hébergement GitBook (custom domain possible). |
| **[Mintlify](https://mintlify.com/)** | Très soigné pour API produit ; déploiement Mintlify ou `docs.json` dans le repo. |
| **ReadMe** | Orienté API publique / développeurs externes. |
| **Notion + Super/Potion** | Rapide en interne, moins « doc produit » versionnée. |

## Déploiement Vercel (`docs-site/`)

1. Projet Vercel → repo `portaki-sdk`, répertoire racine **`docs-site`**.
2. Build : `pnpm install && pnpm build`
3. Output : **`.vitepress/dist`**
4. Variables : aucune obligatoire pour la doc statique.

Preview PR : activer « Vercel for GitHub » sur le même projet.

## Génération API depuis le code

Voir **[code-documentation.md](./code-documentation.md)** :

- `pnpm docs:api` → TypeDoc → `docs-site/src/api-reference/`
- `pnpm docs:doxygen` → Doxygen → `build/doxygen/html/` (optionnel)

Les commentaires dans `packages/sdk`, `packages/cli` et `packages/sdk-test-support` suivent le style **Doxygen** (`@file`, `@brief`, `@param`, `@defgroup`, …) pour revue humaine et génération.
