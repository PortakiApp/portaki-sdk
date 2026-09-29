/**
 * Assembles the contract bundle for one version of the SDK.
 *
 * The three documents ship together, and that is the whole point: a manifest schema from one
 * version with primitives from another describes no real SDK. The registry does reject a release
 * that is missing one of them.
 *
 *   node scripts/build-contracts-bundle.mjs 2.1.1 > bundle.json
 *
 * With no argument, the version is read from the workspace's Cargo.toml — the same one
 * `portaki build` stamps into a module manifest.
 */
import { readFile } from "node:fs/promises";
import { join } from "node:path";

const ROOT = new URL("..", import.meta.url).pathname;

/**
 * The contracts, under the name the registry files them by.
 *
 * The first three are required by the registry; `deprecations.json` is known to it but optional,
 * because making it mandatory would break reading back the versions already stored, which do not
 * carry one.
 */
const CONTRACTS = {
  "module.v1.json": "schema/module.v1.json",
  "host-ops.json": "contracts/host-ops.json",
  "sdui_primitives.json": "crates/portaki-sdk/sdui_primitives.json",
  // What `sdui_primitives.json` names without describing: the variants of `Action`, the enums'
  // variants, the structs' fields. Derived from the Rust by the `sdui_types_contract` test, and
  // optional like `deprecations.json` — versions published before it exists do not carry one.
  "sdui_types.json": "contracts/sdui_types.json",
  "deprecations.json": "contracts/deprecations.json",
  // The typed responses the platform asks modules for, outside SDUI. The dashboard generates its
  // types from them; optional for the registry, which does not read them.
  "publish-readiness.v1.json": "contracts/publish-readiness.v1.json",
  "stats-summary.v1.json": "contracts/stats-summary.v1.json",
  "timeline-tasks.v1.json": "contracts/timeline-tasks.v1.json",
};

/** The workspace version — the source every published crate derives from. */
async function workspaceVersion() {
  const manifest = await readFile(join(ROOT, "Cargo.toml"), "utf8");
  const found = manifest.match(/^\s*version\s*=\s*"([^"]+)"/m);
  if (!found) {
    throw new Error("no version found in Cargo.toml");
  }
  return found[1];
}

const requested = process.argv[2]?.trim();
const version = requested || (await workspaceVersion());

if (requested) {
  // A tag that does not match the workspace would publish contracts under a version nobody
  // stamps — so nothing would find them when it comes to validating a module.
  const actual = await workspaceVersion();
  if (actual !== requested) {
    throw new Error(
      `the tag says ${requested} but Cargo.toml carries ${actual} — line them up`,
    );
  }
}

const contracts = {};
for (const [name, path] of Object.entries(CONTRACTS)) {
  const raw = await readFile(join(ROOT, path), "utf8");
  try {
    contracts[name] = JSON.parse(raw);
  } catch (failure) {
    throw new Error(`${path} is not valid JSON: ${failure.message}`);
  }
}

process.stdout.write(JSON.stringify({ version, channel: "stable", contracts }));
