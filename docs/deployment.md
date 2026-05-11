# Déploiement — npmjs & Maven Central

Les workflows publient :

- **npm** : registre public **[registry.npmjs.org](https://www.npmjs.com/)** en CI via **[Trusted Publishing](https://docs.npmjs.com/trusted-publishers)** (OIDC GitHub Actions : permission **`id-token: write`**, **Node 24** pour le **npm 11.5.x** fourni avec Node — pas de **`NPM_TOKEN`** dans les jobs de publication).
- **Maven** : **Sonatype OSSRH** → **Maven Central** avec **`OSSRH_USERNAME`** et **`OSSRH_TOKEN`**.

---

## Publier sur npmjs (résumé)

1. **Compte npm** : créer un compte sur [npmjs.com](https://www.npmjs.com/). Pour un scope **`@portaki/*`**, créer l’[organisation](https://docs.npmjs.com/creating-an-organization) **portaki** (ou utiliser un scope personnel si vous acceptez de changer les noms de paquets).
2. **CI — Trusted Publishing** : pour chaque paquet publié par GitHub Actions, dans les **paramètres du paquet** sur npm → **Trusted Publisher** : **GitHub Actions**, org **`PortakiApp`**, dépôt **`portaki-sdk`**, et le **nom du fichier workflow** exact (`publish-npm-sdk.yml` pour `@portaki/module-sdk`, `publish-npm-packages.yml` pour les modules sous `packages/`). Optionnel : **Environment name** si le job utilise un [environment GitHub](https://docs.github.com/en/actions/deployment/targeting-different-environments/using-environments-for-deployment) du même nom.
3. **Publication locale / secours** : npm → **Access Tokens** (granulaire avec **Publish** sur le scope **`@portaki`**) si vous publiez hors CI ou sans Trusted Publishing ; config locale : `npm config set //registry.npmjs.org/:_authToken=YOUR_TOKEN` (ne pas committer).
4. **Version dans le dépôt** : bump **`version`** dans `sdk/javascript/package.json` (SDK) ou dans chaque `packages/<module>/package.json` **avant** publication — la CI **ne modifie pas** ces fichiers.
5. **Déclencher une publication** :
   - **Manuel** : **Actions** → **publish-npm-sdk** ou **publish-npm-packages** → **Run workflow**.
   - **Release GitHub** : uniquement pour le SDK JS, un tag du type **`javascript-v…`**, **`js-v…`** ou **`sdk-js-v…`** déclenche [`publish-npm-sdk.yml`](../.github/workflows/publish-npm-sdk.yml) (aucun bump automatique ; la version publiée est celle déjà dans `package.json`).
6. **Localement** (sans CI), depuis la racine du paquet :

   ```bash
   cd sdk/javascript   # ou un dossier sous packages/<nom>/
   npm login           # une fois
   npm publish --access public
   ```

---

## Secrets GitHub

| Secret | Utilisation |
|--------|-------------|
| *(aucun pour npm en CI)* | La publication npm utilise **Trusted Publishing** (OIDC), pas **`NPM_TOKEN`**. |
| `OSSRH_USERNAME` | Identifiant Sonatype (OSSRH / Central Portal). |
| `OSSRH_TOKEN` | Mot de passe ou token Sonatype. |

Les **releases Maven** non-SNAPSHOT peuvent exiger signature GPG selon la politique Sonatype — à ajouter au `pom.xml` si besoin.

---

## Workflows (fichiers en slug)

| Fichier | Rôle |
|---------|------|
| [`ci-verify.yml`](../.github/workflows/ci-verify.yml) | Vérification : SDK JS, SDK Java, lint `packages/`, backend pre-arrival Maven si chemins concernés. |
| [`publish-npm-sdk.yml`](../.github/workflows/publish-npm-sdk.yml) | Publie **`@portaki/module-sdk`** (`sdk/javascript`). |
| [`publish-npm-packages.yml`](../.github/workflows/publish-npm-packages.yml) | Publie manuellement les **`@portaki/module-*`** sous `packages/`. |
| [`publish-maven-sdk.yml`](../.github/workflows/publish-maven-sdk.yml) | Déploie **`app.portaki:portaki-module-sdk`** (`sdk/java`) vers OSSRH. |

### CI — `ci-verify.yml`

Déclenché sur `push` / `pull_request` vers `main` et `develop` lorsque `sdk/**`, `packages/**`, fichiers workspace racine ou ce workflow changent.

Jobs (IDs stables) : **`detect_changes`**, **`sdk_javascript`**, **`sdk_java`**, **`workspace_packages`**, **`pre_arrival_java`**, pilotés par [`dorny/paths-filter`](https://github.com/dorny/paths-filter).

### Publication SDK JS — `publish-npm-sdk.yml`

**Paquet :** `@portaki/module-sdk`.

**Version :** celle de **`sdk/javascript/package.json`** (à faire évoluer dans une PR avant publication).

**Déclencheurs :** **`workflow_dispatch`** ; ou **`release`** **`published`** dont le tag commence par **`javascript-v`**, **`js-v`** ou **`sdk-js-v`** (évite de lancer une publication npm sur une release Maven `java-v…`).

### Publication paquets invités — `publish-npm-packages.yml`

**Uniquement `workflow_dispatch`** : choix **`npm_package`** (`all` ou un `@portaki/module-…`).

**Matrice :** un job par paquet ; avec **`all`**, les publications s’exécutent **en parallèle** (`fail-fast: false`). Les jobs non sélectionnés se terminent tout de suite (**Skip**).

**Version :** celle de chaque **`packages/.../package.json`** — bump manuel par le développeur avant de lancer le workflow.

**Trusted Publishing** (OIDC), comme le SDK.

### Publication Maven — `publish-maven-sdk.yml`

**Artefact :** `app.portaki:portaki-module-sdk`.

**Déclencheurs :** push `develop` sur `sdk/java/`, release (`java-v`, `sdk-java-v`, `v`), ou `workflow_dispatch` avec **`version`**.

Étapes : bump de version optionnel, **`Configure OSSRH credentials`**, puis **`Deploy`** (`mvn deploy`).

---

## Consommer depuis npmjs

```bash
npm install @portaki/module-sdk
```

Paquets **publics** sous scope autorisé : pas de `.npmrc` obligatoire.

---

## Consommer le jar Maven (Maven Central)

Après publication sur Central, utilisez la version publiée. En local dans ce dépôt :

```bash
cd sdk/java && mvn install -DskipTests
```

---

## Références

- [npm — Trusted publishers](https://docs.npmjs.com/trusted-publishers)
- [npm — publishing scoped packages](https://docs.npmjs.com/cli/v10/using-npm/scope)
- [GitHub Actions — npm provenance](https://docs.npmjs.com/generating-provenance-statements)
- [Sonatype / OSSRH](https://central.sonatype.org/)
