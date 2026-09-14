# Contrats de la plateforme — copies, pas sources

Les fichiers de ce dossier **appartiennent à `PortakiApp/portaki-platform`**. Le SDK les recopie
tels quels ; il ne les écrit pas, ne les corrige pas, ne les devance pas.

| Fichier | Source |
|---------|--------|
| `module-limits.json` | [`portaki-platform/contracts/module-limits.json`](https://github.com/PortakiApp/portaki-platform/blob/main/contracts/module-limits.json) |

À l'inverse, `contracts/host-ops.json`, `contracts/sdui_types.json` et
`contracts/deprecations.json` (un niveau au-dessus) sont écrits par le SDK et recopiés par la
plateforme.

## Pourquoi

La plateforme fait foi sur les limites des modules : c'est elle qui refuse un appel. Le SDK en
recopie les valeurs dans `crates/portaki-sdk/src/limits.rs` pour échouer tôt — dans ses
validations et dans le mock de `portaki-test-utils`. Une valeur changée ici à la main ne
changerait rien en production, seulement ce que les tests laissent passer.

## Mettre à jour

```sh
scripts/sync-platform-contracts.sh          # récupère la version de `main` et l'écrit ici
scripts/sync-platform-contracts.sh --check  # compare sans écrire, sort en 1 si ça diverge
```

`PORTAKI_PLATFORM_REF=<sha|tag>` épingle une autre révision que `main`.

Puis `cargo test -p portaki-sdk --test limits_contract` : il échoue tant qu'une constante de
`limits.rs` diffère du JSON, ou qu'une limite du JSON n'a pas de constante. C'est ce test qui
transforme une nouvelle limite côté plateforme en travail visible côté SDK.

Le test ne touche jamais le réseau ; seul le script le fait.
