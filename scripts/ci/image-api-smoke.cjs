// Run inside the accepted application container; every URL is local and synthetic.
const assert = require('node:assert/strict');
const { setTimeout: delay } = require('node:timers/promises');

async function acceptApi(request = fetch) {
  const base = 'http://127.0.0.1:8012';
  const get = (path, options = {}) => request(base + path, { ...options, signal: AbortSignal.timeout(10000) });
  let healthy = false;
  for (let attempt = 0; attempt < 60; attempt += 1) {
    try { healthy = (await get('/health')).ok; } catch { /* Wait for application startup. */ }
    if (healthy) break;
    await delay(1000);
  }
  assert.ok(healthy, 'application liveness did not become healthy');
  const login = await get('/auth/login', { method: 'POST', headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ client_id: 'apitest01', client_secret: 'test_password' }) });
  assert.ok(login.ok, 'seed user login failed');
  const token = (await login.json()).data.access_token;
  assert.ok(token, 'login did not return a token');
  const headers = { Authorization: 'Bearer ' + token };
  const users = await get('/user', { headers });
  assert.ok(users.ok, 'authenticated database read failed');
  assert.ok((await users.json()).data.some(user => user.username === 'apitest01'), 'seed user is missing from database read');
  const search = await get('/cards/search?q=image-smoke-director&entity_types=director', { headers });
  assert.ok(search.ok, 'PostgreSQL search request failed');
  const results = (await search.json()).data;
  assert.equal(results.search_mode, 'sql_fallback');
  assert.ok(results.total > 0 && results.results.some(item =>
    item.entity_type === 'director' && item.title === 'image-smoke-director'), 'synthetic database search row is missing');
  const bytes = Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+jP4sAAAAASUVORK5CYII=', 'base64');
  const form = new FormData();
  form.set('username', 'image-smoke-user');
  form.set('email', 'image-smoke@example.invalid');
  form.set('profile_picture', new Blob([bytes], { type: 'image/png' }), 'synthetic.png');
  const upload = await get('/user', { method: 'POST', headers, body: form });
  assert.ok(upload.ok, 'synthetic file upload failed');
  const fileId = (await upload.json()).data.file_id;
  assert.ok(fileId, 'upload did not create file metadata');
  const download = await get('/file/' + encodeURIComponent(fileId), { headers });
  assert.ok(download.ok, 'authenticated file download failed');
  assert.deepEqual(Buffer.from(await download.arrayBuffer()), bytes, 'download differs from uploaded bytes');
}

async function main() {
  await acceptApi();
  const { chromium } = require('/usr/local/lib/node_modules/playwright');
  const browser = await chromium.launch({ headless: true, args: ['--no-sandbox'] });
  try {
    const page = await browser.newPage();
    await page.route('**/*', route => route.abort());
    await page.setContent('<html><head><title>Image smoke</title></head><body><div id="result">synthetic</div></body></html>');
    assert.equal(await page.title(), 'Image smoke');
    assert.equal(await page.locator('#result').textContent(), 'synthetic');
  } finally {
    await browser.close();
  }
  console.log('Image API, file storage, PostgreSQL search, and installed Chromium acceptance passed.');
}

module.exports = { acceptApi };
if (!module.parent) main().catch(error => { console.error(error.message); process.exitCode = 1; });
