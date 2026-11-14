const assert = require('node:assert/strict');
const { test } = require('node:test');
const { existsSync } = require('node:fs');
const path = require('node:path');
const helper = path.resolve('.github/scripts/find-artifact.cjs');

for (const [name, artifacts, expected] of [
  ['missing', [], 'false'],
  ['retained', [{ name: 'release-image', expired: false, id: 7 }], 'true'],
]) {
  test(`artifact availability lookup: ${name}`, async () => {
    assert.ok(existsSync(helper), 'artifact reuse helper is missing');
    const outputs = {};
    await require(helper)({ name: 'release-image', context: { repo: { owner: 'test', repo: 'test' }, runId: 99 },
      github: { rest: { actions: { listWorkflowRunArtifacts: 'list' } }, paginate: async (_, args) => {
        assert.equal(args.run_id, 99);
        return artifacts;
      } }, core: { setOutput: (key, value) => { outputs[key] = value; } } });
    assert.equal(outputs.exists, expected);
  });
}

test('explicitly expired artifact entries and API errors fail lookup', async () => {
  assert.ok(existsSync(helper), 'artifact reuse helper is missing');
  const args = { name: 'release-image', context: { repo: {}, runId: 99 },
    github: { rest: { actions: {} }, paginate: async () => [{ name: 'release-image', expired: true }] },
    core: { setOutput: () => assert.fail('must fail closed') } };
  await assert.rejects(require(helper)(args), /expired/);
  args.github.paginate = async () => { throw new Error('authentication failed'); };
  await assert.rejects(require(helper)(args), /authentication/);
});
