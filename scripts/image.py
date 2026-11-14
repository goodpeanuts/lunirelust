#!/usr/bin/env python3
"""Build/load or export an image with explicit inputs and no application lifecycle."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile
import tomllib
from urllib.parse import urlsplit

ROOT = Path(__file__).resolve().parent.parent
PRIVATE_PATHS = (".git", ".local", "subm", ".agents", ".codex", ".aws")


def require(condition, message):
    if not condition:
        raise ValueError(message)


def checked_file(path):
    require(path.is_file() and not path.is_symlink(), "required input is missing or is a symlink")
    require(all(not parent.is_symlink() for parent in path.parents), "input parent is a symlink")
    return path


def cache_directory(value, kind):
    candidate = Path(value).absolute()
    require(".." not in candidate.parts and not candidate.is_symlink(), "unsafe cache path")
    path = candidate.resolve(strict=True)
    require(path.is_dir() and path != ROOT and path not in ROOT.parents, "cache must be a dedicated directory")
    for name in PRIVATE_PATHS + ("src", "migration"):
        protected = ROOT / name
        require(path != protected and protected not in path.parents, "cache overlaps protected checkout paths")
    entries = list(path.iterdir())
    require(entries, "cache directory is empty")
    for directory, folders, files in os.walk(path):
        require(all(not (Path(directory) / name).is_symlink() for name in folders + files),
                "cache symlinks are not supported")
    if kind == "vendor":
        require(all(entry.is_dir() and (entry / "Cargo.toml").is_file()
                    and (entry / ".cargo-checksum.json").is_file() for entry in entries),
                "vendor input must contain only cargo-vendored package directories")
    else:
        require(all(entry.is_dir() and (entry.name == ".links" or re.fullmatch(
            r"(?:chromium|chromium_headless_shell|ffmpeg)-[0-9]+", entry.name)) for entry in entries),
            "browser input must be a dedicated Linux Chromium Playwright cache")
        for prefix, executable in (("chromium-", "chrome"),
                                   ("chromium_headless_shell-", "chrome-headless-shell"),
                                   ("ffmpeg-", "ffmpeg-linux")):
            require(any(child.is_file() and os.access(child, os.X_OK)
                        for entry in entries if entry.name.startswith(prefix)
                        for child in entry.rglob(executable)), "browser cache is incomplete or not executable")
    return path


def mirror(value):
    parsed = urlsplit(value)
    require(parsed.scheme in ("http", "https") and parsed.hostname
            and not parsed.username and not parsed.password and not parsed.query and not parsed.fragment
            and re.fullmatch(r"[A-Za-z0-9:/._%\-]+", value),
            "mirror must be a public HTTP(S) URL without credentials, query, or fragment")
    return value.rstrip("/")


def vendor_config(path, destination):
    configuration = tomllib.loads(checked_file(path).read_text())
    # cargo vendor emits only source replacement tables. Never import arbitrary
    # host Cargo configuration, credential providers, or compiler wrappers.
    require(set(configuration) == {"source"}, "vendor config must be the source-only output of cargo vendor")
    sources = configuration["source"]
    require(isinstance(sources, dict) and sources, "vendor config has no source replacements")
    normalized = []
    directories = 0
    for name, values in sources.items():
        require(isinstance(values, dict) and values
                and set(values) <= {"directory", "replace-with", "git", "rev", "tag", "branch", "registry"}
                and all(isinstance(value, str) for value in values.values()), "invalid vendor source table")
        normalized.append("[source." + json.dumps(name) + "]")
        for key, value in values.items():
            if key == "directory":
                value = "/app/vendor"
                directories += 1
            elif key in ("git", "registry"):
                mirror(value.removeprefix("sparse+"))
            normalized.append(key + " = " + json.dumps(value))
    require(directories == 1, "vendor config must have exactly one vendored directory")
    destination.write_text("\n".join(normalized) + "\n")


def source_context(destination):
    names = ["Dockerfile", ".dockerignore", "Cargo.toml", "Cargo.lock", "rust-toolchain",
             ".cargo/config.toml", ".github/scripts/fetch-dependencies.sh", "migration/Cargo.toml"]
    for source in ("src", "migration/src"):
        for directory, folders, files in os.walk(ROOT / source):
            require(all(not (Path(directory) / name).is_symlink() for name in folders),
                    "source directory symlinks are not supported")
            names.extend(str((Path(directory) / name).relative_to(ROOT))
                         for name in files if name.endswith(".rs"))
    require("src/main.rs" in names and "migration/src/main.rs" in names, "application sources are missing")
    for name in names:
        source = checked_file(ROOT / name)
        target = destination / name
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(source, target)


def git(*arguments):
    return subprocess.check_output(["git", "-C", str(ROOT), *arguments],
                                   stderr=subprocess.DEVNULL, text=True).strip()


def labels():
    revision = git("rev-parse", "HEAD")
    dirty = bool(git("status", "--porcelain", "--untracked-files=normal", "--ignore-submodules=none"))
    version = "local-" + revision[:12] + ("-dirty" if dirty else "")
    package = tomllib.loads((ROOT / "Cargo.toml").read_text())["package"]["version"]
    if not dirty:
        tags = git("tag", "--points-at", "HEAD").splitlines()
        if "v" + package in tags:
            version = "v" + package
    return ["org.opencontainers.image.revision=" + revision,
            "org.opencontainers.image.version=" + version,
            "io.lunirelust.source.dirty=" + str(dirty).lower()]


def build(options):
    require(not options.vendor_config or options.vendor, "--vendor-config requires --vendor")
    vendor = cache_directory(options.vendor, "vendor") if options.vendor else None
    browser = cache_directory(options.browser_cache, "browser") if options.browser_cache else None
    mirrors = {name: mirror(value) for name, value in
               (("APT_MIRROR", options.apt_mirror), ("NPM_REGISTRY", options.npm_registry)) if value}
    require(vendor or os.environ.get("GIT_CREDENTIALS"),
            "online builds require exported GIT_CREDENTIALS for a BuildKit secret; alternatively use --vendor")
    with tempfile.TemporaryDirectory(prefix="lunirelust-image-") as temporary:
        staging = Path(temporary)
        context = staging / "source"
        source_context(context)
        command = ["docker", "buildx", "build", "--load", "--provenance=false", "--sbom=false",
                   "--tag", options.image, "--file", str(context / "Dockerfile")]
        for label in labels():
            command.extend(["--label", label])
        for name, value in mirrors.items():
            command.extend(["--build-arg", name + "=" + value])
        if vendor:
            config = staging / "vendor.toml"
            vendor_config(Path(options.vendor_config).absolute() if options.vendor_config
                          else ROOT / ".cargo/config.vendor.toml", config)
            digest = hashlib.sha256(config.read_bytes()).hexdigest()
            command.extend(["--build-context", "vendor-input=" + str(vendor),
                            "--build-arg", "USE_VENDOR=1", "--build-arg", "VENDOR_CONFIG_SHA256=" + digest,
                            "--secret", "id=cargo_vendor_config,src=" + str(config)])
        else:
            command.extend(["--secret", "id=git_credentials,env=GIT_CREDENTIALS"])
        if browser:
            command.extend(["--build-context", "browser-input=" + str(browser),
                            "--build-arg", "USE_BROWSER_CACHE=1"])
        command.append(str(context))
        print("Building and loading the application image (build-only).", flush=True)
        # No cache export, build-record upload, or implicit provenance is enabled.
        environment = dict(os.environ, BUILDX_NO_DEFAULT_ATTESTATIONS="1", BUILDX_GIT_LABELS="0")
        return subprocess.call(command, env=environment)


def export(options):
    destination = Path(options.output).absolute()
    require(not os.path.lexists(destination), "export destination already exists; choose a new archive path")
    resolved = destination.resolve()
    require(not destination.name.startswith(".env")
            and all(resolved != ROOT / name and ROOT / name not in resolved.parents for name in PRIVATE_PATHS),
            "export destination must not overlap environment, private, or runtime storage")
    require(destination.parent.is_dir(), "export destination directory does not exist")
    descriptor, filename = tempfile.mkstemp(prefix=".lunirelust-image-", suffix=".tar", dir=destination.parent)
    os.close(descriptor)
    try:
        status = subprocess.call(["docker", "image", "save", "--output", filename, options.image])
        if status:
            return status
        # Atomic, same-filesystem publication fails if another writer won the name.
        os.link(filename, destination)
        print("Saved application image archive.")
        return 0
    finally:
        os.unlink(filename)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    builder = commands.add_parser("build", help="build and load only")
    builder.add_argument("image")
    builder.add_argument("--vendor", metavar="DIRECTORY")
    builder.add_argument("--vendor-config", metavar="FILE")
    builder.add_argument("--browser-cache", metavar="DIRECTORY")
    builder.add_argument("--apt-mirror", metavar="URL")
    builder.add_argument("--npm-registry", metavar="URL")
    exporter = commands.add_parser("export", help="save an existing local image only")
    exporter.add_argument("image")
    exporter.add_argument("output")
    options = parser.parse_args()
    require(re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9._/:@-]*", options.image), "invalid image reference")
    return build(options) if options.command == "build" else export(options)


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (ValueError, OSError, subprocess.CalledProcessError) as error:
        # File and subprocess errors may contain private input paths or values.
        detail = str(error) if type(error) is ValueError else "input or command failed; check files and required tools"
        print("image command failed: " + detail, file=sys.stderr)
        sys.exit(1)
