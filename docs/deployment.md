# Déploiement — npmjs & Maven Central

Les workflows publient :

- **npm** : registre public **[registry.npmjs.org](https://www.npmjs.com/)** en CI via **[Trusted Publishing](https://docs.npmjs.com/trusted-publishers)** (OIDC GitHub Actions : permission **`id-token: write`**, **Node 24** pour le **npm 11.5.x** fourni avec Node — pas de **`NPM_TOKEN`** dans les jobs de publication).
- **Maven** : **[Maven Central](https://central.sonatype.com/)** via le **[Central Publisher Portal](https://central.sonatype.com/)** — publication des **releases** stables (pas de **`-SNAPSHOT`**) avec **`central-publishing-maven-plugin`** ([guide Sonatype](https://central.sonatype.org/publish/publish-portal-maven/)). La CI reprend les idées du tutoriel GitHub **[Publishing Java packages with Maven](https://docs.github.com/en/actions/tutorials/publish-packages/publish-java-packages-with-maven)** (`actions/setup-java`, secrets `OSSRH_*`), en s’appuyant sur la doc Sonatype plutôt que sur les exemples **OSSRH legacy** signalés dans ce tutoriel ([différences Portal vs OSSRH](https://central.sonatype.org/faq/what-is-different-between-central-portal-and-legacy-ossrh/#publishing)). Le `pom` respecte les **[exigences Central](https://central.sonatype.org/publish/requirements/)** (nom, description, URL, licence, développeurs, SCM, [sources & javadoc](https://central.sonatype.org/publish/requirements/#supply-javadoc-and-sources), [signatures GPG](https://central.sonatype.org/publish/requirements/#sign-files-with-gpgpgp)).

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

## Secrets GitHub (Maven Central)

| Secret | Utilisation |
|--------|-------------|
| *(aucun pour npm en CI)* | La publication npm utilise **Trusted Publishing** (OIDC), pas **`NPM_TOKEN`**. |
| `OSSRH_USERNAME` | **User token** Central Portal — champ *username* ([générer un jeton](https://central.sonatype.org/publish/generate-portal-token/)). |
| `OSSRH_TOKEN` | **User token** Central Portal — champ *password* du même jeton. |
| `GPG_PRIVATE_KEY` | Clé privée **ASCII armored** pour signer les artefacts ([exigences GPG](https://central.sonatype.org/publish/requirements/gpg/)). |
| `GPG_PASSPHRASE` | Passphrase de la clé ; réutilisée comme **`MAVEN_GPG_PASSPHRASE`** pour **`maven-gpg-plugin`** en CI. |

La workflow **`publish-maven-sdk`** suit le modèle GitHub **[Publishing Java packages with Maven](https://docs.github.com/en/actions/tutorials/publish-packages/publish-java-packages-with-maven)** : checkout, **`actions/setup-java`** avec **`server-id`** / **`server-username`** / **`server-password`** (noms de variables d’environnement), secrets **`OSSRH_USERNAME`** / **`OSSRH_TOKEN`**, puis **`mvn deploy`**. Contrairement aux exemples **OSSRH** du tutoriel, ce dépôt utilise le **Central Portal** (`server` **`central`**, **`central-publishing-maven-plugin`**, GPG, profil **`-Dcentral.deploy=true`**). Une étape réécrit **`~/.m2/settings.xml`** avec **`usePreemptiveAuth`** pour éviter les **401** sur l’upload du bundle. L’import GPG utilise **`crazy-max/ghaction-import-gpg`**.

---

## Workflows (fichiers en slug)

| Fichier | Rôle |
|---------|------|
| [`ci-verify.yml`](../.github/workflows/ci-verify.yml) | Vérification : SDK JS, SDK Java. |
| [`publish-npm-sdk.yml`](../.github/workflows/publish-npm-sdk.yml) | Publie **`@portaki/module-sdk`** (`sdk/javascript`). |
| [`publish-maven-sdk.yml`](../.github/workflows/publish-maven-sdk.yml) | Publie **`app.portaki:portaki-module-sdk`** en **release** sur Central (`mvn deploy -Dcentral.deploy=true`). |

Les **`@portaki/module-*`** invités sont publiés depuis le dépôt **[portaki-modules](https://github.com/PortakiApp/portaki-modules)** (workflow `publish-npm.yml`).

### CI — `ci-verify.yml`

Déclenché sur `push` / `pull_request` vers `main` et `develop` lorsque `sdk/**`, `pnpm-workspace.yaml`, `package.json` ou ce workflow changent.

Jobs (IDs stables) : **`detect_changes`**, **`sdk_javascript`**, **`sdk_java`**, pilotés par [`dorny/paths-filter`](https://github.com/dorny/paths-filter).

### Publication SDK JS — `publish-npm-sdk.yml`

**Paquet :** `@portaki/module-sdk`.

**Version :** celle de **`sdk/javascript/package.json`** (à faire évoluer dans une PR avant publication).

**Déclencheurs :** **`workflow_dispatch`** ; ou **`release`** **`published`** dont le tag commence par **`javascript-v`**, **`js-v`** ou **`sdk-js-v`** (évite de lancer une publication npm sur une release Maven `java-v…`).

### Publication Maven — `publish-maven-sdk.yml`

**Coordonnées :** `app.portaki:portaki-module-sdk` — version **release** dans **`sdk/java/pom.xml`** (ex. **`0.3.0`**, sans **`-SNAPSHOT`**).

**Métadonnées POM :** alignées sur les [exigences Sonatype](https://central.sonatype.org/publish/requirements/), y compris [nom, description et URL du projet](https://central.sonatype.org/publish/requirements/#project-name-description-and-url), licence **Apache 2.0**, **`developers`**, **`scm`**.

**Build :** profil Maven **`central-deploy`** activé par **`-Dcentral.deploy=true`** — attache **sources** et **javadoc**, **GPG sign**, puis **`central-publishing-maven-plugin`** (`autoPublish`, `waitUntil` **`published`**). Le **`mvn verify`** local (CI **`ci-verify`**) **n’active pas** ce profil : pas de GPG requis sur les postes de dev.

**Déclencheurs :**

1. **`workflow_dispatch`** sur **`main`** uniquement — champ **`version`** optionnel (ex. `0.3.1`) pour **`versions:set`** avant déploiement ; laisser vide pour publier la version déjà dans le `pom`.
2. **`release`** **`published`** dont le tag commence par **`java-v`** (ex. **`java-v0.3.0`**) — checkout au tag, puis **`mvn deploy`** (utile si la release GitHub est créée à la main après merge du bump de version).

Après un **`workflow_dispatch`** réussi sur **`main`**, création d’une **release GitHub** `java-v{version}` si elle n’existe pas encore.

**Références :**

- [GitHub Actions — Publier des paquets Java avec Maven](https://docs.github.com/en/actions/tutorials/publish-packages/publish-java-packages-with-maven)
- [Publier avec Maven (Central Portal)](https://central.sonatype.org/publish/publish-portal-maven/)
- [Exigences (métadonnées, javadoc, sources, GPG)](https://central.sonatype.org/publish/requirements/)

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

Après synchronisation sur Central, déclarez la dépendance sans dépôt supplémentaire (artefacts **release** sur **`repo1.maven.org`**) :

```xml
<dependency>
  <groupId>app.portaki</groupId>
  <artifactId>portaki-module-sdk</artifactId>
  <version>0.3.0</version>
</dependency>
```

En local dans ce dépôt :

```bash
cd sdk/java && mvn install -DskipTests
```

---

## Références

- [npm — Trusted publishers](https://docs.npmjs.com/trusted-publishers)
- [npm — publishing scoped packages](https://docs.npmjs.com/cli/v10/using-npm/scope)
- [GitHub Actions — npm provenance](https://docs.npmjs.com/generating-provenance-statements)
- [GitHub Actions — Publier des paquets Java avec Maven](https://docs.github.com/en/actions/tutorials/publish-packages/publish-java-packages-with-maven)
- [Sonatype — Publier sur Maven Central (Central Portal + plugin)](https://central.sonatype.org/publish/publish-portal-maven/)
- [Sonatype — Exigences Central (POM, javadoc, sources, GPG)](https://central.sonatype.org/publish/requirements/)
- [Sonatype — Nom, description et URL du projet](https://central.sonatype.org/publish/requirements/#project-name-description-and-url)
