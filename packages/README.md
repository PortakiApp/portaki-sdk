# Manifests `portaki.module.json` (gateway + whitelist)

Les définitions **queries / commands / scopes** consommées par `portaki-web` (`generate-whitelist.mjs`) et par la validation côté API (`OfficialModuleGatewayManifests`) vivent ici sous **`portaki-sdk/packages/<module-id>/`**.

Le dépôt **`portaki-modules`** reste la source des **paquets npm** (`@portaki/module-*`) et du code frontend ; les manifests « gateway » sont dupliqués ou étendus ici pour le monorepo `Repositories`. Le script whitelist fusionne aussi `../portaki-modules/modules/*/portaki.module.json` lorsqu’il expose des `queries` / `commands`.
