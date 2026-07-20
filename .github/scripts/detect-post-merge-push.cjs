/**
 * Push main: skip quality if every commit comes from a merged PR (CI already ran).
 * Loaded via require() from actions/github-script.
 */
module.exports = async function detectPostMergePush({ github, context, core }) {
  const { owner, repo } = context.repo;
  const before = context.payload.before;
  const after = context.payload.after;

  if (!before || before === "0000000000000000000000000000000000000000") {
    core.setOutput("skip_quality", "false");
    return;
  }

  let shas = (context.payload.commits ?? []).map((commit) => commit.id).filter(Boolean);
  if (shas.length === 0) {
    shas = [after];
  }

  if (shas.length >= 20) {
    const comparison = await github.rest.repos.compareCommitsWithBasehead({
      owner,
      repo,
      basehead: `${before}...${after}`,
    });
    shas = comparison.data.commits.map((commit) => commit.sha);
  }

  for (const sha of shas) {
    const { data: prs } = await github.rest.repos.listPullRequestsAssociatedWithCommit({
      owner,
      repo,
      commit_sha: sha,
    });
    const fromMergedPr = prs.some((pr) => Boolean(pr.merged_at));
    if (!fromMergedPr) {
      core.setOutput("skip_quality", "false");
      return;
    }
  }

  core.setOutput("skip_quality", "true");
};
