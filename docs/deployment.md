# Déploiement — npmjs & Maven Central

Les workflows publient :

- **npm** : registre public **[registry.npmjs.org](https://www.npmjs.com/)** (`NPM_TOKEN`).
- **Maven** : **Sonatype OSSRH** → **Maven Central** (`OSSRH_USERNAME`, `OSSRH_TOKEN`). Les coordonnées `distributionManagement` du `pom.xml` pointent vers les URLs OSSRH (`ossrh`).

---

## Secrets GitHub

| Secret | Utilisation |
|--------|-------------|
| `NPM_TOKEN` | Token npm avec permission **publish** pour le scope **`@portakiapp`** (création sur [npmjs.com](https://www.npmjs.com/) → Access Tokens). |
| `OSSRH_USERNAME` | Identifiant Sonatype (Central Portal / OSSRH). |
| `OSSRH_TOKEN` | Mot de passe ou token Sonatype associé au compte OSSRH. |

Pour les **releases Maven** signées (hors SNAPSHOT), une configuration GPG et les plugins `maven-gpg-plugin` / Central Publisher peuvent être nécessaires — à ajouter au `pom.xml` selon la politique Sonatype du projet.

---

## Workflows

### CI — `.github/workflows/ci.yml`

Déclenché sur `push` / `pull_request` vers `main` et `develop`, uniquement si les chemins sous `sdk/**`, `pnpm-workspace.yaml`, `package.json` ou le workflow lui-même changent.

Les jobs **JavaScript**, **Java**, **packages (pnpm)** et **pre-arrival backend Maven** ne s’exécutent que lorsque les fichiers correspondants sont modifiés (filtre [`dorny/paths-filter`](https://github.com/dorny/paths-filter)).

### Publication npm — `.github/workflows/publish-npm.yml`

**Paquet :** `@portakiapp/module-sdk` (répertoire `sdk/javascript`).

**Déclencheurs**

1. **Push sur `develop`** (changements sous `sdk/javascript/` ou ce workflow). Version publiée : **`major.minor.<run_number>`** (sans prérelease).
2. **Release GitHub** `published` — tags acceptés : `javascript-v`, `js-v`, `sdk-js-v`, ou `v`.
3. **`workflow_dispatch`** — champ optionnel `version`.

**Publication :** `npm publish --access public` vers **npmjs** avec `NODE_AUTH_TOKEN: ${{ secrets.NPM_TOKEN }}`.

### Publication Maven — `.github/workflows/publish-maven.yml`

**Artefact :** `app.portaki:portaki-module-sdk` (`sdk/java`).

**Déclencheurs**

1. Push sur `develop` avec changements sous `sdk/java/` ou ce workflow.
2. Release GitHub — tags : `java-v`, `sdk-java-v`, ou `v`.
3. `workflow_dispatch` avec version optionnelle.

Le job génère `~/.m2/settings.xml` avec le serveur **`ossrh`** puis exécute **`mvn deploy`**.

### Publication des paquets invités — `.github/workflows/publish-modules-npm.yml`

Uniquement **`workflow_dispatch`** : choix du package `@portakiapp/module-*` ou `all`. Utilise `pnpm publish --filter` et **`NPM_TOKEN`**.

---

## Consommer le paquet npm (npmjs)

```bash
npm install @portakiapp/module-sdk
```

Aucun `.npmrc` spécifique n’est requis pour un package public sous scope autorisé sur npmjs.

---

## Consommer le jar Maven (Maven Central)

Après publication effective sur Central, déclarez la dépendance avec la version publiée (sans dépôt `repository` privé si l’artefact est sur Central).

En attendant, ou pour un build **local** dans ce dépôt :

```bash
cd sdk/java && mvn install -DskipTests
```

---

## Références utiles

- [Central Publisher / OSSRH](https://central.sonatype.org/)
- [npm CLI publish](https://docs.npmjs.com/cli/v10/commands/npm-publish)
