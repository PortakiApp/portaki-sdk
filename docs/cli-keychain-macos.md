# Le trousseau redemande son mot de passe à chaque build

## Le symptôme

```
portaki-14043a3d45f57b96 veut utiliser vos informations confidentielles
gardées dans « app.portaki.cli » de votre trousseau.
```

À chaque `portaki dev`, après chaque rebuild. « Toujours autoriser » ne change rien : le
dialogue revient au build suivant.

## Pourquoi

Le trousseau macOS n'autorise pas « un fichier à cet emplacement », il autorise une **identité
de code**. Un binaire non signé — ou signé à la volée — n'a pas d'identité stable : son
empreinte change à chaque compilation, et le système voit un programme inconnu qui demande le
secret d'un autre. « Toujours autoriser » enregistre l'autorisation pour *cette* empreinte, que
le build suivant invalide.

Ce n'est donc pas un défaut de la CLI, et le corriger dans son code reviendrait à sortir le
secret du trousseau — ce que ce projet refuse : voir l'en-tête de `crates/portaki-cli/src/auth.rs`.

## Le correctif, une fois

Créer une identité de signature locale. Elle ne sert qu'à votre machine, ne coûte rien et
n'a rien à voir avec un compte développeur Apple.

1. Ouvrir **Trousseaux d'accès**.
2. Menu **Trousseaux d'accès ▸ Assistant de certification ▸ Créer un certificat…**
3. Renseigner :
   - **Nom** : `Portaki Dev`
   - **Type d'identité** : `Racine auto-signée`
   - **Type de certificat** : `Signature de code`
4. Créer, puis fermer.

## Ensuite, à chaque installation

```sh
./scripts/install-cli.sh
```

Le script installe puis signe avec `Portaki Dev`. Le premier lancement redemandera le mot de
passe une dernière fois : cliquer **Toujours autoriser**. Les suivants ne demanderont plus, y
compris après un rebuild — l'exigence désignée porte sur l'identité et l'identifiant, tous deux
inchangés.

Une autre identité : `PORTAKI_SIGN_IDENTITY="Mon identité" ./scripts/install-cli.sh`.

## Vérifier

```sh
codesign -d -r- "$(command -v portaki)"
```

L'exigence affichée doit nommer `app.portaki.cli` et le certificat, jamais une empreinte de
fichier. Si le dialogue revient malgré tout après un rebuild, c'est cette sortie qu'il faut
lire : une exigence qui mentionne un `cdhash` signifie que la signature est restée ad hoc.

## Le dépannage rapide, sans rien signer

`PORTAKI_DEV_TOKEN` court-circuite le trousseau — `auth.rs` le lit avant lui, et il gagne sur
tout, y compris sur l'OIDC d'une CI :

```sh
export PORTAKI_DEV_TOKEN="…"
```

Ça dépanne une session, pas une journée : le jeton d'accès vit quinze minutes, et son
renouvellement, lui, retourne au trousseau.

## Ailleurs que sur macOS

Le problème n'existe pas sous cette forme. Linux passe par Secret Service, Windows par
Credential Manager, et ni l'un ni l'autre n'attache son autorisation à l'empreinte du binaire.
`scripts/install-cli.sh` installe et s'arrête là.
