#!/usr/bin/env sh
# Installe `portaki` et le signe avec une identité stable.
#
# Pourquoi signer : le trousseau macOS attache son autorisation à l'IDENTITÉ de signature, pas
# au contenu du binaire. Sans signature stable, chaque `cargo install` produit aux yeux du
# système un programme inconnu, et « Toujours autoriser » ne vaut que pour celui-là — d'où le
# dialogue à chaque rebuild.
#
# Signé avec la même identité et le même identifiant, chaque build reste le même programme :
# l'autorisation survit, et le secret reste dans le trousseau plutôt que dans un fichier.
#
# Prérequis, une seule fois — voir docs/cli-keychain-macos.md :
#   Trousseaux d'accès ▸ Assistant de certification ▸ Créer un certificat…
#   Nom : Portaki Dev · Type : Signature de code · Auto-signé
#
# Ailleurs que sur macOS, `codesign` n'existe pas : le script installe et s'arrête là.
set -eu

IDENTITY="${PORTAKI_SIGN_IDENTITY:-Portaki Dev}"
IDENTIFIER="app.portaki.cli"

cd "$(dirname "$0")/.."
cargo install --path crates/portaki-cli --force

[ "$(uname -s)" = "Darwin" ] || exit 0

BINARY="${CARGO_INSTALL_ROOT:-${CARGO_HOME:-$HOME/.cargo}}/bin/portaki"

if ! command -v codesign >/dev/null 2>&1; then
  echo "codesign introuvable — binaire non signé, le trousseau redemandera à chaque build." >&2
  exit 0
fi

# `-i` fixe l'identifiant : c'est lui, avec l'identité, qui compose l'exigence désignée que le
# trousseau compare. Le laisser déduire du nom de fichier marcherait aussi, mais l'écrire ici
# rend la stabilité explicite plutôt qu'accidentelle.
if codesign -s "$IDENTITY" -i "$IDENTIFIER" -f "$BINARY" 2>/dev/null; then
  echo "portaki signé « $IDENTITY » — le trousseau ne redemandera qu'une fois."
else
  echo "Identité « $IDENTITY » introuvable dans le trousseau." >&2
  echo "Créez-la une fois (docs/cli-keychain-macos.md), ou passez PORTAKI_SIGN_IDENTITY." >&2
  exit 1
fi
