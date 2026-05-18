import Ajv from 'ajv';
import addFormats from 'ajv-formats';
import { readFileSync, existsSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
const DEFAULT_SCHEMA_URL = 'https://raw.githubusercontent.com/PortakiApp/portaki-sdk/main/schema/module.v1.json';
let cachedValidator = null;
function resolveLocalSchemaPath() {
    const here = dirname(fileURLToPath(import.meta.url));
    const candidates = [
        resolve(here, '../schema/module.v1.json'),
        resolve(here, '../../../schema/module.v1.json'),
        resolve(here, '../../../schema/module.v1.json'),
        resolve(process.cwd(), '../portaki-sdk/schema/module.v1.json'),
        resolve(process.cwd(), '../../portaki-sdk/schema/module.v1.json'),
        resolve(process.cwd(), '../../../portaki-sdk/schema/module.v1.json'),
    ];
    for (const path of candidates) {
        if (existsSync(path)) {
            return path;
        }
    }
    return null;
}
export function loadModuleManifestValidator(schemaPath) {
    if (cachedValidator && !schemaPath) {
        return cachedValidator;
    }
    let schema;
    const local = schemaPath ?? resolveLocalSchemaPath();
    if (local) {
        schema = JSON.parse(readFileSync(local, 'utf8'));
    }
    else {
        throw new Error(`module_manifest_schema_not_found: set schemaPath or clone portaki-sdk (expected ${DEFAULT_SCHEMA_URL})`);
    }
    const ajv = new Ajv({ allErrors: true, strict: false });
    addFormats(ajv);
    const validate = ajv.compile(schema);
    if (!schemaPath) {
        cachedValidator = validate;
    }
    return validate;
}
function normalizeManifestForValidation(manifest) {
    if (manifest == null || typeof manifest !== 'object') {
        return manifest;
    }
    const root = { ...manifest };
    const artifacts = root.artifacts;
    if (artifacts != null && typeof artifacts === 'object') {
        const next = { ...artifacts };
        for (const key of ['wasmUrl', 'guestEsmUrl', 'jarMaven']) {
            if (next[key] === '') {
                delete next[key];
            }
        }
        root.artifacts = next;
    }
    return root;
}
export function validateModuleManifest(manifest, schemaPath) {
    const validate = loadModuleManifestValidator(schemaPath);
    const normalized = normalizeManifestForValidation(manifest);
    const ok = validate(normalized) === true;
    if (ok) {
        return { ok: true };
    }
    const errors = (validate.errors ?? []);
    const message = errors.map((e) => `${e.instancePath} ${e.message}`).join('; ');
    return { ok: false, errors, message };
}
export function assertValidModuleManifest(manifest, schemaPath) {
    const result = validateModuleManifest(manifest, schemaPath);
    if (!result.ok) {
        throw new Error(`invalid portaki.module.json: ${result.message}`);
    }
}
export function validateModuleManifestFile(manifestFilePath, schemaPath) {
    const raw = JSON.parse(readFileSync(manifestFilePath, 'utf8'));
    assertValidModuleManifest(raw, schemaPath);
}
/** Convention: manifest sits next to package root (`../portaki.module.json` from `src/`). */
export function validateSiblingManifest(fromSourceFileUrl) {
    const dir = dirname(fileURLToPath(fromSourceFileUrl));
    const manifestPath = join(dir, '..', 'portaki.module.json');
    validateModuleManifestFile(manifestPath);
}
