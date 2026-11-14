"""Release contract regressions; only synthetic repositories and files are used."""
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile
import unittest

SCRIPT = Path(__file__).resolve().parents[2] / 'scripts/ci/release.py'
TARGETS = ['aarch64-apple-darwin', 'x86_64-apple-darwin',
           'arm-unknown-linux-musleabihf', 'armv7-unknown-linux-musleabihf',
           'x86_64-unknown-linux-musl', 'x86_64-pc-windows-msvc',
           'x86_64-unknown-linux-gnu']
CHECKS = ['migration', 'health', 'authenticated_database', 'postgres_search',
          'file_roundtrip', 'chromium']


def digest(data):
    return hashlib.sha256(data).hexdigest()


class ReleaseTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix='release-contract-')
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        (self.root / 'migration').mkdir()
        (self.root / 'Cargo.toml').write_text('[package]\nname="lunirelust"\nversion="1.2.3"\n')
        (self.root / 'migration/Cargo.toml').write_text('[package]\nname="migration"\nversion="1.2.3"\n')
        (self.root / 'Cargo.lock').write_text(''.join(
            f'[[package]]\nname="{name}"\nversion="1.2.3"\n' for name in ['lunirelust', 'migration']))
        self.git('init', '-q', '-b', 'main')
        self.git('config', 'user.name', 'Synthetic fixture')
        self.git('config', 'user.email', 'fixture@example.invalid')
        self.git('add', '.')
        self.git('commit', '-qm', 'fixture')
        self.sha = self.git('rev-parse', 'HEAD')
        self.git('tag', 'v1.2.3')

    def git(self, *args):
        return subprocess.check_output(['git', '-C', str(self.root), *args], text=True).strip()

    def command(self, *args):
        return subprocess.run(['python3', str(SCRIPT), '--root', str(self.root), *args],
                              text=True, capture_output=True)

    def test_valid_release_binds_tag_source_and_all_package_versions(self):
        result = self.command('validate', '--tag', 'v1.2.3', '--default-ref', 'main')
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(json.loads(result.stdout), {'source_sha': self.sha, 'version': 'v1.2.3'})

    def test_strict_tags_and_mismatched_versions_fail_closed(self):
        for tag in ['v1.2', 'v01.2.3', 'v1.2.3-rc1', '1.2.3', 'v1.2.3\n']:
            with self.subTest(tag=tag):
                self.assertNotEqual(self.command('validate', '--tag', tag, '--default-ref', 'main').returncode, 0)
        (self.root / 'migration/Cargo.toml').write_text('[package]\nname="migration"\nversion="1.2.4"\n')
        result = self.command('validate', '--tag', 'v1.2.3', '--default-ref', 'main')
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('versions', result.stderr)

    def test_lockfile_mismatch_is_rejected(self):
        path = self.root / 'Cargo.lock'
        path.write_text(path.read_text().replace('1.2.3', '1.2.4', 1))
        result = self.command('validate', '--tag', 'v1.2.3', '--default-ref', 'main')
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('lockfile', result.stderr)

    def test_release_source_must_belong_to_default_branch_history(self):
        self.git('checkout', '-qb', 'topic')
        (self.root / 'change').write_text('unmerged')
        self.git('add', 'change')
        self.git('commit', '-qm', 'unmerged fixture')
        self.git('tag', '-f', 'v1.2.3')
        result = self.command('validate', '--tag', 'v1.2.3', '--default-ref', 'main')
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('default branch', result.stderr)

    def artifacts(self):
        directory = self.root / 'artifacts'
        directory.mkdir()
        for target in TARGETS:
            files = {}
            extension = '.exe' if 'windows' in target else ''
            for package in ['lunirelust', 'migration']:
                name = f'{package}-{target}{extension}'
                data = name.encode()
                (directory / name).write_bytes(data)
                files[name] = digest(data)
            (directory / f'binary-{target}.json').write_text(json.dumps({
                'source_sha': self.sha, 'version': 'v1.2.3', 'target': target, 'files': files}))
        archive = b'synthetic image archive'
        (directory / 'image.tar.gz').write_bytes(archive)
        proof = {'source_sha': self.sha, 'version': 'v1.2.3', 'image_id': 'sha256:' + 'a' * 64,
                 'platform': 'linux/amd64', 'archive_sha256': digest(archive)}
        (directory / 'image.json').write_text(json.dumps(proof))
        (directory / 'image-smoke.json').write_text(json.dumps({
            'source_sha': self.sha, 'image_id': proof['image_id'], 'checks': CHECKS}))
        return directory

    def test_manifest_requires_all_seven_targets_and_the_exact_smoked_archive(self):
        directory = self.artifacts()
        result = self.command('manifest', '--artifacts', str(directory), '--output', str(self.root / 'release'))
        self.assertEqual(result.returncode, 0, result.stderr)
        manifest = json.loads((self.root / 'release/release-manifest.json').read_text())
        self.assertEqual(manifest['source_sha'], self.sha)
        self.assertEqual(len(manifest['binaries']), 14)
        self.assertIn('migration-x86_64-pc-windows-msvc.exe', manifest['binaries'])
        self.assertEqual(manifest['image']['image_id'], 'sha256:' + 'a' * 64)

    def test_changed_binary_archive_or_smoke_identity_cannot_be_published(self):
        directory = self.artifacts()
        path = directory / 'lunirelust-x86_64-unknown-linux-gnu'
        original = path.read_bytes()
        path.write_bytes(b'changed')
        result = self.command('manifest', '--artifacts', str(directory), '--output', str(self.root / 'release'))
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('checksum', result.stderr)
        path.write_bytes(original)
        proof = json.loads((directory / 'image-smoke.json').read_text())
        proof['image_id'] = 'sha256:' + 'b' * 64
        (directory / 'image-smoke.json').write_text(json.dumps(proof))
        result = self.command('manifest', '--artifacts', str(directory), '--output', str(self.root / 'release'))
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('smoke', result.stderr)


if __name__ == '__main__':
    unittest.main()
