# Où la CLI range ses identifiants

## Par défaut : un fichier

`~/.config/portaki/credentials.json`, en `0600`, dossier en `0700`.

- **Hors du dépôt** — un fichier de secrets dans un arbre de travail finit par être commité, ou
  balayé par un `git add -A`.
- **Écrit par renommage atomique** — une interruption ne laisse pas un fichier tronqué, ce qui
  obligerait à se reconnecter pour une raison sans rapport.
- **Jamais affiché**, et `portaki logout` l'efface entièrement.

Le chemin se change par `PORTAKI_CREDENTIALS_FILE`.

## Ce que ce fichier ne fait pas

**Il n'est pas chiffré.** Deux idées reviennent, et aucune ne tient :

- *le hacher* — impossible : un jeton doit être rejoué tel quel, et un condensat ne se rejoue
  pas. Ce qu'on hacherait ne servirait plus à s'authentifier ;
- *le chiffrer* — il faudrait une clé, qu'il faudrait ranger quelque part sur la même machine.
  Le seul endroit correct est le trousseau, celui-là même qu'on vient de quitter. Brouiller le
  contenu sans clé protégée ne protège de rien, ça donne seulement l'air de protéger.

Sur une machine mono-utilisateur, la protection qui compte est celle des droits du fichier, et
elle est en place. Le jeton d'accès vit quinze minutes ; celui de renouvellement, sept jours et
se révoque par `portaki logout`.

## Pourquoi ce n'est plus le trousseau

Le trousseau était le bon choix sur le papier : chiffré au repos, verrouillé avec la session. Son
coût réel sur macOS l'a emporté.

Le trousseau n'autorise pas *un fichier à un emplacement*, il autorise une **identité de code**.
Un binaire recompilé n'a pas la même : chaque `cargo install` produit un programme inconnu, et
« Toujours autoriser » ne vaut que pour l'empreinte du jour. Une boucle de développement qui
recompile redemande donc le mot de passe de session à chaque passage.

Un garde-fou qu'on affronte cent fois par jour finit par être contourné — celui-ci l'était déjà,
par la variable d'environnement.

## Revenir au trousseau

```sh
export PORTAKI_CREDENTIALS=keychain
```

Rien n'a été retiré. Si vous le faites sur macOS et que le dialogue vous lasse, la vraie réponse
est une identité de signature stable :

1. **Trousseaux d'accès ▸ Assistant de certification ▸ Créer un certificat…**
   Nom `Portaki Dev`, type d'identité `Racine auto-signée`, type de certificat `Signature de code`.
2. Installer avec `./scripts/install-cli.sh`, qui signe le binaire après l'avoir installé.

Vérifier que la signature tient sur l'identité et non sur une empreinte :

```sh
codesign -d -r- "$(command -v portaki)"
```

Une exigence qui mentionne un `cdhash` signifie que la signature est restée ad hoc, et le
dialogue reviendra.

## En CI

`PORTAKI_DEV_TOKEN` court-circuite tout : ni fichier, ni trousseau. Un agent de build n'a ni
l'un ni l'autre, et cette variable gagne sur le reste — y compris sur l'OIDC.
