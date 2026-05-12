// Validates every packages/*/portaki.module.json against schema/module.v1.json
// and enforces product rules (id vs folder, official author, semver, license, lucide icon).

import Ajv from "ajv";
import addFormats from "ajv-formats";
import { readFileSync, readdirSync, existsSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import * as lucide from "lucide-react";

const __dirname = dirname(fileURLToPath(import.meta.url));
const root = resolve(__dirname, "..");
const schemaPath = join(root, "schema", "module.v1.json");

const schema = JSON.parse(readFileSync(schemaPath, "utf8"));
const ajv = new Ajv({ allErrors: true, strict: false });
addFormats(ajv);
const validate = ajv.compile(schema);

const AGPLISH = new Set(["AGPL-3.0", "AGPL-3.0-only", "AGPL-3.0-or-later"]);

function toPascalFromKebab(icon) {
  return icon
    .split("-")
    .map((p) => p.charAt(0).toUpperCase() + p.slice(1))
    .join("");
}

function listPackageDirs() {
  const pkgs = join(root, "packages");
  return readdirSync(pkgs, { withFileTypes: true })
    .filter((d) => d.isDirectory())
    .map((d) => d.name);
}

let failed = false;

for (const dir of listPackageDirs()) {
  const manifestPath = join(root, "packages", dir, "portaki.module.json");
  if (!existsSync(manifestPath)) {
    console.warn(`[skip] packages/${dir}/portaki.module.json missing`);
    continue;
  }
  const raw = readFileSync(manifestPath, "utf8");
  let data;
  try {
    data = JSON.parse(raw);
  } catch (e) {
    console.error(`[error] ${manifestPath}: invalid JSON`, e);
    failed = true;
    continue;
  }
  if (!validate(data)) {
    console.error(`[error] ${manifestPath}: schema`, validate.errors);
    failed = true;
    continue;
  }
  if (data.id !== dir) {
    console.error(`[error] ${manifestPath}: id "${data.id}" must equal folder "${dir}"`);
    failed = true;
  }
  const semver = /^\d+\.\d+\.\d+$/;
  if (!semver.test(data.version)) {
    console.error(`[error] ${manifestPath}: version must be semver X.Y.Z`);
    failed = true;
  }
  if (!AGPLISH.has(data.license)) {
    console.error(
      `[error] ${manifestPath}: license must be AGPL-3.0 (got ${data.license})`,
    );
    failed = true;
  }
  if (data.type === "official" && data.author?.name !== "Portaki") {
    console.error(`[error] ${manifestPath}: official modules require author.name === "Portaki"`);
    failed = true;
  }
  const pascal = toPascalFromKebab(data.icon);
  if (!lucide[pascal]) {
    console.error(
      `[error] ${manifestPath}: icon "${data.icon}" → "${pascal}" not found in lucide-react`,
    );
    failed = true;
  }
}

if (failed) {
  process.exit(1);
}
console.log("All portaki.module.json files validated.");
