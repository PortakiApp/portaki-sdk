# Déploiement — npm

Les paquets **`@portaki/sdk`**, **`@portaki/cli`** et **`@portaki/sdk-test-support`** sont publiés sur **[registry.npmjs.org](https://www.npmjs.com/)** via **[Trusted Publishing](https://docs.npmjs.com/trusted-publishers)** (OIDC GitHub Actions : permission **`id-token: write`**, Node **24** pour npm **≥ 11.5.1** — pas de **`NPM_TOKEN`** en CI).

Les modules invités **`@portaki/module-*`** sont publiés depuis **[portaki-modules](https://github.com/PortakiApp/portaki-modules)** (`publish-npm-packages.yml`).

---

## Workflows

| Fichier | Rôle |
|---------|------|
| [`ci.yml`](../.github/workflows/ci.yml) | Vérification sur `push` / `pull_request` (`main`, `develop`) : build + `pnpm test:coverage`. Job requis : **`verify`**. |
| [`release.yml`](../.github/workflows/release.yml) | Après **CI** verte sur **`main`**, ou **`workflow_dispatch`** : build, publication npm des trois paquets, puis création de la release GitHub **`v{semver}`** (version lue dans `packages/sdk/package.json`). |

### Avant de merger sur `main`

1. Bump **`version`** dans `packages/sdk/package.json`, `packages/cli/package.json` et `packages/sdk-test-support/package.json` (semver cohérent sur le SDK ; CLI peut diverger si besoin).
2. Merger — **CI** doit passer.
3. **Release** se déclenche automatiquement (ou lancer **Actions → Release → Run workflow**).

Si la release **`vX.Y.Z`** existe déjà sur GitHub, le job **publish** est ignoré (idempotent).

### Trusted Publishing (npm)

Configurer sur [npmjs.com](https://www.npmjs.com/) pour chaque paquet publié :

| Paquet | Dépôt GitHub | Workflow |
|--------|--------------|----------|
| `@portaki/sdk` | `portaki-sdk` | `release.yml` |
| `@portaki/cli` | `portaki-sdk` | `release.yml` |
| `@portaki/sdk-test-support` | `portaki-sdk` | `release.yml` |

Publication locale (secours) :

```bash
pnpm -r --filter '@portaki/sdk' --filter '@portaki/cli' --filter '@portaki/sdk-test-support' build
cd packages/sdk && npm publish --access public
```

---

## Consommer depuis npm

```bash
npm install @portaki/sdk
```

Dépendances typiques d’un module auteur :

```json
{
  "dependencies": { "@portaki/sdk": "^2.0.0" },
  "devDependencies": {
    "@portaki/cli": "^0.1.0",
    "@portaki/sdk-test-support": "^2.0.0"
  }
}
```

---

## Dépannage — `gh release` en **403**

1. **Settings → Actions → General → Workflow permissions** → **Read and write**.
2. Rulesets : autoriser **GitHub Actions** sur les releases/tags.
3. Secret optionnel **`GH_RELEASE_PAT`** (classic `repo` ou fine-grained *Contents: Read and write*) — utilisé à la place de `GITHUB_TOKEN` pour `gh release create`.

---

## Références

- [npm — Trusted publishers](https://docs.npmjs.com/trusted-publishers)
- [npm — scoped packages](https://docs.npmjs.com/cli/v10/using-npm/scope)
- [GitHub Actions — npm provenance](https://docs.npmjs.com/generating-provenance-statements)
