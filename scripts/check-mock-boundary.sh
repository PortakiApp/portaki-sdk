#!/usr/bin/env bash
# Proves that `portaki-connectors`' mock responses cannot reach a published module.
#
# Builds a throwaway module outside this workspace — `portaki-connectors` as a normal
# dependency, `portaki-test-utils` as a dev-dependency, exactly as `portaki init` lays a
# module out — for wasm32-unknown-unknown in release, and looks for the mock marker in
# the produced .wasm. It must not be there: the `mock` feature is off by default and
# resolver v2 does not unify a dev-dependency's features into a build with no test target.
#
# The second build is the control: it turns the feature on and references the table, so a
# marker that never showed up would prove the grep broken rather than the boundary sound.
#
# Not wired into CI (runner minutes). Run it when the mock layer or its gating changes.
set -euo pipefail

MARKER='portakiMock'
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PROBE="$(mktemp -d)"
trap 'rm -rf "$PROBE"' EXIT

cat >"$PROBE/Cargo.toml" <<EOF
[workspace]

[package]
name = "boundary-probe"
version = "0.0.0"
edition = "2021"
publish = false

[lib]
crate-type = ["cdylib"]

[features]
leak = ["portaki-connectors/mock"]

[dependencies]
portaki-connectors = { path = "$ROOT/crates/portaki-connectors" }

[dev-dependencies]
portaki-test-utils = { path = "$ROOT/crates/portaki-test-utils" }
EOF

# Same rustflag `portaki init` lays down, or getrandom refuses to build for wasm32.
mkdir -p "$PROBE/.cargo" "$PROBE/src"
cat >"$PROBE/.cargo/config.toml" <<'EOF'
[target.wasm32-unknown-unknown]
rustflags = ['--cfg', 'getrandom_backend="custom"']
EOF

cat >"$PROBE/src/lib.rs" <<'EOF'
use portaki_connectors::open_weather::{CurrentArgs, OpenWeather};

/// Stands in for a module surface: real connector call, nothing else.
#[no_mangle]
pub extern "C" fn probe() -> f64 {
    OpenWeather::current(&CurrentArgs {
        lat: 43.55,
        lng: 7.01,
    })
    .map(|weather| weather.temp_c)
    .unwrap_or(0.0)
}

/// Control arm only — a published module has no way to write this. An exported symbol
/// handing back a pointer into the table pins those bytes in the artifact.
#[cfg(feature = "leak")]
#[no_mangle]
pub extern "C" fn probe_mock(connector: usize, operation: usize) -> *const u8 {
    portaki_connectors::mock::ALL[connector].1[operation].1.as_ptr()
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_dev_dependency_is_really_used() {
        portaki_test_utils::MockContext::guest()
            .with_builtin_connectors()
            .run(|_ctx| {});
    }
}
EOF

build() {
    ( cd "$PROBE" && cargo build --release --quiet --target wasm32-unknown-unknown "$@" )
    ls "$PROBE/target/wasm32-unknown-unknown/release/"*.wasm
}

echo "== published shape: default features, dev-dependency on portaki-test-utils"
WASM="$(build)"
if grep -q "$MARKER" "$WASM"; then
    echo "FAIL: '$MARKER' found in $WASM — the mocks reached the published artifact." >&2
    exit 1
fi
echo "ok — no '$MARKER' in $(basename "$WASM") ($(wc -c <"$WASM") bytes)"

echo "== control: --features leak (portaki-connectors/mock on, table referenced)"
WASM="$(build --features leak)"
if ! grep -q "$MARKER" "$WASM"; then
    echo "FAIL: control build has no '$MARKER' — this check proves nothing." >&2
    exit 1
fi
echo "ok — '$MARKER' present when the feature is on ($(wc -c <"$WASM") bytes)"

echo "boundary holds."
