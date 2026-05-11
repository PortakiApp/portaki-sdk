# Déploiement des SDK sur GitHub

Les workflows publient les artefacts vers **GitHub Packages** (registre npm et registre Maven du même dépôt). Ils utilisent le **`GITHUB_TOKEN`** intégré aux Actions : aucun secret personnel n’est obligatoire pour la publication standard.

---

## Prérequis

1. Dépôt sur GitHub avec Actions activées.
2. Permissions du workflow : `packages: write` (déjà défini dans les fichiers YAML).
3. Pour **npm** : le nom publié doit être du forme `@<propriétaire-github>/module-sdk` avec `<propriétaire-github>` = owner du repo en **minuscules** (règle GitHub Packages). Le workflow **Publish JavaScript** applique cette règle automatiquement via `npm pkg set`.

---

## Workflows

### CI — `.github/workflows/ci.yml`

Déclenché sur `push` / `pull_request` vers `main` et `develop` :

- **javascript** : `npm ci`, `npm run build`
- **java** : `mvn verify`

### Publication npm — `.github/workflows/publish-npm.yml`

**Déclencheurs**

1. **Push sur `develop`** (fichiers sous `javascript/` ou ce workflow)  
   - Version publiée : **`major.minor.<run_number>`** où `major.minor` vient des deux premiers segments du `package.json` (ex. `0.1.0` → `0.1`) et `<run_number>` est l’identifiant monotonique du workflow sur GitHub (ex. **`0.1.47`**). Pas de prérelease du type `-develop.*`. Chaque push produit une **nouvelle** version npm (npm interdit de republier une version identique).
2. **Release GitHub** : type `published`  
   - Le tag de la release est interprété pour la version npm : préfixes supportés `javascript-v`, `js-v`, ou `v` seul (ex. `javascript-v0.2.0` → `0.2.0`).
3. **workflow_dispatch** : champ optionnel `version` (semver). Si vide, la version du `package.json` est conservée.

**Effet** : `npm publish` vers `https://npm.pkg.github.com`, après alignement du nom du paquet sur `@owner/module-sdk`.

### Publication Maven — `.github/workflows/publish-maven.yml`

**Déclencheurs**

1. **Push sur `develop`** (fichiers sous `java/` ou ce workflow)  
   - Déploie la version courante du `pom.xml` (typiquement **`0.1.0-SNAPSHOT`**). Les SNAPSHOT Maven peuvent être **republiés** à chaque push.
2. **Release GitHub** : `published`  
   - Tag supportés : `java-v0.2.0` ou `v0.2.0` → version Maven `0.2.0`.
3. **workflow_dispatch** : champ optionnel `version` (semver ou `…-SNAPSHOT`). Si vide, la version du `pom.xml` est utilisée telle quelle.

**Effet** : `mvn deploy` avec dépôt alternatif  
`github::default::https://maven.pkg.github.com/<owner>/<repo>`  
et fichier `~/.m2/settings.xml` généré dans le job (authentification par `GITHUB_TOKEN`).

---

## Publier via une release GitHub (recommandé)

1. Fusionnez votre code sur la branche par défaut (souvent `main`).
2. Créez un **tag** git (ex. `javascript-v0.2.0` pour le JS, `java-v0.2.0` pour le Java) ou un tag unique si vous versionnez les deux en même temps — adaptez les workflows si vous utilisez une convention unique.
3. Dans GitHub : **Releases** → **Draft a new release** → choisissez le tag → **Publish release**.
4. Les workflows **Publish JavaScript** et **Publish Maven** se lancent ; vérifiez l’onglet **Actions** puis **Packages** à droite du dépôt.

Pour ne publier qu’un seul runtime, utilisez **workflow_dispatch** sur le workflow concerné au lieu d’une release complète.

---

## Consommer le paquet npm (GitHub Packages)

Dans le projet consommateur, ajoutez un `.npmrc` à la racine (ou au niveau utilisateur) :

```ini
@VOTRE_OWNER:registry=https://npm.pkg.github.com
//npm.pkg.github.com/:_authToken=${NODE_AUTH_TOKEN}
```

- En **CI**, définissez `NODE_AUTH_TOKEN` avec un PAT ayant au moins `read:packages`, ou avec `GITHUB_TOKEN` si le job est dans le même dépôt et les permissions le permettent.
- Installez ensuite : `npm install @votre_owner/module-sdk`

---

## Consommer le jar Maven (GitHub Packages)

Dans le `pom.xml` du consommateur, déclarez le dépôt :

```xml
<repositories>
  <repository>
    <id>github</id>
    <url>https://maven.pkg.github.com/VOTRE_OWNER/portaki-sdk</url>
    <snapshots>
      <enabled>true</enabled>
    </snapshots>
  </repository>
</repositories>
```

Configurez l’authentification dans `~/.m2/settings.xml` (serveur `id` = `github`, utilisateur GitHub, mot de passe = PAT avec `read:packages`) ou via les variables supportées par votre CI.

Référence de dépendance :

```xml
<dependency>
  <groupId>app.portaki</groupId>
  <artifactId>portaki-module-sdk</artifactId>
  <version>0.1.0-SNAPSHOT</version>
</dependency>
```

Adaptez `version` à la version réellement publiée.

---

## Publication sur le registre npm public (npmjs.com)

Ce dépôt est prêt pour **GitHub Packages**. Pour npmjs.com, il faudrait :

- retirer ou surcharger `publishConfig.registry` ;
- utiliser un scope autorisé sur npmjs ;
- stocker `NPM_TOKEN` dans les secrets du dépôt et l’utiliser à la place de `GITHUB_TOKEN` dans un workflow dédié.

Vous pouvez dupliquer `publish-npm.yml` et ajuster ces points si vous basculez vers npmjs.

---

## Dépannage

| Symptôme | Piste |
|----------|--------|
| `403` npm | Vérifier le scope `@owner`, le token, et que `packages: write` est bien présent sur le job. |
| `401` Maven | Vérifier `settings.xml`, le `server` `id` aligné avec le dépôt, et le PAT. |
| Version incorrecte depuis une release | Vérifier le format du tag (`javascript-v`, `java-v`, `v`). |
| SNAPSHOT Java | Les SNAPSHOT sont acceptés sur GitHub Packages ; les consommateurs doivent autoriser les snapshots dans leur `pom`. |

Pour toute évolution du contrat d’API, gardez ce fichier aligné avec les workflows réels sous `.github/workflows/`.
