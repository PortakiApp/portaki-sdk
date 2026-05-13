#!/usr/bin/env bash
set -euo pipefail

SDK_REL="${SDK_RELATIVE_PATH:-.deps/portaki-sdk}"
SDK_DIR="${GITHUB_WORKSPACE:?}/${SDK_REL}"

if [[ ! -d "$SDK_DIR" ]]; then
  echo "Expected portaki-sdk checkout at ${SDK_REL} (resolved: ${SDK_DIR})"
  exit 1
fi

cd "$SDK_DIR"

echo "Installing app.portaki:portaki-module-sdk into local ~/.m2 from source…"

if mvn -B install -pl :portaki-module-sdk -am -DskipTests; then
  exit 0
fi

echo "Reactor install failed; trying full install from repo root…"
mvn -B install -DskipTests
