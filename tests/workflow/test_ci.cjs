const assert = require('node:assert/strict');
const { test } = require('node:test');
const route = require('../../.github/scripts/route-ci.cjs');

function fixture(eventName, ref, open = []) {
  const outputs = {};
  const requests = [];
  return {
    outputs, requests,
    context: { eventName, ref, repo: { owner: 'example', repo: 'backend' } },
    github: { rest: { pulls: { list: async (request) => {
      requests.push(request);
      return { data: open };
    } } } },
    core: { setOutput: (key, value) => { outputs[key] = value; }, info: () => {} },
  };
}

for (const [event, ref] of [['pull_request', 'refs/pull/1/merge'], ['push', 'refs/heads/master'], ['workflow_dispatch', 'refs/heads/topic']]) {
  test(`${event} ${ref} always validates the selected revision`, async () => {
    const args = fixture(event, ref, [{ number: 1 }]);
    await route(args);
    assert.equal(args.outputs.run, 'true');
    assert.equal(args.requests.length, 0);
  });
}

test('a new branch gets full checks before a PR exists', async () => {
  const args = fixture('push', 'refs/heads/feature/native');
  await route(args);
  assert.equal(args.outputs.run, 'true');
  assert.equal(args.requests[0].head, 'example:feature/native');
});

test('an open PR owns full checks for subsequent branch pushes', async () => {
  const args = fixture('push', 'refs/heads/feature/native', [{ number: 1 }]);
  await route(args);
  assert.equal(args.outputs.run, 'false');
  assert.equal(args.requests[0].state, 'open');
});

test('a GitHub lookup failure cannot silently skip verification', async () => {
  const args = fixture('push', 'refs/heads/feature/native');
  args.github.rest.pulls.list = async () => { throw new Error('API unavailable'); };
  await assert.rejects(route(args), /API unavailable/);
  assert.equal(args.outputs.run, undefined);
});

test('a renamed default branch still gets post-merge verification', async () => {
  const args = fixture('push', 'refs/heads/main', [{ number: 1 }]);
  args.context.payload = { repository: { default_branch: 'main' } };
  await route(args);
  assert.equal(args.outputs.run, 'true');
  assert.equal(args.requests.length, 0);
});
