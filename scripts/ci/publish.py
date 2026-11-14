#!/usr/bin/env python3
"""Promote checked release artifacts. Invoked only by the tag workflow's write job."""
import argparse
import base64
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import urllib.error
import urllib.parse
import urllib.request

from release import TAG, identity, load_image, manifest, require, sha256, write_json


def authorize(environment, data):
    require(environment.get('GITHUB_EVENT_NAME') == 'push'
            and environment.get('GITHUB_REF') == 'refs/tags/' + data['version']
            and TAG.fullmatch(data['version']), 'publication requires an explicit strict release tag push')
    require(data['image']['version'] == data['version'], 'publication requires the release version image label')


def version_tuple(tag):
    require(TAG.fullmatch(tag), 'release version must use strict vX.Y.Z')
    return tuple(map(int, tag[1:].split('.')))


def publish(data, directory, remote):
    """Every external mutation is resumable; never replace a version or an asset."""
    tag, image_id = data['version'], data['image']['image_id']
    marker = '<!-- lunirelust-release ' + json.dumps({
        'source_sha': data['source_sha'], 'version': tag,
        'manifest_sha256': sha256(directory / 'release-manifest.json'),
    }, sort_keys=True) + ' -->'
    existing = remote.get_release(tag)
    if existing:
        require(existing['tag_name'] == tag and existing['target_commitish'] == data['source_sha']
                and (existing.get('body') or '').splitlines()[:1] == [marker],
                'existing release identity differs; refusing to replace it')
    image = remote.get_image(tag)
    if image:
        require(image['image_id'] == image_id, 'existing version image differs; refusing to replace it')
    files = {name: directory / name for name in (*data['binaries'], 'release-manifest.json', 'SHA256SUMS')}
    expected = {name: sha256(path) for name, path in files.items()}
    assets = remote.get_assets(existing['id']) if existing else {}
    require(set(assets) <= set(expected) | {'publication.json'}, 'existing release has unexpected assets')
    for name, checksum in assets.items():
        if name != 'publication.json':
            require(checksum == expected[name], f'existing asset checksum differs: {name}')
    require('publication.json' not in assets or image, 'publication receipt exists but version image is missing')
    if image:
        receipt = {'source_sha': data['source_sha'], 'version': tag, 'image_id': image_id,
                   'image_digest': image['digest']}
        write_json(directory / 'publication.json', receipt)
        if 'publication.json' in assets:
            require(assets['publication.json'] == sha256(directory / 'publication.json'),
                    'existing publication receipt checksum differs')
    existing = existing or remote.create_release(data, marker)
    for name, path in files.items():
        if name not in assets:
            remote.upload(existing['id'], path)
    if image is None:
        remote.push_image(tag, image_id)
        image = remote.get_image(tag)
    require(image and image['image_id'] == image_id, 'published version image does not match tested image')
    write_json(directory / 'publication.json', {
        'source_sha': data['source_sha'], 'version': tag, 'image_id': image_id, 'image_digest': image['digest']})
    expected['publication.json'] = sha256(directory / 'publication.json')
    if 'publication.json' not in assets:
        remote.upload(existing['id'], directory / 'publication.json')
    require(remote.get_assets(existing['id']) == expected, 'published asset checksums are incomplete or differ')
    if existing['draft']:
        remote.publish_release(existing['id'], existing['body'] + '\n\n'
                               + f'Source: `{data["source_sha"]}`\n\nImage digest: `{image["digest"]}`\n\n'
                               + 'See release-manifest.json, publication.json, and SHA256SUMS for artifact identity.')
    # The mutable GitHub latest pointer may lag a successful registry promotion.
    # Every already-public release participates in this monotonic version check.
    latest = remote.highest_published_tag()
    if latest is None or version_tuple(latest) <= version_tuple(tag):
        current = remote.get_image('latest')
        if current is None or current['digest'] != image['digest']:
            remote.push_image('latest', image_id)
        require(remote.get_image('latest')['digest'] == image['digest'], 'latest promotion differs from version digest')
        remote.mark_latest(existing['id'])
    print(f'Published {tag} from {data["source_sha"]}; image digest {image["digest"]}')


class Remote:
    """Small GitHub/registry adapter. Only an explicit HTTP 404 means missing."""
    def __init__(self, repository, token, actor):
        self.repository = repository
        self.api = 'https://api.github.com/repos/' + repository
        self.package = repository.lower() + '/lunirelust'
        self.token = token
        self.actor = actor
        self.headers = {'Authorization': 'Bearer ' + token, 'User-Agent': 'lunirelust-release',
                        'Accept': 'application/vnd.github+json', 'X-GitHub-Api-Version': '2022-11-28'}
        self.json_request(self.api)  # Prove repository access before interpreting a release 404.

    def registry_token(self):
        # Uploads can outlive a registry bearer token; acquire one for each read.
        credentials = base64.b64encode((self.actor + ':' + self.token).encode()).decode()
        url = 'https://ghcr.io/token?' + urllib.parse.urlencode({
            'service': 'ghcr.io', 'scope': 'repository:' + self.package + ':pull'})
        request = urllib.request.Request(url, headers={'Authorization': 'Basic ' + credentials})
        with urllib.request.urlopen(request, timeout=60) as response:
            return json.load(response)['token']

    def json_request(self, url, *, method='GET', data=None, missing=False):
        payload = json.dumps(data).encode() if data is not None else None
        headers = dict(self.headers, **({'Content-Type': 'application/json'} if payload else {}))
        try:
            with urllib.request.urlopen(urllib.request.Request(url, data=payload, headers=headers, method=method),
                                        timeout=120) as response:
                return json.load(response)
        except urllib.error.HTTPError as error:
            if missing and error.code == 404:
                return None
            raise

    def get_release(self, tag):
        # The by-tag endpoint only finds published releases. Authenticated listings
        # also include drafts, whose identity and uploaded assets must survive retries.
        match = None
        page = 1
        while True:
            releases = self.json_request(f'{self.api}/releases?per_page=100&page={page}')
            for item in releases:
                if item['tag_name'] == tag:
                    require(match is None, 'ambiguous releases share the requested tag; inspect them before retrying')
                    match = item
            if len(releases) < 100:
                return match
            page += 1

    def get_image(self, tag):
        index_types = ('application/vnd.oci.image.index.v1+json',
                       'application/vnd.docker.distribution.manifest.list.v2+json')
        headers = {'Authorization': 'Bearer ' + self.registry_token(),
                   'Accept': ', '.join(('application/vnd.docker.distribution.manifest.v2+json',
                                       'application/vnd.oci.image.manifest.v1+json', *index_types))}
        try:
            with urllib.request.urlopen(urllib.request.Request(
                    'https://ghcr.io/v2/' + self.package + '/manifests/' + tag, headers=headers), timeout=60) as response:
                raw = response.read()
                digest = response.headers['Docker-Content-Digest']
        except urllib.error.HTTPError as error:
            if error.code == 404:
                return None
            raise
        require(digest == 'sha256:' + hashlib.sha256(raw).hexdigest(), 'registry manifest digest differs')
        content = json.loads(raw)
        if tag == 'latest' and content.get('mediaType') in index_types and isinstance(content.get('manifests'), list):
            return {'digest': digest}
        require('config' in content, 'existing registry image is not the supported single-platform manifest')
        return {'image_id': content['config']['digest'], 'digest': digest}

    def create_release(self, data, marker):
        return self.json_request(self.api + '/releases', method='POST', data={
            'tag_name': data['version'], 'target_commitish': data['source_sha'], 'name': data['version'],
            'body': marker, 'draft': True, 'prerelease': False, 'make_latest': 'false',
            'generate_release_notes': True})

    def get_assets(self, identifier):
        result = {}
        page = 1
        while True:
            assets = self.json_request(f'{self.api}/releases/{identifier}/assets?per_page=100&page={page}')
            for asset in assets:
                require(asset.get('state') == 'uploaded',
                        'incomplete GitHub asset; inspect and remove the empty starter upload before retrying')
                checksum = asset.get('digest')
                if checksum and checksum.startswith('sha256:'):
                    checksum = checksum.removeprefix('sha256:')
                else:
                    request = urllib.request.Request(asset['url'], headers=dict(
                        self.headers, Accept='application/octet-stream'))
                    with urllib.request.urlopen(request, timeout=120) as response:
                        checksum = hashlib.file_digest(response, 'sha256').hexdigest()
                require(asset['name'] not in result, 'duplicate release asset names')
                result[asset['name']] = checksum
            if len(assets) < 100:
                return result
            page += 1

    def upload(self, identifier, path):
        url = f'https://uploads.github.com/repos/{self.repository}/releases/{identifier}/assets?'
        url += urllib.parse.urlencode({'name': path.name})
        headers = dict(self.headers, **{'Content-Type': 'application/octet-stream',
                                       'Content-Length': str(path.stat().st_size)})
        with path.open('rb') as data:
            with urllib.request.urlopen(urllib.request.Request(url, data=data, headers=headers, method='POST'),
                                        timeout=600) as response:
                json.load(response)

    def push_image(self, tag, image_id):
        reference = 'ghcr.io/' + self.package + ':' + tag
        subprocess.run(['docker', 'image', 'tag', image_id, reference], check=True)
        subprocess.run(['docker', 'image', 'push', reference], check=True)

    def publish_release(self, identifier, body):
        self.json_request(f'{self.api}/releases/{identifier}', method='PATCH',
                          data={'draft': False, 'body': body, 'make_latest': 'false'})

    def highest_published_tag(self):
        highest = None
        page = 1
        while True:
            releases = self.json_request(f'{self.api}/releases?per_page=100&page={page}')
            for item in releases:
                tag = item['tag_name']
                if not item['draft'] and not item['prerelease'] and TAG.fullmatch(tag):
                    if highest is None or version_tuple(tag) > version_tuple(highest):
                        highest = tag
            if len(releases) < 100:
                return highest
            page += 1

    def mark_latest(self, identifier):
        self.json_request(f'{self.api}/releases/{identifier}', method='PATCH', data={'make_latest': 'true'})


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--artifacts', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--default-ref', required=True)
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[2]
    data = manifest(root, args.artifacts, args.output)
    authorize(os.environ, data)
    identity(root, data['version'], args.default_ref)
    load_image(root, args.artifacts)
    remote = Remote(os.environ['GITHUB_REPOSITORY'], os.environ['GH_TOKEN'], os.environ['GITHUB_ACTOR'])
    publish(data, args.output, remote)


if __name__ == '__main__':
    try:
        main()
    except (ValueError, OSError, KeyError, subprocess.CalledProcessError) as error:
        print(f'publication failed: {error}; inspect the draft/version state and rerun this same workflow', file=sys.stderr)
        sys.exit(1)
