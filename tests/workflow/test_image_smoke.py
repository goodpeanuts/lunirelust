"""Image smoke lifecycle tests with fake Docker; no daemon or image build."""
import gzip
import hashlib
import json
import os
from pathlib import Path
import signal
import subprocess
import tempfile
import time
import unittest

ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / 'scripts/ci/image-smoke.sh'


class SmokeTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix='image-smoke-contract-')
        self.addCleanup(self.temporary.cleanup)
        self.directory = Path(self.temporary.name)
        self.artifacts = self.directory / 'artifacts'
        self.artifacts.mkdir()
        self.source = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip()
        self.image_id = 'sha256:' + 'a' * 64
        archive = gzip.compress(b'synthetic docker save data')
        (self.artifacts / 'image.tar.gz').write_bytes(archive)
        (self.artifacts / 'image.json').write_text(json.dumps({
            'source_sha': self.source, 'version': 'local-' + self.source[:12], 'image_id': self.image_id,
            'platform': 'linux/amd64', 'archive_sha256': hashlib.sha256(archive).hexdigest()}))
        executable = self.directory / 'docker'
        executable.write_text('''#!/usr/bin/env python3
import json, os, sys, time
from pathlib import Path
args = sys.argv[1:]
with open(os.environ['FAKE_LOG'], 'a') as log:
    log.write(json.dumps(args) + '\\n')
if args[:2] == ['image', 'load']:
    sys.stdin.buffer.read()
if args[:2] == ['image', 'inspect']:
    print(json.dumps([{'Id': os.environ.get('BAD_IMAGE_ID') or os.environ['FAKE_IMAGE_ID'],
      'Config': {'Labels': {'org.opencontainers.image.revision': os.environ['FAKE_SOURCE'],
        'org.opencontainers.image.version': 'local-' + os.environ['FAKE_SOURCE'][:12],
        'io.lunirelust.source.dirty': 'false'}}, 'Os': 'linux', 'Architecture': 'amd64'}]))
if '/app/migration' in args and os.environ.get('FAIL_AT') == 'migration':
    sys.exit(42)
if 'node' in args:
    sys.stdin.read()
    if os.environ.get('FAIL_AT') == 'api': sys.exit(43)
    if os.environ.get('WAIT_NODE'):
        Path(os.environ['WAIT_NODE']).touch()
        time.sleep(60)
''')
        executable.chmod(0o755)
        self.environment = dict(os.environ, PATH=str(self.directory) + ':' + os.environ['PATH'],
                                FAKE_LOG=str(self.directory / 'commands.jsonl'), FAKE_SOURCE=self.source,
                                FAKE_IMAGE_ID=self.image_id, GITHUB_ACTIONS='true', GITHUB_RUN_ID='123',
                                GITHUB_RUN_ATTEMPT='2', GITHUB_JOB='image')

    def command(self):
        return ['bash', str(ROOT / 'scripts/env-run.sh'), '.env.test', 'bash', str(SCRIPT), str(self.artifacts)]

    def log(self):
        path = self.directory / 'commands.jsonl'
        return [json.loads(line) for line in path.read_text().splitlines()] if path.exists() else []

    def test_success_loads_exact_image_migrates_and_cleans_volumes(self):
        result = subprocess.run(self.command(), cwd=ROOT, env=self.environment, capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        commands = self.log()
        self.assertIn(['image', 'load'], commands)
        self.assertTrue(any('/app/migration' in command for command in commands))
        self.assertTrue(any('node' in command for command in commands))
        self.assertTrue(any('down' in command and '--volumes' in command for command in commands))
        self.assertTrue(all('lunirelust-image-123-2-image' in command for command in commands if command[0] == 'compose'))
        self.assertTrue((self.artifacts / 'image-smoke.json').is_file())

    def test_migration_or_api_failure_cleans_without_accepting_image(self):
        for failure in ('migration', 'api'):
            with self.subTest(failure=failure):
                result = subprocess.run(self.command(), cwd=ROOT, env=dict(self.environment, FAIL_AT=failure),
                                        capture_output=True, text=True)
                self.assertNotEqual(result.returncode, 0)
                self.assertTrue(any('down' in command and '--volumes' in command for command in self.log()))
                self.assertFalse((self.artifacts / 'image-smoke.json').exists())

    def test_wrong_loaded_image_is_rejected_before_database_start(self):
        result = subprocess.run(self.command(), cwd=ROOT, env=dict(self.environment, BAD_IMAGE_ID='sha256:wrong'),
                                capture_output=True, text=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse(any('compose' in command for command in self.log()))

    def test_interruption_cleans_isolated_resources(self):
        marker = self.directory / 'waiting'
        process = subprocess.Popen(self.command(), cwd=ROOT, env=dict(self.environment, WAIT_NODE=str(marker)),
                                   stdout=subprocess.PIPE, stderr=subprocess.PIPE, start_new_session=True)
        try:
            deadline = time.monotonic() + 10
            while not marker.exists() and process.poll() is None and time.monotonic() < deadline:
                time.sleep(0.05)
            self.assertTrue(marker.exists(), 'smoke helper did not reach the synthetic API step')
            os.killpg(process.pid, signal.SIGTERM)
            process.communicate(timeout=10)
            self.assertEqual(process.returncode, 143)
            self.assertTrue(any('down' in command and '--volumes' in command for command in self.log()))
            self.assertFalse((self.artifacts / 'image-smoke.json').exists())
        finally:
            if process.poll() is None:
                os.killpg(process.pid, signal.SIGKILL)
                process.communicate()


if __name__ == '__main__':
    unittest.main()
