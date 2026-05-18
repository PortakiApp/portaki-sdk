/**
 * @file index.ts
 * @brief Public API for `@portaki/sdk-test-support` — module author test harness.
 *
 * @details
 * Provides Vitest presets, catalogue manifest validation (AJV + `module.v1.json`),
 * deterministic fixtures, and Testing Library helpers to render guest/host surfaces.
 *
 * @copyright Portaki — SPDX-License-Identifier: MIT
 * @package @portaki/sdk-test-support
 * @addtogroup sdk_test_support Module test support
 * @{
 */

export {
  FIXTURE_PROPERTY,
  FIXTURE_STAY,
  createMockHostModuleContext,
  createMockModuleContext,
  createMockPortakiRuntimeValue,
  createSpyTrack,
  type MockModuleContextOverrides,
  type MockPortakiRuntimeValue,
} from './fixtures.js'

export {
  assertGuestSurface,
  assertHostSurface,
  assertModuleDefinition,
} from './assert-module-definition.js'

export {
  assertValidModuleManifest,
  loadModuleManifestValidator,
  validateModuleManifest,
  validateModuleManifestFile,
  validateSiblingManifest,
  type ManifestValidationResult,
} from './validate-manifest.js'

export {
  renderGuestModule,
  renderHostModule,
  type RenderGuestModuleOptions,
} from './render-module.js'

/** @} */
