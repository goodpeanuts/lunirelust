#!/usr/bin/env bash
# Exercise image commands without contacting Docker or reading local application data.
set -euo pipefail
source_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)"
python3 - "$source_root" <<'PY'
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import tomllib
import unittest

SOURCE = Path(sys.argv.pop())


class ImageCommands(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="lunirelust-image-test-")
        self.addCleanup(self.temporary.cleanup)
        self.scratch = Path(self.temporary.name)
        self.root = self.scratch / "checkout with spaces"
        self.root.mkdir()
        shutil.copytree(SOURCE / "scripts", self.root / "scripts")
        for name in ("Dockerfile", ".dockerignore", "justfile", "Cargo.toml", "Cargo.lock", "rust-toolchain", ".gitignore"):
            shutil.copy2(SOURCE / name, self.root / name)
        for name in ("src", "migration/src", ".cargo", ".github/scripts"):
            (self.root / name).mkdir(parents=True, exist_ok=True)
        (self.root / "src/main.rs").write_text("fn main() {}\n")
        (self.root / "migration/src/main.rs").write_text("fn main() {}\n")
        shutil.copy2(SOURCE / "migration/Cargo.toml", self.root / "migration/Cargo.toml")
        shutil.copy2(SOURCE / ".cargo/config.toml", self.root / ".cargo/config.toml")
        fetch = SOURCE / ".github/scripts/fetch-dependencies.sh"
        # Delivery B supplies the shared fetch helper before image integration.
        (self.root / ".github/scripts/fetch-dependencies.sh").write_text(
            fetch.read_text() if fetch.exists() else "#!/usr/bin/env bash\nexit 0\n")
        self.bin = self.scratch / "bin"
        self.bin.mkdir()
        self.events = self.scratch / "docker.jsonl"
        docker = self.bin / "docker"
        docker.write_text('''#!/usr/bin/env python3
import json, os, pathlib, sys
args = sys.argv[1:]
event = {"args": args, "cwd": os.getcwd()}
if args[:2] == ["buildx", "build"]:
    root = pathlib.Path(args[-1])
    event["context"] = sorted(str(p.relative_to(root)) for p in root.rglob("*") if p.is_file())
    event["named"] = {}
    for index, arg in enumerate(args):
        if arg == "--build-context":
            name, path = args[index + 1].split("=", 1)
            event["named"][name] = str(pathlib.Path(path).resolve())
elif args[:2] == ["image", "save"]:
    pathlib.Path(args[args.index("--output") + 1]).write_bytes(b"image archive\\n")
else:
    sys.exit("unexpected Docker operation")
with open(os.environ["IMAGE_TEST_EVENTS"], "a") as stream:
    stream.write(json.dumps(event) + "\\n")
sys.exit(int(os.environ.get("IMAGE_TEST_FAIL", "0")))
''')
        docker.chmod(0o755)
        cargo = self.bin / "cargo"
        cargo.write_text("#!/usr/bin/env bash\necho 'unexpected Cargo operation' >&2\nexit 89\n")
        cargo.chmod(0o755)
        self.env = os.environ.copy()
        for name in list(self.env):
            if name.startswith(("GIT_", "COMPOSE_", "BUILDX_", "DOCKER_BUILD_")):
                self.env.pop(name)
        self.env.update(PATH=str(self.bin) + os.pathsep + self.env["PATH"],
                        IMAGE_TEST_EVENTS=str(self.events), GIT_CONFIG_GLOBAL=os.devnull,
                        GIT_CONFIG_NOSYSTEM="1", GIT_CREDENTIALS="synthetic-image-token-never-log")
        self.git("init", "-q")
        self.git("add", ".")
        self.git("-c", "user.name=Image Test", "-c", "user.email=image-test@example.invalid",
                 "commit", "-qm", "fixture")
        self.release_tag = "v" + tomllib.loads((self.root / "Cargo.toml").read_text())["package"]["version"]
        self.git("tag", self.release_tag)

    def git(self, *args):
        return subprocess.check_output(["git", *args], cwd=self.root, env=self.env, text=True).strip()

    def run_image(self, *args, ok=True, env=None):
        result = subprocess.run(["bash", str(self.root / "scripts/image.sh"), *args],
                                cwd=self.root, env=env or self.env, text=True,
                                stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        self.assertNotIn(self.env["GIT_CREDENTIALS"], result.stdout)
        if ok:
            self.assertEqual(result.returncode, 0, result.stdout)
        else:
            self.assertNotEqual(result.returncode, 0, result.stdout)
        return result

    def recorded(self):
        return [json.loads(line) for line in self.events.read_text().splitlines()] if self.events.exists() else []

    def option(self, args, flag):
        return [args[index + 1] for index, value in enumerate(args) if value == flag]

    def vendor(self):
        directory = self.scratch / "cached dependencies"
        (directory / "example").mkdir(parents=True)
        (directory / "example/.cargo-checksum.json").write_text('{}')
        (directory / "example/Cargo.toml").write_text('[package]\nname = "example"\nversion = "1.0.0"\n')
        config = self.scratch / "vendor config.toml"
        config.write_text('[source.crates-io]\nreplace-with = "vendored-sources"\n'
                          '[source.vendored-sources]\ndirectory = "some host directory"\n')
        return directory, config

    def browsers(self):
        directory = self.scratch / "browser cache"
        for executable in ("chromium-1217/chrome-linux64/chrome",
                           "chromium_headless_shell-1217/chrome-headless-shell-linux64/chrome-headless-shell",
                           "ffmpeg-1011/ffmpeg-linux"):
            target = directory / executable
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_text("browser fixture\n")
            target.chmod(0o755)
        return directory

    def test_build_only_and_secret_transport(self):
        result = self.run_image("build", "lunirelust:review")
        event, = self.recorded()
        args = event["args"]
        self.assertEqual(args[:2], ["buildx", "build"])
        self.assertIn("--load", args)
        self.assertIn("--provenance=false", args)
        self.assertIn("--sbom=false", args)
        self.assertIn("id=git_credentials,env=GIT_CREDENTIALS", self.option(args, "--secret"))
        self.assertFalse(any(flag in args for flag in ("--push", "--cache-to", "--metadata-file")))
        self.assertFalse(any("CREDENTIALS" in value for value in self.option(args, "--build-arg")))
        self.assertNotIn(self.env["GIT_CREDENTIALS"], self.events.read_text())
        self.assertIn("build", result.stdout.lower())
        self.assertFalse((self.root / ".env").exists())
        self.assertFalse((self.root / ".local").exists())

    def test_context_excludes_custom_runtime_and_private_inputs(self):
        for name in (".env", "subm/private.txt", ".local/data", "custom/user-images/secret.jpg",
                     "src/custom-runtime/secret.jpg", ".cargo/credentials.toml", ".cargo/config.vendor.toml",
                     "vendor/private-source.rs", ".playwright-cache/browser"):
            path = self.root / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text("must never enter the source context\n")
        self.run_image("build", "lunirelust:review")
        event, = self.recorded()
        self.assertEqual(set(event["context"]), {
            "Dockerfile", ".dockerignore", "Cargo.toml", "Cargo.lock", "rust-toolchain",
            ".cargo/config.toml", ".github/scripts/fetch-dependencies.sh",
            "src/main.rs", "migration/Cargo.toml", "migration/src/main.rs"})
        self.assertFalse(Path(event["args"][-1]).exists(), "temporary source context was retained")
        self.assertEqual((self.root / ".env").read_text(), "must never enter the source context\n")

    def test_clean_release_and_dirty_source_labels(self):
        revision = self.git("rev-parse", "HEAD")
        self.run_image("build", "lunirelust:review")
        labels = self.option(self.recorded()[-1]["args"], "--label")
        self.assertIn("org.opencontainers.image.revision=" + revision, labels)
        self.assertIn("org.opencontainers.image.version=" + self.release_tag, labels)
        (self.root / "src/main.rs").write_text("fn main() { /* local change */ }\n")
        self.run_image("build", "lunirelust:review")
        labels = self.option(self.recorded()[-1]["args"], "--label")
        self.assertIn("org.opencontainers.image.version=local-" + revision[:12] + "-dirty", labels)
        self.assertIn("io.lunirelust.source.dirty=true", labels)

    def test_clean_untagged_source_is_local(self):
        self.git("tag", "-d", self.release_tag)
        self.run_image("build", "lunirelust:review")
        labels = self.option(self.recorded()[-1]["args"], "--label")
        self.assertIn("org.opencontainers.image.version=local-" + self.git("rev-parse", "HEAD")[:12], labels)

    def test_explicit_vendor_browser_contexts_and_mirrors(self):
        vendor, config = self.vendor()
        browser = self.browsers()
        environment = dict(self.env)
        environment.pop("GIT_CREDENTIALS")
        self.run_image("build", "lunirelust:cached", "--vendor", str(vendor),
                       "--vendor-config", str(config), "--browser-cache", str(browser),
                       "--apt-mirror", "https://mirror.example.invalid",
                       "--npm-registry", "https://npm.example.invalid", env=environment)
        event, = self.recorded()
        self.assertEqual(event["named"], {"vendor-input": str(vendor), "browser-input": str(browser)})
        self.assertIn("USE_VENDOR=1", self.option(event["args"], "--build-arg"))
        self.assertIn("USE_BROWSER_CACHE=1", self.option(event["args"], "--build-arg"))
        self.assertIn("APT_MIRROR=https://mirror.example.invalid", self.option(event["args"], "--build-arg"))
        self.assertIn("NPM_REGISTRY=https://npm.example.invalid", self.option(event["args"], "--build-arg"))
        self.assertFalse(any("git_credentials" in value for value in self.option(event["args"], "--secret")))
        self.assertEqual(config.read_text().count("some host directory"), 1, "vendor config changed")

    def test_normalized_vendor_mapping_controls_build_cache_key(self):
        vendor, config = self.vendor()

        def cache_key():
            self.run_image("build", "lunirelust:cached", "--vendor", str(vendor),
                           "--vendor-config", str(config))
            arguments = self.option(self.recorded()[-1]["args"], "--build-arg")
            digests = [value.partition("=")[2] for value in arguments
                       if value.startswith("VENDOR_CONFIG_SHA256=")]
            self.assertEqual(len(digests), 1, "normalized vendor config has no build cache key")
            self.assertRegex(digests[0], r"^[0-9a-f]{64}$")
            return digests[0]

        original = cache_key()
        self.assertEqual(cache_key(), original, "unchanged input changed the build cache key")
        config.write_text(config.read_text().replace("some host directory", "another host directory"))
        self.assertEqual(cache_key(), original, "normalized-away host paths changed the build cache key")
        config.write_text(config.read_text().replace("vendored-sources", "cached-sources"))
        self.assertNotEqual(cache_key(), original, "changed source mapping reused the build cache key")
        dockerfile = (SOURCE / "Dockerfile").read_text()
        self.assertIn("ARG VENDOR_CONFIG_SHA256", dockerfile)
        self.assertIn('"$VENDOR_CONFIG_SHA256"', dockerfile)
        self.assertIn("sha256sum --check --status", dockerfile)

    def test_unsafe_or_missing_inputs_fail_before_docker(self):
        vendor, config = self.vendor()
        browser = self.browsers()
        for extra in (("--vendor", "/does/not/exist"), ("--vendor", str(self.root)),
                      ("--vendor-config", str(config)), ("--browser-cache", str(self.root)),
                      ("--browser-cache", str(vendor)), ("--browser-cache", "../missing"),
                      ("--apt-mirror", "https://user:secret@example.invalid"),
                      ("--npm-registry", "https://example.invalid/?token=secret"),
                      ("--cache-to", "type=registry,mode=max"), ("--vendor",)):
            with self.subTest(extra=extra):
                self.run_image("build", "lunirelust:review", *extra, ok=False)
                self.assertEqual(self.recorded(), [])
        (browser / "escaped").symlink_to(self.root / ".env")
        self.run_image("build", "lunirelust:review", "--browser-cache", str(browser), ok=False)
        self.assertEqual(self.recorded(), [])
        (vendor / "example/escaped").symlink_to(self.root / ".env")
        self.run_image("build", "lunirelust:review", "--vendor", str(vendor),
                       "--vendor-config", str(config), ok=False)
        self.assertEqual(self.recorded(), [])

    def test_online_build_requires_explicit_environment_secret(self):
        environment = dict(self.env)
        environment.pop("GIT_CREDENTIALS")
        (self.root / ".env").write_text("GIT_CREDENTIALS=dotenv-must-not-be-read\n")
        self.run_image("build", "lunirelust:review", ok=False, env=environment)
        self.assertEqual(self.recorded(), [])

    def test_source_symlink_is_rejected_before_docker(self):
        (self.root / "src/main.rs").unlink()
        (self.root / "src/main.rs").symlink_to(self.scratch / "unrelated-secret")
        self.run_image("build", "lunirelust:review", ok=False)
        self.assertEqual(self.recorded(), [])

    def test_export_saves_only_and_never_overwrites(self):
        archive = self.scratch / "-archive with spaces.tar"
        self.run_image("export", "lunirelust:review", str(archive))
        event, = self.recorded()
        self.assertEqual(event["args"][:2], ["image", "save"])
        self.assertEqual(event["args"][-1], "lunirelust:review")
        self.assertEqual(archive.read_bytes(), b"image archive\n")
        self.run_image("export", "lunirelust:review", str(archive), ok=False)
        self.assertEqual(len(self.recorded()), 1)
        self.assertEqual(archive.read_bytes(), b"image archive\n")
        self.assertFalse((self.root / ".env").exists())
        self.assertFalse((self.root / ".local").exists())

    def test_export_failure_and_dangling_destination_leave_no_archive(self):
        archive = self.scratch / "failed.tar"
        environment = dict(self.env, IMAGE_TEST_FAIL="37")
        result = self.run_image("export", "lunirelust:review", str(archive), ok=False, env=environment)
        self.assertEqual(result.returncode, 37)
        self.assertFalse(archive.exists())
        archive.symlink_to(self.scratch / "absent")
        self.run_image("export", "lunirelust:review", str(archive), ok=False)
        self.assertEqual(len(self.recorded()), 1)
        self.assertTrue(archive.is_symlink())

    def test_export_rejects_environment_and_runtime_storage_destinations(self):
        for directory in (".local", "subm"):
            (self.root / directory).mkdir()
        for name in (".env", ".env.backup", ".local/archive.tar", "subm/archive.tar", ".git/archive.tar"):
            self.run_image("export", "lunirelust:review", str(self.root / name), ok=False)
            self.assertFalse((self.root / name).exists())
        self.assertEqual(self.recorded(), [])

    def test_invalid_image_does_not_become_a_docker_option(self):
        for name in ("--push", "bad image", "name;touch unsafe"):
            self.run_image("build", name, ok=False)
            self.run_image("export", name, str(self.scratch / "unused.tar"), ok=False)
        self.assertEqual(self.recorded(), [])

    def test_just_build_compatibility_is_independent_of_dev(self):
        result = subprocess.run(["just", "build"], cwd=self.root, env=self.env,
                                text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        self.assertEqual(result.returncode, 0, result.stdout)
        self.assertIn("build-only", result.stdout.lower())
        event, = self.recorded()
        self.assertEqual(event["args"][:2], ["buildx", "build"])
        self.assertIn("lunirelust:local", self.option(event["args"], "--tag"))
        self.assertFalse((self.root / ".env").exists())
        self.assertFalse((self.root / ".local").exists())
        for command in ("dev", "check", "test", "ci"):
            result = subprocess.run(["just", "--dry-run", command], cwd=self.root, env=self.env,
                                    text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
            self.assertEqual(result.returncode, 0, result.stdout)
            self.assertNotIn("image.sh", result.stdout)
            self.assertNotIn("buildx", result.stdout)
        result = subprocess.run(["just", "build-down"], cwd=self.root, env=self.env,
                                text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("retired", result.stdout.lower())
        self.assertEqual(len(self.recorded()), 1)

    def test_just_export_preserves_literal_path_arguments(self):
        archive = self.scratch / "archive 'literal' $(touch forbidden).tar"
        result = subprocess.run(["just", "image-export", "lunirelust:review", str(archive)],
                                cwd=self.root, env=self.env, text=True,
                                stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        self.assertEqual(result.returncode, 0, result.stdout)
        self.assertTrue(archive.exists())
        self.assertFalse((self.root / "forbidden").exists())

    def test_canonical_dockerfile_contract(self):
        dockerfile = (SOURCE / "Dockerfile").read_text()
        self.assertFalse((SOURCE / "Dockerfile.local").exists())
        self.assertFalse((SOURCE / "docker-compose.build.yml").exists())
        self.assertNotIn("ARG GIT_CREDENTIALS", dockerfile)
        self.assertNotIn("git config --global", dockerfile)
        self.assertNotIn("COPY . .", dockerfile)
        self.assertIn("id=git_credentials", dockerfile)
        self.assertIn("fetch-dependencies.sh", dockerfile)
        self.assertIn("--locked --release --no-default-features", dockerfile)
        self.assertIn("node:22-", dockerfile)
        self.assertIn("rust:1.92-", dockerfile)
        self.assertEqual(dockerfile.count("1.59.1"), 1, "Playwright pin has multiple sources")
        self.assertIn("install-deps chromium", dockerfile)
        self.assertIn("package-lock.json", dockerfile)
        patterns = (SOURCE / ".dockerignore").read_text().splitlines()
        self.assertEqual(patterns[2], "**")
        # Docker cleans trailing slashes; a directory exception also exposes all
        # descendants, including credentials or custom asset files in that tree.
        for pattern in patterns:
            if pattern.startswith("!"):
                self.assertFalse(pattern.endswith(("/", "/**")), "broad context exception: " + pattern)


unittest.main(verbosity=2)
PY
