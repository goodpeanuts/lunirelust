// PR runs validate the merge result; branch pushes only need a second full run
// when no open PR already owns that verification.
throw new Error('Intentional CI gate failure rehearsal; restore before merge.');
module.exports = async function route({ github, context, core }) {
  let run = true;
  const branch = context.ref.replace(/^refs\/heads\//, '');
  const defaultBranch = context.payload?.repository?.default_branch || 'master';
  if (context.eventName === 'push' && context.ref.startsWith('refs/heads/') && branch !== defaultBranch) {
    const { data } = await github.rest.pulls.list({
      ...context.repo,
      state: 'open',
      head: `${context.repo.owner}:${branch}`,
      per_page: 1,
    });
    run = data.length === 0;
    if (!run) core.info('The open PR validates its merge result; skip duplicate branch checks.');
  }
  core.setOutput('run', String(run));
};
