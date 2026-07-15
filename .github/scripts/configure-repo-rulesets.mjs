#!/usr/bin/env node
/**
 * Configure repository rulesets (replaces classic branch protection).
 *
 * Prerequisites:
 *   - Org CI GitHub App installed on this repo
 *   - CI_APP_ID (App ID, not installation ID) — same secret as Actions workflows
 *
 * Usage (org admin):
 *   CI_APP_ID=123456 node .github/scripts/configure-repo-rulesets.mjs
 *
 * Variables:
 *   GITHUB_REPO               — owner/repo (default: PortakiApp/portaki-sdk)
 *   CI_APP_ID                 — GitHub App ID for bypass Integration (required)
 *   REQUIRED_REVIEWS          — PR approvals (default: 0)
 *   REMOVE_BRANCH_PROTECTION  — if 0, keep classic protection (default: 1)
 *   WITHOUT_CI_BYPASS=1       — branch rulesets without CI bypass (no tag ruleset)
 *   DRY_RUN=1                 — print payloads without calling the API
 *
 * Setup: .github/BRANCH_POLICY.md
 */

const REPO = process.env.GITHUB_REPO ?? 'PortakiApp/portaki-sdk';
const CI_APP_ID = Number.parseInt(process.env.CI_APP_ID ?? '', 10);
const REQUIRED_REVIEWS = Number(process.env.REQUIRED_REVIEWS ?? '0');
const REMOVE_BRANCH_PROTECTION = process.env.REMOVE_BRANCH_PROTECTION !== '0';
const WITHOUT_CI_BYPASS = process.env.WITHOUT_CI_BYPASS === '1';
const DRY_RUN = process.env.DRY_RUN === '1';

const GITHUB_ACTIONS_APP_ID = 15368;

/** Aggregator job — fmt, clippy, test, doc. */
const CI_QUALITY_CHECK = 'quality';

const LONG_LIVED_BRANCHES = ['main'];

const RULESET_NAMES = {
  integrity: 'portaki-sdk: branch integrity',
  main: 'portaki-sdk: main integration',
  tags: 'portaki-sdk: release tags',
};

const TOKEN = process.env.GITHUB_TOKEN ?? process.env.GH_TOKEN;
if (!TOKEN && !DRY_RUN) {
  console.error('GITHUB_TOKEN or GH_TOKEN required (repo + admin).');
  process.exit(1);
}

if (!Number.isFinite(CI_APP_ID) && !DRY_RUN && !WITHOUT_CI_BYPASS) {
  console.error('CI_APP_ID required — same App ID as Actions secret CI_APP_ID');
  console.error('Or WITHOUT_CI_BYPASS=1 for branch rulesets without CI bypass (tags omitted).');
  process.exit(1);
}

const CI_APP_ID_EFFECTIVE = Number.isFinite(CI_APP_ID) ? CI_APP_ID : 0;

const MAIN_INTEGRATION_CHECKS = [{ context: CI_QUALITY_CHECK, integration_id: GITHUB_ACTIONS_APP_ID }];

/** @param {'always' | 'pull_request'} [mode] */
function ciAppBypass(mode = 'always') {
  if (WITHOUT_CI_BYPASS || CI_APP_ID_EFFECTIVE <= 0) return [];
  return [
    {
      actor_id: CI_APP_ID_EFFECTIVE,
      actor_type: 'Integration',
      bypass_mode: mode,
    },
  ];
}

function pullRequestRule() {
  return {
    type: 'pull_request',
    parameters: {
      required_approving_review_count: REQUIRED_REVIEWS,
      dismiss_stale_reviews_on_push: true,
      require_code_owner_review: false,
      require_last_push_approval: false,
      required_review_thread_resolution: true,
      allowed_merge_methods: ['rebase'],
    },
  };
}

/** @param {{ context: string, integration_id: number }[]} checks @param {boolean} strict */
function requiredStatusChecksRule(checks, strict) {
  return {
    type: 'required_status_checks',
    parameters: {
      strict_required_status_checks_policy: strict,
      required_status_checks: checks,
    },
  };
}

function integrityRules() {
  return [{ type: 'non_fast_forward' }, { type: 'deletion' }];
}

/** @returns {object[]} */
function buildRulesetDefinitions() {
  const branchRefs = LONG_LIVED_BRANCHES.map((branch) => `refs/heads/${branch}`);

  return [
    {
      name: RULESET_NAMES.integrity,
      target: 'branch',
      bypass_actors: ciAppBypass(),
      conditions: {
        ref_name: {
          include: branchRefs,
          exclude: [],
        },
      },
      rules: integrityRules(),
    },
    {
      name: RULESET_NAMES.main,
      target: 'branch',
      bypass_actors: ciAppBypass('always'),
      conditions: {
        ref_name: {
          include: ['refs/heads/main'],
          exclude: [],
        },
      },
      rules: [
        ...integrityRules(),
        pullRequestRule(),
        requiredStatusChecksRule(MAIN_INTEGRATION_CHECKS, true),
      ],
    },
    {
      name: RULESET_NAMES.tags,
      target: 'tag',
      bypass_actors: ciAppBypass(),
      conditions: {
        ref_name: {
          include: ['refs/tags/v*'],
          exclude: [],
        },
      },
      rules: [{ type: 'creation' }, { type: 'update' }, { type: 'deletion' }],
    },
  ].filter((definition) => {
    if (definition.name !== RULESET_NAMES.tags) return true;
    if (WITHOUT_CI_BYPASS || CI_APP_ID_EFFECTIVE <= 0) return false;
    return true;
  });
}

async function gh(method, path, body) {
  const url = path.startsWith('http') ? path : `https://api.github.com${path}`;
  if (DRY_RUN) {
    console.log(`[dry-run] ${method} ${path}`, body ? JSON.stringify(body, null, 2) : '');
    return body?.__dryRunResponse ?? {};
  }
  const res = await fetch(url, {
    method,
    headers: {
      Authorization: `Bearer ${TOKEN}`,
      Accept: 'application/vnd.github+json',
      'X-GitHub-Api-Version': '2022-11-28',
      ...(body ? { 'Content-Type': 'application/json' } : {}),
    },
    body: body ? JSON.stringify(body) : undefined,
  });
  if (!res.ok) {
    const text = await res.text();
    throw new Error(`${method} ${path} → ${res.status}: ${text}`);
  }
  if (res.status === 204) return {};
  return res.json();
}

async function listRulesets() {
  const response = await gh('GET', `/repos/${REPO}/rulesets`);
  if (DRY_RUN) return [];
  return Array.isArray(response) ? response : [];
}

async function upsertRuleset(definition) {
  const existing = (await listRulesets()).find((ruleset) => ruleset.name === definition.name);
  const payload = {
    name: definition.name,
    target: definition.target,
    enforcement: 'active',
    bypass_actors: definition.bypass_actors,
    conditions: definition.conditions,
    rules: definition.rules,
  };

  console.log(`→ Ruleset ${definition.name}`);
  console.log(`   target: ${definition.target}`);
  console.log(
    `   bypass: ${definition.bypass_actors.length ? `Integration app ${CI_APP_ID_EFFECTIVE} (${definition.bypass_actors[0]?.bypass_mode ?? 'always'})` : 'none'}`,
  );
  console.log(`   rules: ${definition.rules.map((rule) => rule.type).join(', ')}`);

  if (existing?.id) {
    await gh('PUT', `/repos/${REPO}/rulesets/${existing.id}`, payload);
    console.log(`   updated (#${existing.id})`);
    return;
  }

  const created = await gh('POST', `/repos/${REPO}/rulesets`, payload);
  console.log(`   created (#${created.id ?? '?'})`);
}

async function removeBranchProtection(branch) {
  console.log(`→ Remove classic branch protection: ${branch}`);
  try {
    await gh('DELETE', `/repos/${REPO}/branches/${encodeURIComponent(branch)}/protection`);
    console.log('   removed');
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    if (message.includes('404')) {
      console.log('   absent — skip');
      return;
    }
    throw error;
  }
}

async function patchRepoSettings() {
  console.log('→ Repo merge settings');
  await gh('PATCH', `/repos/${REPO}`, {
    delete_branch_on_merge: true,
    allow_squash_merge: false,
    allow_merge_commit: false,
    allow_rebase_merge: true,
    allow_update_branch: true,
  });
}

async function main() {
  console.log(`Repo: ${REPO}`);
  if (WITHOUT_CI_BYPASS) {
    console.log('⚠ WITHOUT_CI_BYPASS=1 — tag ruleset v* omitted; re-run with CI_APP_ID when secrets are ready.');
  }

  await patchRepoSettings();

  for (const definition of buildRulesetDefinitions()) {
    await upsertRuleset(definition);
  }

  if (REMOVE_BRANCH_PROTECTION) {
    for (const branch of LONG_LIVED_BRANCHES) {
      await removeBranchProtection(branch);
    }
  } else {
    console.log('→ REMOVE_BRANCH_PROTECTION=0 — classic branch protection kept');
  }

  console.log('\nRulesets active — rebase merge only, strict on main.');
  console.log('Release-please + tags v* via CI App bypass.');
  console.log('Required repo secrets: CI_APP_ID, CI_APP_PRIVATE_KEY');
  console.log('Done. Settings → Rules → verify portaki-sdk:*.');
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
