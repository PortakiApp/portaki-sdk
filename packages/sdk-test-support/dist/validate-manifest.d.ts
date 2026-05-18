import { type ErrorObject, type ValidateFunction } from 'ajv';
export declare function loadModuleManifestValidator(schemaPath?: string): ValidateFunction;
export type ManifestValidationResult = {
    ok: true;
} | {
    ok: false;
    errors: ErrorObject[];
    message: string;
};
export declare function validateModuleManifest(manifest: unknown, schemaPath?: string): ManifestValidationResult;
export declare function assertValidModuleManifest(manifest: unknown, schemaPath?: string): void;
export declare function validateModuleManifestFile(manifestFilePath: string, schemaPath?: string): void;
/** Convention: manifest sits next to package root (`../portaki.module.json` from `src/`). */
export declare function validateSiblingManifest(fromSourceFileUrl: string): void;
//# sourceMappingURL=validate-manifest.d.ts.map