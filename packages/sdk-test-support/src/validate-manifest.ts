import Ajv, { type ErrorObject, type ValidateFunction } from 'ajv'
import addFormats from 'ajv-formats'
import { readFileSync, existsSync } from 'node:fs'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const DEFAULT_SCHEMA_URL =
  'https://raw.githubusercontent.com/PortakiApp/portaki-sdk/main/schema/module.v1.json'

let cachedValidator: ValidateFunction | null = null

function resolveLocalSchemaPath(): string | null {
  const here = dirname(fileURLToPath(import.meta.url))
  const candidates = [
    resolve(here, '../schema/module.v1.json'),
    resolve(here, '../../../schema/module.v1.json'),
    resolve(here, '../../../schema/module.v1.json'),
    resolve(process.cwd(), '../portaki-sdk/schema/module.v1.json'),
    resolve(process.cwd(), '../../portaki-sdk/schema/module.v1.json'),
    resolve(process.cwd(), '../../../portaki-sdk/schema/module.v1.json'),
  ]
  for (const path of candidates) {
    if (existsSync(path)) {
      return path
    }
  }
  return null
}

export function loadModuleManifestValidator(schemaPath?: string): ValidateFunction {
  if (cachedValidator && !schemaPath) {
    return cachedValidator
  }

  let schema: object
  const local = schemaPath ?? resolveLocalSchemaPath()
  if (local) {
    schema = JSON.parse(readFileSync(local, 'utf8')) as object
  } else {
    throw new Error(
      `module_manifest_schema_not_found: set schemaPath or clone portaki-sdk (expected ${DEFAULT_SCHEMA_URL})`,
    )
  }

  const ajv = new Ajv({ allErrors: true, strict: false })
  addFormats(ajv)
  const validate = ajv.compile(schema)
  if (!schemaPath) {
    cachedValidator = validate
  }
  return validate
}

export type ManifestValidationResult =
  | { ok: true }
  | { ok: false; errors: ErrorObject[]; message: string }

function normalizeManifestForValidation(manifest: unknown): unknown {
  if (manifest == null || typeof manifest !== 'object') {
    return manifest
  }
  const root = { ...(manifest as Record<string, unknown>) }
  const artifacts = root.artifacts
  if (artifacts != null && typeof artifacts === 'object') {
    const next = { ...(artifacts as Record<string, unknown>) }
    for (const key of ['wasmUrl', 'guestEsmUrl', 'jarMaven'] as const) {
      if (next[key] === '') {
        delete next[key]
      }
    }
    root.artifacts = next
  }
  return root
}

export function validateModuleManifest(
  manifest: unknown,
  schemaPath?: string,
): ManifestValidationResult {
  const validate = loadModuleManifestValidator(schemaPath)
  const normalized = normalizeManifestForValidation(manifest)
  const ok = validate(normalized) === true
  if (ok) {
    return { ok: true }
  }
  const errors = (validate.errors ?? []) as ErrorObject[]
  const message = errors.map((e) => `${e.instancePath} ${e.message}`).join('; ')
  return { ok: false, errors, message }
}

export function assertValidModuleManifest(manifest: unknown, schemaPath?: string): void {
  const result = validateModuleManifest(manifest, schemaPath)
  if (!result.ok) {
    throw new Error(`invalid portaki.module.json: ${result.message}`)
  }
}

export function validateModuleManifestFile(manifestFilePath: string, schemaPath?: string): void {
  const raw = JSON.parse(readFileSync(manifestFilePath, 'utf8')) as unknown
  assertValidModuleManifest(raw, schemaPath)
}

/** Convention: manifest sits next to package root (`../portaki.module.json` from `src/`). */
export function validateSiblingManifest(fromSourceFileUrl: string): void {
  const dir = dirname(fileURLToPath(fromSourceFileUrl))
  const manifestPath = join(dir, '..', 'portaki.module.json')
  validateModuleManifestFile(manifestPath)
}
