#!/usr/bin/env python3
"""Validate release identity and assemble checked artifacts; never publish."""
import argparse
import gzip
import hashlib
import json
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tomllib

TARGETS = (
    'aarch64-apple-darwin', 'x86_64-apple-darwin',
    'arm-unknown-linux-musleabihf', 'armv7-unknown-linux-musleabihf',
    'x86_64-unknown-linux-musl', 'x86_64-pc-windows-msvc', 'x86_64-unknown-linux-gnu',
)
SMOKE_CHECKS = ('migration', 'health', 'authenticated_database', 'postgres_search',
                'file_roundtrip', 'chromium')
TAG = re.compile(r'v(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)')


def require(condition, message):
    if not condition:
        raise ValueError(message)


def sha256(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def read_json(path):
    return json.loads(Path(path).read_text())


def write_json(path, value):
    Path(path).write_text(json.dumps(value, sort_keys=True, indent=2) + '\n')


def git(root, *args):
    return subprocess.check_output(['git', '-C', str(root), *args], text=True).strip()


def identity(root, tag=None, default_ref=None):
    app = tomllib.loads((root / 'Cargo.toml').read_text())['package']['version']
    migration = tomllib.loads((root / 'migration/Cargo.toml').read_text())['package']['version']
    require(app == migration, 'application and migration versions differ')
    version = 'v' + app
    require(TAG.fullmatch(version), 'package versions must use X.Y.Z')
    locked = tomllib.loads((root / 'Cargo.lock').read_text())['package']
    for package in ('lunirelust', 'migration'):
        versions = [p['version'] for p in locked if p['name'] == package and 'source' not in p]
        require(versions == [app], f'{package} lockfile version differs from package versions')
    source = git(root, 'rev-parse', 'HEAD')
    if tag is not None:
        require(TAG.fullmatch(tag), 'release tag must match strict vX.Y.Z')
        require(tag == version, 'release tag and package versions differ')
        require(git(root, 'rev-parse', f'refs/tags/{tag}^{{commit}}') == source,
                'release tag does not identify the checked out source')
        require(default_ref, 'a fetched default branch ref is required')
        result = subprocess.run(['git', '-C', str(root), 'merge-base', '--is-ancestor', source, default_ref])
        require(result.returncode == 0, 'release source must belong to default branch history')
    return {'source_sha': source, 'version': version}


def binary_names(target):
    require(target in TARGETS, 'unsupported binary target')
    extension = '.exe' if target == 'x86_64-pc-windows-msvc' else ''
    return {name: f'{name}-{target}{extension}' for name in ('lunirelust', 'migration')}


def binary(root, target, directory, create=False):
    expected = identity(root)
    names = binary_names(target)
    path = directory / f'binary-{target}.json'
    if create:
        directory.mkdir(parents=True, exist_ok=True)
        extension = '.exe' if target == 'x86_64-pc-windows-msvc' else ''
        for package, name in names.items():
            shutil.copyfile(root / 'target' / target / 'release' / (package + extension), directory / name)
        write_json(path, dict(expected, target=target,
                              files={name: sha256(directory / name) for name in names.values()}))
    proof = read_json(path)
    require(all(proof.get(key) == value for key, value in expected.items()) and proof.get('target') == target,
            'binary source/version/target identity differs')
    require(set(proof['files']) == set(names.values()), 'binary artifact has unexpected files')
    for name, checksum in proof['files'].items():
        require(sha256(directory / name) == checksum, f'binary checksum differs: {name}')
    return proof['files']


def inspect_image(image):
    return json.loads(subprocess.check_output(['docker', 'image', 'inspect', image], text=True))[0]


def check_loaded(root, proof):
    data = inspect_image(proof['image_id'])
    labels = data['Config']['Labels']
    require(data['Id'] == proof['image_id'], 'loaded image ID differs from artifact proof')
    require(labels['org.opencontainers.image.revision'] == proof['source_sha'], 'loaded image source differs')
    require(labels['org.opencontainers.image.version'] == proof['version'], 'loaded image version differs')
    require(labels['io.lunirelust.source.dirty'] == 'false', 'image source checkout was dirty')
    require(data['Os'] + '/' + data['Architecture'] == 'linux/amd64', 'only linux/amd64 images are supported')


def image_proof(root, directory, image):
    data = inspect_image(image)
    labels = data['Config']['Labels']
    proof = {'source_sha': labels['org.opencontainers.image.revision'],
             'version': labels['org.opencontainers.image.version'], 'image_id': data['Id'],
             'platform': data['Os'] + '/' + data['Architecture'],
             'archive_sha256': sha256(directory / 'image.tar.gz')}
    write_json(directory / 'image.json', proof)
    verify_image(root, directory)
    check_loaded(root, proof)


def verify_image(root, directory, smoked=False):
    expected = identity(root)
    proof = read_json(directory / 'image.json')
    require(proof['source_sha'] == expected['source_sha'], 'image source differs from checkout')
    require(proof['version'] in (expected['version'], 'local-' + expected['source_sha'][:12]),
            'image version differs from checkout')
    require(re.fullmatch(r'sha256:[0-9a-f]{64}', proof['image_id']), 'invalid image ID')
    require(proof['platform'] == 'linux/amd64', 'only linux/amd64 images are supported')
    require(sha256(directory / 'image.tar.gz') == proof['archive_sha256'], 'image archive checksum differs')
    if smoked:
        smoke = read_json(directory / 'image-smoke.json')
        require(smoke['source_sha'] == proof['source_sha'] and smoke['image_id'] == proof['image_id']
                and sorted(smoke['checks']) == sorted(SMOKE_CHECKS), 'image smoke proof differs or is incomplete')
    return proof


def load_image(root, directory):
    proof = verify_image(root, directory)
    process = subprocess.Popen(['docker', 'image', 'load'], stdin=subprocess.PIPE)
    try:
        with gzip.open(directory / 'image.tar.gz', 'rb') as archive:
            shutil.copyfileobj(archive, process.stdin)
    finally:
        process.stdin.close()
    require(process.wait() == 0, 'loading the tested image archive failed')
    check_loaded(root, proof)
    return proof


def manifest(root, directory, output):
    files = {}
    for target in TARGETS:
        files.update(binary(root, target, directory))
    image = verify_image(root, directory, smoked=True)
    data = dict(identity(root), schema=1, binaries=files, image=image)
    output.mkdir(parents=True, exist_ok=True)
    for name in files:
        shutil.copyfile(directory / name, output / name)
    write_json(output / 'release-manifest.json', data)
    checksums = dict(files, **{'release-manifest.json': sha256(output / 'release-manifest.json')})
    (output / 'SHA256SUMS').write_text(''.join(f'{checksum}  {name}\n' for name, checksum in sorted(checksums.items())))
    return data


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, default=Path(__file__).resolve().parents[2])
    commands = parser.add_subparsers(dest='command', required=True)
    validate = commands.add_parser('validate')
    validate.add_argument('--tag')
    validate.add_argument('--default-ref')
    for action in ('binary', 'verify-binary'):
        command = commands.add_parser(action)
        command.add_argument('--target', choices=TARGETS, required=True)
        command.add_argument('--directory', type=Path, required=True)
    for action in ('image-proof', 'verify-image', 'load-image', 'smoke-proof'):
        command = commands.add_parser(action)
        command.add_argument('--directory', type=Path, required=True)
        if action == 'image-proof':
            command.add_argument('--image', required=True)
        if action == 'verify-image':
            command.add_argument('--smoked', action='store_true')
    assembly = commands.add_parser('manifest')
    assembly.add_argument('--artifacts', type=Path, required=True)
    assembly.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    if args.command == 'validate':
        print(json.dumps(identity(args.root, args.tag, args.default_ref)))
    elif args.command in ('binary', 'verify-binary'):
        binary(args.root, args.target, args.directory, args.command == 'binary')
    elif args.command == 'image-proof':
        image_proof(args.root, args.directory, args.image)
    elif args.command == 'verify-image':
        verify_image(args.root, args.directory, args.smoked)
    elif args.command == 'load-image':
        load_image(args.root, args.directory)
    elif args.command == 'smoke-proof':
        proof = verify_image(args.root, args.directory)
        check_loaded(args.root, proof)
        write_json(args.directory / 'image-smoke.json', {
            'source_sha': proof['source_sha'], 'image_id': proof['image_id'], 'checks': SMOKE_CHECKS})
    else:
        manifest(args.root, args.artifacts, args.output)


if __name__ == '__main__':
    try:
        main()
    except (ValueError, OSError, KeyError, subprocess.CalledProcessError) as error:
        print(f'release validation failed: {error}', file=sys.stderr)
        sys.exit(1)
