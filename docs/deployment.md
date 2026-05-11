# Déploiement — npmjs & Maven Central

Les workflows publient :

- **npm** : registre public **[registry.npmjs.org](https://www.npmjs.com/)** via le secret GitHub **`NPM_TOKEN`**.
- **Maven** : **Sonatype OSSRH** → **Maven Central** avec **`OSSRH_USERNAME`** et **`OSSRH_TOKEN`**.

---

## Publier sur npmjs (résumé)

1. **Compte npm** : créer un compte sur [npmjs.com](https://www.npmjs.com/). Pour un scope **`@portakiapp/*`**, créer l’[organisation](https://docs.npmjs.com/creating-an-organization) **portakiapp** (ou utiliser un scope personnel si vous acceptez de changer les noms de paquets).
2. **Token de publication** : npm → **Access Tokens** → **Generate New Token** → type **Granular** ou **Classic** avec au minimum :
   - **Publish** pour les packages concernés (scope `@portakiapp` si granular).
3. **Secret GitHub** : dans le dépôt → **Settings** → **Secrets and variables** → **Actions** → **New repository secret** :
   - Nom : **`NPM_TOKEN`**
   - Valeur : le token npm (commence souvent par `npm_…`).
4. **Déclencher une publication** :
   - **Automatique** : push sur **`develop`** qui modifie `sdk/javascript/` déclenche [`publish-npm-sdk.yml`](../.github/workflows/publish-npm-sdk.yml) (version `major.minor.<run_number>`).
   - **Release GitHub** : créer une release avec un tag du type `javascript-v0.2.0`, `js-v0.2.0`, ou `v0.2.0`.
   - **Manuel** : **Actions** → **publish-npm-sdk** ou **publish-npm-packages** → **Run workflow**.
5. **Localement** (sans CI), depuis la racine du paquet :

   ```bash
   cd sdk/javascript   # ou un dossier sous packages/<nom>/
   npm login           # une fois
   npm publish --access public
   ```

   Utiliser un token en CI/local : `npm config set //registry.npmjs.org/:_authToken=YOUR_TOKEN` (éviter de committer le token).

---

## Secrets GitHub

| Secret | Utilisation |
|--------|-------------|
| `NPM_TOKEN` | Publication npm (`NODE_AUTH_TOKEN` dans les jobs `Setup Node.js` avec `registry-url: https://registry.npmjs.org`). |
| `OSSRH_USERNAME` | Identifiant Sonatype (OSSRH / Central Portal). |
| `OSSRH_TOKEN` | Mot de passe ou token Sonatype. |

Les **releases Maven** non-SNAPSHOT peuvent exiger signature GPG selon la politique Sonatype — à ajouter au `pom.xml` si besoin.

---

## Workflows (fichiers en slug)

| Fichier | Rôle |
|---------|------|
| [`ci-verify.yml`](../.github/workflows/ci-verify.yml) | Vérification : SDK JS, SDK Java, lint `packages/`, backend pre-arrival Maven si chemins concernés. |
| [`publish-npm-sdk.yml`](../.github/workflows/publish-npm-sdk.yml) | Publie **`@portakiapp/module-sdk`** (`sdk/javascript`). |
| [`publish-npm-packages.yml`](../.github/workflows/publish-npm-packages.yml) | Publie manuellement les **`@portakiapp/module-*`** sous `packages/`. |
| [`publish-maven-sdk.yml`](../.github/workflows/publish-maven-sdk.yml) | Déploie **`app.portaki:portaki-module-sdk`** (`sdk/java`) vers OSSRH. |

### CI — `ci-verify.yml`

Déclenché sur `push` / `pull_request` vers `main` et `develop` lorsque `sdk/**`, `packages/**`, fichiers workspace racine ou ce workflow changent.

Jobs (IDs stables) : **`detect_changes`**, **`sdk_javascript`**, **`sdk_java`**, **`workspace_packages`**, **`pre_arrival_java`**, pilotés par [`dorny/paths-filter`](https://github.com/dorny/paths-filter).

### Publication SDK JS — `publish-npm-sdk.yml`

**Paquet :** `@portakiapp/module-sdk`.

**Déclencheurs :**

1. Push sur **`develop`** (changements sous `sdk/javascript/` ou ce workflow). Version : **`major.minor.<run_number>`**.
2. Release GitHub **`published`** — tags : `javascript-v`, `js-v`, `sdk-js-v`, ou `v`.
3. **`workflow_dispatch`** — champ **`version`** optionnel.

### Publication paquets invités — `publish-npm-packages.yml`

Uniquement **`workflow_dispatch`** : choix **`npm_package`** (`all` ou un `@portakiapp/module-…`). Utilise `pnpm publish --filter` et **`NPM_TOKEN`**.

### Publication Maven — `publish-maven-sdk.yml`

**Artefact :** `app.portaki:portaki-module-sdk`.

**Déclencheurs :** push `develop` sur `sdk/java/`, release (`java-v`, `sdk-java-v`, `v`), ou `workflow_dispatch` avec **`version`**.

Étapes : bump de version optionnel, **`Configure OSSRH credentials`**, puis **`Deploy`** (`mvn deploy`).

---

## Consommer depuis npmjs

```bash
npm install @portakiapp/module-sdk
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

- [npm — publishing scoped packages](https://docs.npmjs.com/cli/v10/using-npm/scope)
- [GitHub Actions — npm provenance](https://docs.npmjs.com/generating-provenance-statements)
- [Sonatype / OSSRH](https://central.sonatype.org/)
