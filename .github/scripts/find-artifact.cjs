// Reuse an artifact only while GitHub still lists it for this run.
// Full workflow reruns clear earlier artifacts; this lookup does not preserve them.
// Propagate lookup errors and reject entries explicitly marked expired.
module.exports = async function findArtifact({ github, context, core, name }) {
  const artifacts = await github.paginate(github.rest.actions.listWorkflowRunArtifacts, {
    ...context.repo, run_id: context.runId, per_page: 100,
  });
  const matches = artifacts.filter(artifact => artifact.name === name);
  if (matches.length > 1) throw new Error(`Ambiguous artifact: ${name}`);
  if (matches.some(artifact => artifact.expired)) {
    throw new Error(`Artifact ${name} expired; inspect existing publication before choosing recovery.`);
  }
  core.setOutput('exists', String(matches.length === 1));
};
