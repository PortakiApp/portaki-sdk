# Manifests `portaki.module.json` (gateway + whitelist)

Les définitions **queries / commands / scopes** consommées par `portaki-web` (`generate-whitelist.mjs`) et par la validation côté API (`OfficialModuleGatewayManifests`) vivent ici sous **`portaki-sdk/packages/<module-id>/`**.

Le dépôt **`portaki-modules`** reste la source des **paquets npm** (`@portaki/module-*`) et du code frontend ; les manifests « gateway » sont dupliqués ou étendus ici pour le monorepo `Repositories`. Le script whitelist fusionne aussi `../portaki-modules/modules/*/portaki.module.json` lorsqu’il expose des `queries` / `commands`.

## Aperçus voyageur (documentation)

Des **illustrations SVG factices** (faux téléphone, alignés sur `portaki-web/public/design-handoff/guest-modules-section.jsx`) sont générés dans le dépôt **`portaki-web`** :

- Dossier : [`../portaki-web/public/module-previews/`](../portaki-web/public/module-previews/)
- Régénération : `cd ../portaki-web && pnpm run generate:module-previews`

Les README du dépôt **`portaki-modules`** référencent ces fichiers pour les vignettes GitHub.
