#!/usr/bin/env bash
set -euo pipefail

SDK_REL="${SDK_RELATIVE_PATH:-.deps/portaki-sdk}"
SDK_DIR="${GITHUB_WORKSPACE:?}/${SDK_REL}"
SDK_POM="${SDK_DIR}/sdk/java/pom.xml"

if [[ ! -f "$SDK_POM" ]]; then
  echo "Expected portaki-sdk Java POM at sdk/java/pom.xml (resolved: ${SDK_POM})"
  exit 1
fi

echo "Installing app.portaki:portaki-module-sdk into local ~/.m2 from ${SDK_POM}…"
mvn -B install -f "$SDK_POM" -DskipTests

if [[ -f "${SDK_DIR}/sdk/java-test-support/pom.xml" ]]; then
  echo "Installing app.portaki:portaki-module-sdk-test into local ~/.m2…"
  mvn -B install -f "${SDK_DIR}/sdk/java-test-support/pom.xml" -DskipTests
fi
