#!/usr/bin/env bash
# Recopie les contrats dont la plateforme est l'autorité dans `contracts/platform/`.
#
# La plateforme (PortakiApp/portaki-platform) fait foi sur ces documents ; le SDK n'en garde
# qu'une copie, que `cargo test` compare à ses constantes. Ce script est le seul chemin par
# lequel la copie change.
#
#   scripts/sync-platform-contracts.sh           écrit les fichiers récupérés
#   scripts/sync-platform-contracts.sh --check   compare sans écrire ; sort en 1 si ça diverge
#
# PORTAKI_PLATFORM_REF épingle une révision (défaut : main).
set -euo pipefail

FILES=(module-limits.json)

REF="${PORTAKI_PLATFORM_REF:-main}"
BASE_URL="https://raw.githubusercontent.com/PortakiApp/portaki-platform/${REF}/contracts"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST_DIR="$ROOT/contracts/platform"

usage() {
  sed -n '2,11p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
}

mode="write"
case "${1:-}" in
  "") ;;
  --check) mode="check" ;;
  -h | --help)
    usage
    exit 0
    ;;
  *)
    usage >&2
    exit 2
    ;;
esac

mkdir -p "$DEST_DIR"

# Les fichiers temporaires vivent dans le dossier de destination : `mv` y reste un renommage,
# donc atomique — jamais de copie à moitié écrite, même si le script est interrompu.
tmp_files=()
cleanup() {
  if ((${#tmp_files[@]})); then
    rm -f "${tmp_files[@]}"
  fi
}
trap cleanup EXIT

drift=0
for file in "${FILES[@]}"; do
  dest="$DEST_DIR/$file"
  tmp="$(mktemp "$DEST_DIR/.${file}.XXXXXX")"
  tmp_files+=("$tmp")

  # --fail : une 404 ou une 500 est une erreur, pas un fichier contenant la page d'erreur.
  if ! curl --fail --silent --show-error --location --proto '=https' --retry 3 \
    --output "$tmp" "$BASE_URL/$file"; then
    echo "error: impossible de récupérer $BASE_URL/$file — rien n'a été modifié" >&2
    exit 1
  fi

  if [[ ! -s "$tmp" ]]; then
    echo "error: $BASE_URL/$file est vide" >&2
    exit 1
  fi

  if [[ "$mode" == check ]]; then
    if [[ ! -f "$dest" ]]; then
      echo "drift: $file absent de contracts/platform/" >&2
      drift=1
    elif ! cmp -s "$tmp" "$dest"; then
      echo "drift: contracts/platform/$file diffère de portaki-platform@$REF" >&2
      diff -u --label "contracts/platform/$file" --label "portaki-platform@$REF" "$dest" "$tmp" >&2 || true
      drift=1
    else
      echo "ok: $file"
    fi
  else
    chmod 644 "$tmp"
    mv -f "$tmp" "$dest"
    echo "synced: $file (portaki-platform@$REF)"
  fi
done

if ((drift)); then
  echo "Lancer scripts/sync-platform-contracts.sh puis cargo test -p portaki-sdk --test limits_contract." >&2
  exit 1
fi
