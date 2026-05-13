# Déploiement — npmjs & Maven Central

Les workflows publient :

- **npm** : registre public **[registry.npmjs.org](https://www.npmjs.com/)** en CI via **[Trusted Publishing](https://docs.npmjs.com/trusted-publishers)** (OIDC GitHub Actions : permission **`id-token: write`**, **Node 24** pour le **npm 11.5.x** fourni avec Node — pas de **`NPM_TOKEN`** dans les jobs de publication).
- **Maven** : **[Maven Central](https://central.sonatype.com/)** via le **[Central Publisher Portal](https://central.sonatype.com/)** — publication des **releases** stables (pas de **`-SNAPSHOT`**) avec **`central-publishing-maven-plugin`** ([guide Sonatype](https://central.sonatype.org/publish/publish-portal-maven/)). La CI reprend les idées du tutoriel GitHub **[Publishing Java packages with Maven](https://docs.github.com/en/actions/tutorials/publish-packages/publish-java-packages-with-maven)** (`actions/setup-java`, secrets `OSSRH_*`), en s’appuyant sur la doc Sonatype plutôt que sur les exemples **OSSRH legacy** signalés dans ce tutoriel ([différences Portal vs OSSRH](https://central.sonatype.org/faq/what-is-different-between-central-portal-and-legacy-ossrh/#publishing)). Le `pom` respecte les **[exigences Central](https://central.sonatype.org/publish/requirements/)** (nom, description, URL, licence, développeurs, SCM, [sources & javadoc](https://central.sonatype.org/publish/requirements/#supply-javadoc-and-sources), [signatures GPG](https://central.sonatype.org/publish/requirements/#sign-files-with-gpgpgp)).

---

## Publier sur npmjs (résumé)

1. **Compte npm** : créer un compte sur [npmjs.com](https://www.npmjs.com/). Pour un scope **`@portaki/*`**, créer l’[organisation](https://docs.npmjs.com/creating-an-organization) **portaki** (ou utiliser un scope personnel si vous acceptez de changer les noms de paquets).
2. **CI — Trusted Publishing** : pour **`@portaki/module-sdk`**, configurer sur npm : dépôt **`portaki-sdk`**, fichier **`publish-npm-sdk.yml`**. Pour les **`@portaki/module-*`** invités, configurer sur npm : dépôt **[portaki-modules](https://github.com/PortakiApp/portaki-modules)**, fichier **`publish-npm-packages.yml`** (voir la doc de ce dépôt).
3. **Publication locale / secours** : npm → **Access Tokens** (granulaire avec **Publish** sur le scope **`@portaki`**) si vous publiez hors CI ou sans Trusted Publishing ; config locale : `npm config set //registry.npmjs.org/:_authToken=YOUR_TOKEN` (ne pas committer).
4. **Version dans le dépôt** : bump **`version`** dans `sdk/module-sdk/package.json` (SDK) **avant** publication — la CI **ne modifie pas** ce fichier. Les modules invités : bump dans **[portaki-modules](https://github.com/PortakiApp/portaki-modules)** (`modules/<id>/package.json`).
5. **Déclencher une publication** :
   - **SDK** : **Actions** → **publish-npm-sdk** → **Run workflow** (ou release tag `javascript-v…` / `js-v…` / `sdk-js-v…`).
   - **Modules invités** : **Actions** sur **portaki-modules** → **Publish npm** (`publish-npm-packages.yml`) → **Run workflow**.
6. **Localement** (sans CI), depuis la racine du paquet :

   ```bash
   cd sdk/module-sdk
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
| `GPG_PASSPHRASE` | Passphrase de la clé ; en CI elle est exposée sous le nom d’environnement **`MAVEN_GPG_PASSPHRASE`** pour **`actions/setup-java`** et **`maven-gpg-plugin`**. |

La workflow **`publish-maven-sdk`** suit le fil du tutoriel Medium **[Publish your artifact to the Maven Central Repository using GitHub Actions](https://medium.com/@jtbsorensen/publish-your-artifact-to-the-maven-central-repository-using-github-actions-15d3b5d9ce88)** : checkout, **`actions/setup-java`** avec `server-id` **`central`**, identifiants **`MAVEN_USERNAME`** / **`MAVEN_PASSWORD`** (remplis depuis **`OSSRH_USERNAME`** / **`OSSRH_TOKEN`** au lieu des secrets **`CENTRAL_TOKEN_*`** de l’article), import GPG intégré (`gpg-private-key` ← **`GPG_PRIVATE_KEY`**, `gpg-passphrase` ← **`MAVEN_GPG_PASSPHRASE`** ← **`GPG_PASSPHRASE`**), **`versions:set`** sur **`release`** à partir du tag **`java-v…`**, puis **`mvn --batch-mode deploy -DskipTests -P central-deploy`** (profil **`central-deploy`** : sources, javadoc, GPG, plugin Central).

---

## Prérequis Maven Central (checklist)

À valider **côté compte Sonatype / build** avant de chercher un problème côté HTTP (ex. **401** sur l’upload) :

1. **Namespace** : le `groupId` **`app.portaki`** correspond à un [namespace vérifié](https://central.sonatype.com/publishing/namespaces) sur le Central Portal (même compte ou droits de publication).
2. **User token** : jeton généré depuis [Generate User Token](https://central.sonatype.org/publish/generate-portal-token/) / compte [central.sonatype.com](https://central.sonatype.com/account) — **pas** un ancien mot de passe OSSRH. Les secrets GitHub **`OSSRH_USERNAME`** et **`OSSRH_TOKEN`** doivent être **exactement** le couple *username* / *password* affiché à la création du jeton (sans espace parasite, sans retour ligne).
3. **Métadonnées POM** : [exigences Sonatype](https://central.sonatype.org/publish/requirements/) — notamment [nom, description, URL](https://central.sonatype.org/publish/requirements/#project-name-description-and-url), licence, développeurs (email **ou** URL de profil, cf. doc), **SCM** ; version **release** (pas **`-SNAPSHOT`**).
4. **Sources & Javadoc** : jars **`-sources`** et **`-javadoc`** (profil **`central-deploy`** dans ce dépôt).
5. **GPG** : chaque fichier publié a un **`.asc`** ; voir [Working with PGP Signatures](https://central.sonatype.org/publish/requirements/gpg/) — clé publique poussée vers un [serveur de clés supporté](https://central.sonatype.org/publish/requirements/gpg/#distributing-your-public-key) (**`keyserver.ubuntu.com`**, **`keys.openpgp.org`**, **`pgp.mit.edu`**) et **clé enregistrée** sur votre profil Central. **Attention** : si une **sous-clé** est utilisée pour signer (`usage: S` sur un `ssb`), Central peut refuser la vérification — il faut signer avec la **clé primaire** (voir la section *Delete a Sub Key* de la doc GPG Sonatype).
6. **Checksums** : générés par **`central-publishing-maven-plugin`** (cf. [exigences checksums](https://central.sonatype.org/publish/requirements/#provide-file-checksums)).
7. **Secrets CI** : **`GPG_PRIVATE_KEY`** (armored) + **`GPG_PASSPHRASE`** (aucun secret supplémentaire ; la CI mappe la passphrase vers **`MAVEN_GPG_PASSPHRASE`**).

---

## Workflows (fichiers en slug)

| Fichier | Rôle |
|---------|------|
| [`ci-verify.yml`](../.github/workflows/ci-verify.yml) | Vérification : SDK JS, SDK Java. |
| [`sdk-release-main.yml`](../.github/workflows/sdk-release-main.yml) | Enchaîne sur **`ci-verify`** terminé avec succès sur **`main`** (ou `workflow_dispatch`) : vérifie Java/JS selon les fichiers du commit, **releases GitHub** `java-v*` / `javascript-v*`, **Maven Central** + **npmjs** (évite la course avec la CI du même push). |
| [`publish-npm-sdk.yml`](../.github/workflows/publish-npm-sdk.yml) | Publie **`@portaki/module-sdk`** (`sdk/module-sdk`) — manuel / release UI. |
| [`publish-maven-sdk.yml`](../.github/workflows/publish-maven-sdk.yml) | Publie **`app.portaki:portaki-module-sdk`** — **`workflow_dispatch`** ou **`release`** publiée depuis l’UI GitHub (`java-v…`). |

### Maven Central — attente de fin de déploiement

Le profil **`central-deploy`** active **`central-publishing-maven-plugin`** avec **`waitUntil: published`**. Le job **`mvn deploy`** attend donc que Sonatype ait **terminé** la publication côté Central avant de se terminer avec succès : ce n’est **pas** du fire-and-forget à l’échelle du workflow (contrairement à un simple upload HTTP sans attente).

Les **`@portaki/module-*`** invités sont publiés depuis le dépôt **[portaki-modules](https://github.com/PortakiApp/portaki-modules)** (fichier **`publish-npm-packages.yml`**).

### CI — `ci-verify.yml`

Déclenché sur `push` / `pull_request` vers `main` et `develop` lorsque `sdk/**`, `pnpm-workspace.yaml`, `package.json` ou ce workflow changent.

Jobs (IDs stables) : **`detect_changes`**, **`sdk_module_sdk`**, **`sdk_guest`**, **`sdk_java`**, pilotés par [`dorny/paths-filter`](https://github.com/dorny/paths-filter).

### Publication SDK JS — `publish-npm-sdk.yml`

**Paquet :** `@portaki/module-sdk`.

**Version :** celle de **`sdk/module-sdk/package.json`** (à faire évoluer dans une PR avant publication).

**Déclencheurs :** **`workflow_dispatch`** ; ou **`release`** **`published`** dont le tag commence par **`javascript-v`**, **`js-v`** ou **`sdk-js-v`** (évite de lancer une publication npm sur une release Maven `java-v…`).

### Publication Maven — `publish-maven-sdk.yml`

**Coordonnées :** `app.portaki:portaki-module-sdk` — version **release** dans **`sdk/java/pom.xml`** (sans **`-SNAPSHOT`**).

**Métadonnées POM :** alignées sur les [exigences Sonatype](https://central.sonatype.org/publish/requirements/), y compris [nom, description et URL du projet](https://central.sonatype.org/publish/requirements/#project-name-description-and-url), licence **Apache 2.0**, **`developers`** (avec **URL** profil GitHub si pas d’email), **`scm`**.

**Build :** profil Maven **`central-deploy`** activé explicitement avec **`-P central-deploy`** sur **`deploy`** (CI et publication locale) — attache **sources** et **javadoc**, **GPG sign**, puis **`central-publishing-maven-plugin`** (`autoPublish`, `waitUntil` **`published`**). Le **`mvn verify`** local (CI **`ci-verify`**) **sans** ce profil : pas de GPG requis sur les postes de dev.

**Déclencheurs :**

1. **`workflow_dispatch`** sur **`main`** uniquement — champ **`version`** optionnel (ex. `0.3.1`) pour **`versions:set`** avant déploiement ; laisser vide pour publier la version déjà dans le `pom`.
2. **`release`** **`published`** dont le tag commence par **`java-v`** (ex. **`java-v0.3.0`**) — checkout au tag, puis **`versions:set`** sur la version sans préfixe (ex. **`0.3.0`**), puis **`mvn --batch-mode deploy -DskipTests -P central-deploy`**.

Après un **`workflow_dispatch`** réussi sur **`main`**, création d’une **release GitHub** `java-v{version}` si elle n’existe pas encore.

**Références :**

- [GitHub Actions — Publier des paquets Java avec Maven](https://docs.github.com/en/actions/tutorials/publish-packages/publish-java-packages-with-maven)
- [Medium — Publish to Maven Central with GitHub Actions](https://medium.com/@jtbsorensen/publish-your-artifact-to-the-maven-central-repository-using-github-actions-15d3b5d9ce88) (noms de secrets Portaki : **`OSSRH_*`**, **`GPG_PRIVATE_KEY`**, **`GPG_PASSPHRASE`**)
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

Pour publier une nouvelle série : bump des **`version`** côté **portaki-sdk** (SDK) et côté **portaki-modules** (invités), puis lancer **`publish-npm-sdk`** sur ce dépôt et **Publish npm** sur **portaki-modules** (`publish-npm-packages.yml`).

---

## Consommer le jar Maven (Maven Central)

Après synchronisation sur Central, déclarez la dépendance sans dépôt supplémentaire (artefacts **release** sur **`repo1.maven.org`**) :

```xml
<dependency>
  <groupId>app.portaki</groupId>
  <artifactId>portaki-module-sdk</artifactId>
  <version>0.3.2</version>
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
- [Sonatype — API Publisher (authentification)](https://central.sonatype.org/publish/publish-portal-api/#authentication--authorization)
- [Sonatype — Publier sur Maven Central (Central Portal + plugin)](https://central.sonatype.org/publish/publish-portal-maven/)
- [Sonatype — Exigences Central (POM, javadoc, sources, GPG)](https://central.sonatype.org/publish/requirements/)
- [Sonatype — Nom, description et URL du projet](https://central.sonatype.org/publish/requirements/#project-name-description-and-url)
