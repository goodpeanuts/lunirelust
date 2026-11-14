const assert = require('node:assert/strict');
const { test } = require('node:test');
const { mkdtempSync, mkdirSync, writeFileSync, readFileSync, rmSync } = require('node:fs');
const { join, resolve } = require('node:path');
const { tmpdir } = require('node:os');
const { spawnSync } = require('node:child_process');

test('private dependency access is process-only and missing credentials fail closed', () => {
  const scratch = mkdtempSync(join(tmpdir(), 'lunirelust-credential-test-'));
  try {
    const bin = join(scratch, 'bin');
    mkdirSync(bin);
    const config = join(scratch, 'gitconfig');
    const original = '[user]\n\tname = Synthetic test\n';
    writeFileSync(config, original);
    writeFileSync(join(bin, 'cargo'), `#!/usr/bin/env node
const assert = require('node:assert/strict');
const { execFileSync } = require('node:child_process');
assert.deepEqual(process.argv.slice(2), ['fetch', '--locked', '--target', 'synthetic-target']);
assert.equal(process.env.CARGO_NET_GIT_FETCH_WITH_CLI, 'true');
assert.equal(process.env.GIT_TERMINAL_PROMPT, '0');
assert.ok(!process.env.GIT_CONFIG_VALUE_0.includes(process.env.GIT_CREDENTIALS));
const result = execFileSync('git', ['credential', 'fill'], {
  input: 'protocol=https\\nhost=github.com\\n\\n', encoding: 'utf8'
});
assert.ok(result.includes('username=x-access-token'));
assert.ok(result.includes('password=' + process.env.GIT_CREDENTIALS));
`, { mode: 0o755 });
    const environment = {
      ...process.env,
      PATH: bin + ':' + process.env.PATH,
      GIT_CONFIG_GLOBAL: config,
      GIT_CONFIG_NOSYSTEM: '1',
      GIT_CREDENTIALS: 'synthetic-only-credential',
    };
    const script = resolve('.github/scripts/fetch-dependencies.sh');
    const result = spawnSync('bash', [script, '--target', 'synthetic-target'], {
      env: environment, encoding: 'utf8',
    });
    assert.equal(result.status, 0, result.stderr);
    assert.ok(!result.stdout.includes(environment.GIT_CREDENTIALS));
    assert.ok(!result.stderr.includes(environment.GIT_CREDENTIALS));
    assert.equal(readFileSync(config, 'utf8'), original);
    environment.GIT_CREDENTIALS = '';
    const missing = spawnSync('bash', [script], { env: environment, encoding: 'utf8' });
    assert.equal(missing.status, 1);
    assert.match(missing.stderr, /Private dependency access is required/);
  } finally {
    rmSync(scratch, { recursive: true, force: true });
  }
});
