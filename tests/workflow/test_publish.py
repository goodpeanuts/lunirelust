"""Publication state-machine tests: no GitHub, registry, or Docker access."""
import copy
import contextlib
import importlib.util
import io
import hashlib
import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest import mock
import urllib.error

SCRIPTS = Path(__file__).resolve().parents[2] / 'scripts/ci'
sys.path.insert(0, str(SCRIPTS))
import release


class FakeRemote:
    def __init__(self):
        self.release = None
        self.images = {}
        self.assets = {}
        self.latest = None
        self.published_tags = set()
        self.fail_mark_latest = False
        self.events = []
        self.fail_latest = False
        self.lookup_error = False
        self.generated_notes = '## Changes\n\n- Synthetic release change.'

    def get_release(self, tag):
        return copy.deepcopy(self.release)

    def get_image(self, tag):
        if self.lookup_error:
            raise RuntimeError('registry authentication failed')
        return self.images.get(tag)

    def create_release(self, manifest, marker):
        self.events.append('create-draft')
        self.release = dict(id=1, tag_name=manifest['version'], target_commitish=manifest['source_sha'],
                            body=marker + '\n\n' + self.generated_notes, draft=True)
        return self.get_release(manifest['version'])

    def get_assets(self, identifier):
        return dict(self.assets)

    def upload(self, identifier, path):
        self.events.append('upload:' + path.name)
        self.assets[path.name] = release.sha256(path)

    def push_image(self, tag, image_id):
        self.events.append('push:' + tag)
        if tag == 'latest' and self.fail_latest:
            raise RuntimeError('registry temporarily unavailable')
        self.images[tag] = {'image_id': image_id, 'digest': 'sha256:' + hashlib.sha256(image_id.encode()).hexdigest()}

    def publish_release(self, identifier, body):
        self.events.append('publish-release')
        self.release.update(draft=False, body=body)
        self.published_tags.add(self.release['tag_name'])

    def highest_published_tag(self):
        return max(self.published_tags, key=lambda tag: tuple(map(int, tag[1:].split('.'))), default=None)

    def latest_tag(self):
        return self.latest

    def mark_latest(self, identifier):
        self.events.append('mark-latest')
        if self.fail_mark_latest:
            raise RuntimeError('GitHub latest update failed')
        self.latest = self.release['tag_name']


class PublicationTests(unittest.TestCase):
    def setUp(self):
        path = SCRIPTS / 'publish.py'
        self.assertTrue(path.exists(), 'publication boundary helper is missing')
        spec = importlib.util.spec_from_file_location('publication', path)
        self.publisher = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(self.publisher)
        self.temporary = tempfile.TemporaryDirectory(prefix='publication-contract-')
        self.addCleanup(self.temporary.cleanup)
        self.directory = Path(self.temporary.name)
        self.manifest = {'schema': 1, 'version': 'v1.2.3', 'source_sha': 'c' * 40,
                         'binaries': {'synthetic-binary': release.sha256(__file__)},
                         'image': {'image_id': 'sha256:' + 'a' * 64, 'version': 'v1.2.3'}}
        (self.directory / 'synthetic-binary').write_bytes(Path(__file__).read_bytes())
        release.write_json(self.directory / 'release-manifest.json', self.manifest)
        (self.directory / 'SHA256SUMS').write_text('synthetic checksums\n')
        self.remote = FakeRemote()

    def publish(self):
        with contextlib.redirect_stdout(io.StringIO()):
            self.publisher.publish(self.manifest, self.directory, self.remote)

    def test_version_and_assets_complete_before_release_and_latest(self):
        self.publish()
        events = self.remote.events
        self.assertLess(events.index('push:v1.2.3'), events.index('publish-release'))
        self.assertLess(events.index('upload:publication.json'), events.index('publish-release'))
        self.assertLess(events.index('publish-release'), events.index('push:latest'))
        self.assertFalse(self.remote.release['draft'])
        self.assertEqual(self.remote.latest, 'v1.2.3')

    def test_identical_rerun_reuses_assets_and_image(self):
        self.publish()
        self.remote.events.clear()
        self.publish()
        self.assertFalse(any(event.startswith(('upload:', 'push:')) for event in self.remote.events))

    def test_existing_version_with_different_image_fails_before_writes(self):
        self.remote.images['v1.2.3'] = {'image_id': 'sha256:other', 'digest': 'sha256:other'}
        with self.assertRaisesRegex(ValueError, 'image'):
            self.publish()
        self.assertEqual(self.remote.events, [])

    def test_existing_release_with_different_identity_or_checksum_fails(self):
        self.publish()
        self.remote.events.clear()
        self.remote.assets['synthetic-binary'] = 'different'
        with self.assertRaisesRegex(ValueError, 'checksum'):
            self.publish()
        self.assertEqual(self.remote.events, [])
        self.remote.release['target_commitish'] = 'd' * 40
        with self.assertRaisesRegex(ValueError, 'identity'):
            self.publish()
        self.assertEqual(self.remote.events, [])

    def test_lookup_errors_are_not_treated_as_absence(self):
        self.remote.lookup_error = True
        with self.assertRaisesRegex(RuntimeError, 'authentication'):
            self.publish()
        self.assertEqual(self.remote.events, [])

    def test_failure_after_release_is_visible_and_resumes_without_overwrite(self):
        self.remote.fail_latest = True
        with self.assertRaisesRegex(RuntimeError, 'unavailable'):
            self.publish()
        self.assertFalse(self.remote.release['draft'])
        self.assertIsNone(self.remote.latest)
        self.remote.fail_latest = False
        self.remote.events.clear()
        self.publish()
        self.assertNotIn('push:v1.2.3', self.remote.events)
        self.assertFalse(any(event.startswith('upload:') for event in self.remote.events))
        self.assertEqual(self.remote.latest, 'v1.2.3')

    def test_old_release_retry_does_not_move_latest_backwards(self):
        self.remote.latest = 'v2.0.0'
        self.remote.published_tags.add('v2.0.0')
        self.publish()
        self.assertNotIn('push:latest', self.remote.events)
        self.assertEqual(self.remote.latest, 'v2.0.0')

    def test_older_retry_preserves_registry_latest_after_newer_github_pointer_failure(self):
        self.publish()
        old_manifest = copy.deepcopy(self.manifest)
        old_release = copy.deepcopy(self.remote.release)
        old_assets = dict(self.remote.assets)
        self.manifest['version'] = 'v2.0.0'
        self.manifest['image'] = {'version': 'v2.0.0', 'image_id': 'sha256:' + 'e' * 64}
        release.write_json(self.directory / 'release-manifest.json', self.manifest)
        self.remote.release = None
        self.remote.assets = {}
        self.remote.fail_mark_latest = True
        with self.assertRaisesRegex(RuntimeError, 'GitHub latest update failed'):
            self.publish()
        self.assertFalse(self.remote.release['draft'])
        new_image = dict(self.remote.images['latest'])
        self.assertEqual(new_image['image_id'], self.manifest['image']['image_id'])
        self.assertEqual(self.remote.latest, old_manifest['version'])
        self.manifest = old_manifest
        release.write_json(self.directory / 'release-manifest.json', self.manifest)
        self.remote.release = old_release
        self.remote.assets = old_assets
        self.remote.fail_mark_latest = False
        self.remote.events.clear()
        self.publish()
        self.assertEqual(self.remote.images['latest'], new_image)
        self.assertNotIn('push:latest', self.remote.events)
        self.assertNotIn('mark-latest', self.remote.events)

    def test_highest_release_scans_every_page_and_excludes_drafts_prereleases_and_invalid_tags(self):
        remote = self.publisher.Remote.__new__(self.publisher.Remote)
        remote.api = 'https://api.github.com/repos/synthetic/repository'
        self.assertTrue(hasattr(remote, 'highest_published_tag'), 'published-version lookup is missing')
        releases = [dict(tag_name='v1.2.3', draft=False, prerelease=False)] * 100
        releases += [dict(tag_name='v2.0.0', draft=False, prerelease=False),
                     dict(tag_name='v10.0.0', draft=True, prerelease=False),
                     dict(tag_name='v20.0.0', draft=False, prerelease=True),
                     dict(tag_name='v100.0.0-rc1', draft=False, prerelease=False)]
        with mock.patch.object(remote, 'json_request', side_effect=[releases[:100], releases[100:]]) as request:
            self.assertEqual(remote.highest_published_tag(), 'v2.0.0')
        self.assertEqual(request.call_count, 2)
        self.assertTrue(request.call_args_list[0].args[0].endswith('per_page=100&page=1'))
        self.assertTrue(request.call_args_list[1].args[0].endswith('per_page=100&page=2'))

    def test_generated_and_edited_draft_notes_survive_publication_and_retry(self):
        with mock.patch.object(self.remote, 'upload', side_effect=RuntimeError('upload interrupted')):
            with self.assertRaisesRegex(RuntimeError, 'interrupted'):
                self.publish()
        self.remote.release['body'] += '\n\nMaintainer note: synthetic recovery detail.'
        draft_body = self.remote.release['body']
        self.remote.fail_mark_latest = True
        with self.assertRaisesRegex(RuntimeError, 'GitHub latest update failed'):
            self.publish()
        published_body = self.remote.release['body']
        self.assertTrue(published_body.startswith(draft_body + '\n\n'))
        self.assertEqual(published_body.count(self.remote.generated_notes), 1)
        self.assertIn('Maintainer note: synthetic recovery detail.', published_body)
        self.assertTrue(published_body.splitlines()[0].startswith('<!-- lunirelust-release '))
        self.remote.fail_mark_latest = False
        self.publish()
        self.assertEqual(self.remote.release['body'], published_body)

    def test_new_draft_requests_github_generated_notes_after_identity_marker(self):
        remote = self.publisher.Remote.__new__(self.publisher.Remote)
        remote.api = 'https://api.github.com/repos/synthetic/repository'
        marker = '<!-- synthetic identity -->'
        with mock.patch.object(remote, 'json_request', return_value={'body': marker + '\n\nGenerated notes'}) as request:
            created = remote.create_release(self.manifest, marker)
        payload = request.call_args.kwargs['data']
        self.assertTrue(payload.get('generate_release_notes'))
        self.assertTrue(payload['draft'])
        self.assertEqual(payload['body'], marker)
        self.assertTrue(created['body'].startswith(marker + '\n'))

    def test_partial_draft_upload_resumes_without_replacing_completed_assets(self):
        original = self.remote.upload
        uploads = 0

        def interrupted_upload(identifier, path):
            nonlocal uploads
            uploads += 1
            if uploads == 2:
                raise RuntimeError('upload interrupted')
            original(identifier, path)

        self.remote.upload = interrupted_upload
        with self.assertRaisesRegex(RuntimeError, 'interrupted'):
            self.publish()
        self.assertTrue(self.remote.release['draft'])
        self.assertNotIn('v1.2.3', self.remote.images)
        first_asset = next(iter(self.remote.assets))
        self.remote.upload = original
        self.remote.events.clear()
        self.publish()
        self.assertNotIn('upload:' + first_asset, self.remote.events)
        self.assertFalse(self.remote.release['draft'])

    def test_release_lookup_finds_paginated_draft_despite_tag_endpoint_404(self):
        remote = self.publisher.Remote.__new__(self.publisher.Remote)
        remote.api = 'https://api.github.com/repos/synthetic/repository'
        remote.headers = {}
        tag = self.manifest['version']
        draft = {'id': 77, 'tag_name': tag, 'draft': True, 'body': 'retained draft notes',
                 'target_commitish': self.manifest['source_sha'], 'assets': [{'name': 'retained-binary'}]}
        pages = [[{'id': index, 'tag_name': 'v0.0.1', 'draft': False} for index in range(100)], [draft]]
        requests = []

        def response(request, **kwargs):
            requests.append(request.full_url)
            if '/releases/tags/' in request.full_url:
                raise urllib.error.HTTPError(request.full_url, 404, 'published release missing', {}, None)
            for page, data in enumerate(pages, 1):
                if request.full_url == f'{remote.api}/releases?per_page=100&page={page}':
                    return io.BytesIO(json.dumps(data).encode())
            self.fail('Unexpected GitHub request: ' + request.full_url)

        with mock.patch('urllib.request.urlopen', side_effect=response):
            self.assertIsNone(remote.json_request(remote.api + '/releases/tags/' + tag, missing=True))
            self.assertEqual(remote.get_release(tag), draft)
        self.assertIn(remote.api + '/releases?per_page=100&page=2', requests)

    def test_release_lookup_rejects_ambiguous_same_tag_matches_across_pages(self):
        remote = self.publisher.Remote.__new__(self.publisher.Remote)
        remote.api = 'https://api.github.com/repos/synthetic/repository'
        tag = self.manifest['version']
        first = [{'id': 1, 'tag_name': tag, 'draft': False}]
        first += [{'id': index, 'tag_name': 'v0.0.1'} for index in range(2, 101)]
        second = [{'id': 101, 'tag_name': tag, 'draft': True}]
        with mock.patch.object(remote, 'json_request', side_effect=[first, second]):
            with self.assertRaisesRegex(ValueError, 'ambiguous'):
                remote.get_release(tag)

    def test_release_lookup_only_reports_absence_after_every_listing_page(self):
        remote = self.publisher.Remote.__new__(self.publisher.Remote)
        remote.api = 'https://api.github.com/repos/synthetic/repository'
        first = [{'id': index, 'tag_name': 'v0.0.1'} for index in range(100)]
        with mock.patch.object(remote, 'json_request', side_effect=[first, []]) as request:
            self.assertIsNone(remote.get_release('v1.2.3'))
        self.assertEqual(request.call_count, 2)

    def test_release_listing_authentication_network_and_404_errors_propagate(self):
        remote = self.publisher.Remote.__new__(self.publisher.Remote)
        remote.api = 'https://api.github.com/repos/synthetic/repository'
        remote.headers = {}
        errors = [urllib.error.HTTPError(remote.api, code, 'synthetic error', {}, None)
                  for code in (401, 403, 404, 500)]
        errors.append(urllib.error.URLError('synthetic network error'))
        for error in errors:
            with self.subTest(error=error):
                with mock.patch('urllib.request.urlopen', side_effect=error):
                    with self.assertRaises(type(error)):
                        remote.get_release('v1.2.3')

    def test_http_adapters_only_treat_explicit_404_as_missing(self):
        remote = self.publisher.Remote.__new__(self.publisher.Remote)
        remote.package = 'synthetic/repository/lunirelust'
        remote.registry_token = lambda: 'synthetic-token'
        remote.headers = {}
        for code in (401, 403, 404, 500):
            error = urllib.error.HTTPError('https://example.invalid', code, 'synthetic error', {}, None)
            with mock.patch('urllib.request.urlopen', side_effect=error):
                if code == 404:
                    self.assertIsNone(remote.get_image('v1.2.3'))
                    self.assertIsNone(remote.json_request('https://example.invalid', missing=True))
                else:
                    with self.assertRaises(urllib.error.HTTPError):
                        remote.get_image('v1.2.3')
                    with self.assertRaises(urllib.error.HTTPError):
                        remote.json_request('https://example.invalid', missing=True)

    def test_legacy_latest_index_is_readable_without_relaxing_version_identity(self):
        remote = self.publisher.Remote.__new__(self.publisher.Remote)
        remote.package = 'synthetic/repository/lunirelust'
        remote.registry_token = lambda: 'synthetic-token'
        index_type = 'application/vnd.oci.image.index.v1+json'
        raw = json.dumps({'schemaVersion': 2, 'mediaType': index_type, 'manifests': [
            {'digest': 'sha256:' + '1' * 64}, {'digest': 'sha256:' + '2' * 64}]}).encode()
        digest = 'sha256:' + hashlib.sha256(raw).hexdigest()
        requests = []

        def response(request, **kwargs):
            requests.append(request)
            stream = io.BytesIO(raw)
            stream.headers = {'Docker-Content-Digest': digest}
            return stream

        with mock.patch('urllib.request.urlopen', side_effect=response):
            self.assertEqual(remote.get_image('latest'), {'digest': digest})
            with self.assertRaisesRegex(ValueError, 'single-platform'):
                remote.get_image('v1.2.3')
        self.assertIn(index_type, requests[0].headers['Accept'])

    def test_registry_reads_refresh_auth_after_slow_asset_uploads(self):
        remote = self.publisher.Remote.__new__(self.publisher.Remote)
        remote.package = 'synthetic/repository/lunirelust'
        remote.token = 'synthetic-token'
        remote.actor = 'synthetic-actor'
        tokens = 0
        manifest = json.dumps({'config': {'digest': self.manifest['image']['image_id']}}).encode()

        def response(request, **kwargs):
            nonlocal tokens
            if '/token?' in request.full_url:
                tokens += 1
                return io.BytesIO(json.dumps({'token': f'fresh-{tokens}'}).encode())
            self.assertEqual(request.headers['Authorization'], f'Bearer fresh-{tokens}')
            result = io.BytesIO(manifest)
            result.headers = {'Docker-Content-Digest': 'sha256:' + hashlib.sha256(manifest).hexdigest()}
            return result

        with mock.patch('urllib.request.urlopen', side_effect=response):
            remote.get_image('v1.2.3')
            remote.get_image('v1.2.3')
        self.assertEqual(tokens, 2)

    def test_manual_context_is_never_authorized_to_publish(self):
        for event, ref in [('workflow_dispatch', 'refs/tags/v1.2.3'), ('push', 'refs/heads/main')]:
            with self.assertRaisesRegex(ValueError, 'tag push'):
                self.publisher.authorize({'GITHUB_EVENT_NAME': event, 'GITHUB_REF': ref}, self.manifest)


if __name__ == '__main__':
    unittest.main()
