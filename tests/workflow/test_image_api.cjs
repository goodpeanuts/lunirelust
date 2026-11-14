const assert = require('node:assert/strict');
const { test } = require('node:test');
const { existsSync } = require('node:fs');
const helper = '../../scripts/ci/image-api-smoke.cjs';

function fixture(overrides = {}) {
  const calls = [];
  let uploaded;
  const fetch = async (url, options = {}) => {
    const route = new URL(url).pathname;
    calls.push(route);
    if (route === '/health') return new Response('ok');
    if (route === '/auth/login') {
      assert.deepEqual(JSON.parse(options.body), { client_id: 'apitest01', client_secret: 'test_password' });
      return Response.json({ data: { access_token: 'synthetic-token' } });
    }
    assert.equal(options.headers.Authorization, 'Bearer synthetic-token');
    if (route === '/user' && options.method !== 'POST') {
      return Response.json({ data: overrides.users ?? [{ username: 'apitest01' }] });
    }
    if (route === '/cards/search') return Response.json({ data: {
      search_mode: overrides.mode ?? 'sql_fallback', total: 1,
      results: [{ title: 'image-smoke-director', entity_type: 'director' }],
    } });
    if (route === '/user') {
      uploaded = Buffer.from(await options.body.get('profile_picture').arrayBuffer());
      return Response.json({ data: { file_id: 'synthetic-file' } });
    }
    if (route === '/file/synthetic-file') return new Response(overrides.file ?? uploaded);
    throw new Error('Unexpected route: ' + route);
  };
  return { fetch, calls };
}

test('image API acceptance checks authenticated database/search and exact upload bytes', async () => {
  assert.ok(existsSync(require('node:path').resolve('scripts/ci/image-api-smoke.cjs')), 'API acceptance helper missing');
  const fixtureData = fixture();
  await require(helper).acceptApi(fixtureData.fetch);
  assert.deepEqual(fixtureData.calls, ['/health', '/auth/login', '/user', '/cards/search', '/user', '/file/synthetic-file']);
});

test('liveness alone, wrong search mode, and altered file bytes cannot pass image acceptance', async () => {
  assert.ok(existsSync(require('node:path').resolve('scripts/ci/image-api-smoke.cjs')), 'API acceptance helper missing');
  for (const overrides of [{ users: [] }, { mode: 'keyword_only' }, { file: Buffer.from('wrong') }]) {
    await assert.rejects(require(helper).acceptApi(fixture(overrides).fetch));
  }
});
