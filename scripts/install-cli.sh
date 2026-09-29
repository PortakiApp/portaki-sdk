#!/usr/bin/env sh
# Installs `portaki` and signs it with a stable identity.
#
# Why sign: the macOS keychain attaches its authorisation to the signing IDENTITY, not to the
# binary's contents. Without a stable signature, every `cargo install` produces what the system
# sees as an unknown program, and "Always Allow" only ever covers that one — hence the dialog on
# every rebuild.
#
# Signed with the same identity and the same identifier, every build stays the same program: the
# authorisation survives, and the secret stays in the keychain rather than in a file.
#
# One-time prerequisite — see docs/cli-keychain-macos.md:
#   Keychain Access ▸ Certificate Assistant ▸ Create a Certificate…
#   Name: Portaki Dev · Type: Code Signing · Self signed
#
# Anywhere other than macOS, `codesign` does not exist: the script installs and stops there.
set -eu

IDENTITY="${PORTAKI_SIGN_IDENTITY:-Portaki Dev}"
IDENTIFIER="app.portaki.cli"

cd "$(dirname "$0")/.."
cargo install --path crates/portaki-cli --force

[ "$(uname -s)" = "Darwin" ] || exit 0

BINARY="${CARGO_INSTALL_ROOT:-${CARGO_HOME:-$HOME/.cargo}}/bin/portaki"

if ! command -v codesign >/dev/null 2>&1; then
  echo "codesign not found — binary left unsigned, the keychain will ask again on every build." >&2
  exit 0
fi

# `-i` pins the identifier: together with the identity, it is what makes up the designated
# requirement the keychain compares against. Letting it be inferred from the file name would work
# too, but writing it here makes the stability explicit rather than accidental.
if codesign -s "$IDENTITY" -i "$IDENTIFIER" -f "$BINARY" 2>/dev/null; then
  echo "portaki signed as \"$IDENTITY\" — the keychain will only ask once."
else
  echo "Identity \"$IDENTITY\" not found in the keychain." >&2
  echo "Create it once (docs/cli-keychain-macos.md), or pass PORTAKI_SIGN_IDENTITY." >&2
  exit 1
fi
