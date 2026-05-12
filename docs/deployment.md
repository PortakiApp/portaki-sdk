# Déploiement — npmjs & Maven Central

Les workflows publient :

- **npm** : registre public **[registry.npmjs.org](https://www.npmjs.com/)** en CI via **[Trusted Publishing](https://docs.npmjs.com/trusted-publishers)** (OIDC GitHub Actions : permission **`id-token: write`**, **Node 24** pour le **npm 11.5.x** fourni avec Node — pas de **`NPM_TOKEN`** dans les jobs de publication).
- **Maven** : **[Central Publisher Portal](https://central.sonatype.com/)** — snapshots via **`maven-deploy`** + jeton Portal ([détails](#secrets-github)). Activer **Enable SNAPSHOTs** sur le namespace **`app.portaki`** ([namespaces](https://central.sonatype.com/publishing/namespaces)).

---

## Publier sur npmjs (résumé)

1. **Compte npm** : créer un compte sur [npmjs.com](https://www.npmjs.com/). Pour un scope **`@portaki/*`**, créer l’[organisation](https://docs.npmjs.com/creating-an-organization) **portaki** (ou utiliser un scope personnel si vous acceptez de changer les noms de paquets).
2. **CI — Trusted Publishing** : pour **`@portaki/module-sdk`**, configurer sur npm : dépôt **`portaki-sdk`**, workflow **`publish-npm-sdk.yml`**. Pour les **`@portaki/module-*`** invités, configurer sur npm : dépôt **[portaki-modules](https://github.com/PortakiApp/portaki-modules)**, workflow **`publish-npm.yml`** (voir la doc de ce dépôt).
3. **Publication locale / secours** : npm → **Access Tokens** (granulaire avec **Publish** sur le scope **`@portaki`**) si vous publiez hors CI ou sans Trusted Publishing ; config locale : `npm config set //registry.npmjs.org/:_authToken=YOUR_TOKEN` (ne pas committer).
4. **Version dans le dépôt** : bump **`version`** dans `sdk/javascript/package.json` (SDK) **avant** publication — la CI **ne modifie pas** ce fichier. Les modules invités : bump dans **[portaki-modules](https://github.com/PortakiApp/portaki-modules)** (`modules/<id>/package.json`).
5. **Déclencher une publication** :
   - **SDK** : **Actions** → **publish-npm-sdk** → **Run workflow** (ou release tag `javascript-v…` / `js-v…` / `sdk-js-v…`).
   - **Modules invités** : **Actions** sur **portaki-modules** → **publish-npm-packages** (`publish-npm.yml`) → **Run workflow**.
6. **Localement** (sans CI), depuis la racine du paquet :

   ```bash
   cd sdk/javascript
   npm login           # une fois
   npm publish --access public
   ```

---

## Secrets GitHub

| Secret | Utilisation |
|--------|-------------|
| *(aucun pour npm en CI)* | La publication npm utilise **Trusted Publishing** (OIDC), pas **`NPM_TOKEN`**. |
| `OSSRH_USERNAME` | **User token** Central — champ *username* ([usertoken](https://central.sonatype.com/usertoken)). |
| `OSSRH_TOKEN` | **User token** Central — champ *password* du même jeton. |

La workflow **`publish-maven-sdk`** définit **`MAVEN_USERNAME`** et **`MAVEN_CENTRAL_TOKEN`** à partir des secrets **`OSSRH_USERNAME`** / **`OSSRH_TOKEN`**, puis **`actions/setup-java`** avec **`server-id: ossrh`**, **`server-username: MAVEN_USERNAME`** et **`server-password: MAVEN_CENTRAL_TOKEN`** : c’est la [méthode documentée](https://github.com/actions/setup-java/blob/main/docs/advanced-usage.md) — le action écrit **`~/.m2/settings.xml`** avec le couple HTTP Basic attendu par **`mvn deploy`** (GET `maven-metadata.xml` + PUT). Les **`-SNAPSHOT`** vont vers **`https://central.sonatype.com/repository/maven-snapshots/`** (sans GPG).

---

## Workflows (fichiers en slug)

| Fichier | Rôle |
|---------|------|
| [`ci-verify.yml`](../.github/workflows/ci-verify.yml) | Vérification : SDK JS, SDK Java. |
| [`publish-npm-sdk.yml`](../.github/workflows/publish-npm-sdk.yml) | Publie **`@portaki/module-sdk`** (`sdk/javascript`). |
| [`publish-maven-sdk.yml`](../.github/workflows/publish-maven-sdk.yml) | Déploie **`app.portaki:portaki-module-sdk`** (`sdk/java`, **SNAPSHOT**) via **`setup-java`** (`server-id` **ossrh** + secrets). |

Les **`@portaki/module-*`** invités sont publiés depuis le dépôt **[portaki-modules](https://github.com/PortakiApp/portaki-modules)** (workflow `publish-npm.yml`).

### CI — `ci-verify.yml`

Déclenché sur `push` / `pull_request` vers `main` et `develop` lorsque `sdk/**`, `pnpm-workspace.yaml`, `package.json` ou ce workflow changent.

Jobs (IDs stables) : **`detect_changes`**, **`sdk_javascript`**, **`sdk_java`**, pilotés par [`dorny/paths-filter`](https://github.com/dorny/paths-filter).

### Publication SDK JS — `publish-npm-sdk.yml`

**Paquet :** `@portaki/module-sdk`.

**Version :** celle de **`sdk/javascript/package.json`** (à faire évoluer dans une PR avant publication).

**Déclencheurs :** **`workflow_dispatch`** ; ou **`release`** **`published`** dont le tag commence par **`javascript-v`**, **`js-v`** ou **`sdk-js-v`** (évite de lancer une publication npm sur une release Maven `java-v…`).

### Publication Maven — `publish-maven-sdk.yml`

**Artefact :** `app.portaki:portaki-module-sdk` (version **`0.3.0-SNAPSHOT`** dans le `pom` tant qu’on publie des snapshots).

**Déclencheurs :** push **`main`** sur `sdk/java/` (ou ce workflow) ; ou **`workflow_dispatch`** avec **`version`** optionnelle.

Étapes : contrôle des secrets, **`actions/setup-java`** (JDK + cache Maven + **`settings.xml`** serveur **`ossrh`**), **`mvn verify`**, **`mvn deploy`**, puis sur **`main`** sans **`-SNAPSHOT`** : **release GitHub** `java-v{version}` si absente.

**Snapshots :** activer **Enable SNAPSHOTs** sur le namespace dans [Publishing → Namespaces](https://central.sonatype.com/publishing/namespaces).

**Releases** (sans `-SNAPSHOT`) : exigent GPG + sources + javadoc côté Central — non couvert par ce flux minimal ; utiliser plus tard **`central-publishing-maven-plugin`** ou une doc dédiée.

---

## Consommer depuis npmjs

```bash
npm install @portaki/module-sdk
```

Paquets **publics** sous scope autorisé : pas de `.npmrc` obligatoire.

### `portaki-web` et la série **0.3.2**

Les paquets **`@portaki/module-*`** et **`@portaki/module-sdk`** sont publiés en **0.3.2** sur npmjs (patch après des **0.3.0** avec dépendance `file:` incorrecte). **portaki-web** peut déclarer **`^0.3.2`** (ou **`^0.3.0`** pour rétrocompat semver) et exécuter **`pnpm install`**.

Pour publier une nouvelle série : bump des **`version`** côté **portaki-sdk** (SDK) et côté **portaki-modules** (invités), puis lancer **`publish-npm-sdk`** sur ce dépôt et **`publish-npm-packages`** sur **portaki-modules** (workflow `publish-npm.yml`).

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
