#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

checkout="$scratch/checkout"
make_checkout "$checkout"
[[ -f "$checkout/compose.middleware.yml" ]] || fail "shared middleware base is missing"
[[ -x "$checkout/scripts/check-config.sh" ]] || fail "read-only configuration check is missing"

cp "$checkout/.env" "$scratch/original.env"
# Retired container settings are neither required nor active path inputs.
sed -i '/^\(DEPLOY_ENV_FILE\|ASSETS_HOME_PATH\|ASSETS_DATA_PATH\)=/d' "$checkout/.env"
"$checkout/scripts/check-config.sh" dev > "$scratch/active-assets-only.log" 2>&1 \
    || fail "native configuration still requires retired container settings"
printf '\nDEPLOY_ENV_FILE=/unused\nASSETS_HOME_PATH=/unused\nASSETS_DATA_PATH=/unused\n' >> "$checkout/.env"
"$checkout/scripts/check-config.sh" dev > "$scratch/retired-settings.log" 2>&1 \
    || fail "retired container settings are still active"
cp "$scratch/original.env" "$checkout/.env"
"$checkout/scripts/check-config.sh" > "$scratch/initial-check.log"
cmp -s "$scratch/original.env" "$checkout/.env" || fail "configuration check changed .env"
rm "$checkout/.env"
"$checkout/scripts/check-config.sh" dev > "$scratch/fallback-check.log"
[[ ! -e "$checkout/.env" && ! -e "$checkout/.local" ]] || fail "configuration check created local files"
cp "$scratch/original.env" "$checkout/.env"

# Existing native checkouts can keep assets under their configured checkout paths.
for prefix in "" "$checkout/"; do
    printf '\nASSETS_PUBLIC_PATH=%sassets/public\nASSETS_PRIVATE_PATH=%sassets/private\n' \
        "$prefix" "$prefix" >> "$checkout/.env"
    "$checkout/scripts/check-config.sh" dev > "$scratch/legacy-assets.log" 2>&1 \
        || fail "legacy native asset paths inside the checkout were rejected"
    cp "$scratch/original.env" "$checkout/.env"
done

# Runtime assets must not overlap any directory used as application build input.
for key in ASSETS_PUBLIC_PATH ASSETS_PRIVATE_PATH; do
    for directory in src migration .cargo .github; do
        for prefix in "" "$checkout/"; do
            for suffix in "" /private-runtime; do
                printf '\n%s=%s%s%s\n' "$key" "$prefix" "$directory" "$suffix" >> "$checkout/.env"
                expect_failure "$checkout/scripts/check-config.sh" dev
                grep -q "$key" "$scratch/rejected.log" || fail "build-input overlap did not identify the asset key"
                cp "$scratch/original.env" "$checkout/.env"
            done
        done
    done
done

for key in ASSETS_PUBLIC_PATH ASSETS_PRIVATE_PATH; do
    printf '\n%s=.local/dev/postgres/review-missing/../../assets/public\n' "$key" >> "$checkout/.env"
    expect_failure "$checkout/scripts/check-config.sh" dev
    grep -q "$key" "$scratch/rejected.log" || fail "parent-path rejection did not identify the key"
    if grep -q 'review-missing' "$scratch/rejected.log"; then fail "parent-path rejection printed the value"; fi
    cp "$scratch/original.env" "$checkout/.env"
done

cp "$checkout/.env.test" "$scratch/original-test.env"
for entry in ASSETS_PUBLIC_PATH=assets/public ASSETS_PRIVATE_PATH=assets/private; do
    key="${entry%%=*}"
    printf '\n%s=.local/dev/postgres/review-missing/../../../test/%s\n' \
        "$key" "${entry#*=}" >> "$checkout/.env.test"
    expect_failure "$checkout/scripts/check-config.sh" test
    grep -q "$key" "$scratch/rejected.log" || fail "test parent-path rejection did not identify the key"
    if grep -q 'review-missing' "$scratch/rejected.log"; then fail "test parent-path rejection printed the value"; fi
    cp "$scratch/original-test.env" "$checkout/.env.test"
done

for path in /tmp/unrelated-assets subm/assets .git/assets .local/test/assets .local/dev/postgres; do
    printf '\nASSETS_PUBLIC_PATH=%s\n' "$path" >> "$checkout/.env"
    expect_failure "$checkout/scripts/check-config.sh" dev
    cp "$scratch/original.env" "$checkout/.env"
done
ln -s "$scratch/outside-assets" "$checkout/assets/escaped"
printf '\nASSETS_PUBLIC_PATH=assets/escaped\n' >> "$checkout/.env"
expect_failure "$checkout/scripts/check-config.sh" dev
cp "$scratch/original.env" "$checkout/.env"
rm "$checkout/assets/escaped"

"$checkout/scripts/env-run.sh" .env docker compose --project-name lunirelust-base-check \
    --env-file /dev/null -f "$checkout/compose.middleware.yml" --profile search \
    config --format json > "$scratch/base.json"

render() {
    local mode="$1" profile="${2:-}" env_file
    env_file="$checkout/.env"
    [[ "$mode" == dev ]] || env_file="$checkout/.env.test"
    local command=(docker compose --project-name "lunirelust-$mode" --env-file /dev/null \
        -f "$checkout/compose.middleware.yml" -f "$checkout/compose.$mode.yml")
    [[ -z "$profile" ]] || command+=(--profile "$profile")
    "$checkout/scripts/env-run.sh" "$env_file" "${command[@]}" config --format json
}

for mode in dev test; do
    render "$mode" > "$scratch/$mode.json"
    render "$mode" search > "$scratch/$mode-search.json"
    "$checkout/scripts/check-config.sh" "$mode" > "$scratch/$mode-check.log"
done
render dev hybrid > "$scratch/dev-hybrid.json"
python3 - "$scratch" "$checkout" <<'PY'
import json
import pathlib
import sys

scratch, checkout = map(pathlib.Path, sys.argv[1:])
base = json.loads((scratch / "base.json").read_text())
assert set(base["services"]) == {"postgres", "meilisearch"}
assert all(not service.get("ports") and not service.get("volumes")
           for service in base["services"].values()), "shared base must not own host ports or storage"
for mode, port in (("dev", "5434"), ("test", "5433")):
    default = json.loads((scratch / (mode + ".json")).read_text())
    search = json.loads((scratch / (mode + "-search.json")).read_text())
    assert set(default["services"]) == {"postgres"}, "default must start PostgreSQL only"
    assert set(search["services"]) == {"postgres", "meilisearch"}, "search must not start a GPU service"
    for name, service in search["services"].items():
        assert service["healthcheck"]["test"], "middleware must report readiness"
        assert float(service["cpus"]) == (1 if name == "postgres" else 2)
        assert service["image"] == ("postgres:15" if name == "postgres" else "getmeili/meilisearch:v1.42")
        assert int(service["mem_limit"]) == (512 * 1024**2 if name == "postgres" else 1024**3)
        assert "devices" not in service and "deploy" not in service
        mount, = service["volumes"]
        if mode == "dev":
            assert mount["type"] == "bind"
            assert pathlib.Path(mount["source"]) == checkout / ".local/dev" / name
        else:
            assert mount["type"] == "volume"
            assert search["volumes"][mount["source"]]["name"] == "lunirelust-test_" + mount["source"]
            assert not search["volumes"][mount["source"]].get("external", False)
    postgres = search["services"]["postgres"]
    assert postgres["image"] == "postgres:15"
    assert postgres["environment"]["POSTGRES_USER"] == "luna_" + mode
    assert postgres["environment"]["POSTGRES_DB"] == "luna_" + mode
    mapping, = postgres["ports"]
    assert (mapping["host_ip"], mapping["published"], mapping["target"]) == ("127.0.0.1", port, 5432)
    mapping, = search["services"]["meilisearch"]["ports"]
    assert (mapping["host_ip"], mapping["published"], mapping["target"]) == ("127.0.0.1", "7700" if mode == "dev" else "7701", 7700)
assert set(json.loads((scratch / "dev-hybrid.json").read_text())["services"]) == {"postgres", "meilisearch", "vllm"}
PY

# Render the legacy helper's actual argument array without running its data steps.
python3 - "$checkout" <<'PY'
import json
from pathlib import Path
import shlex
import subprocess
import sys

checkout = Path(sys.argv[1])
declarations = [line for line in (checkout / "scripts/migrate-legacy-dev.sh").read_text().splitlines()
                if line.startswith("dev_compose=(")]
assert len(declarations) == 1
command = shlex.split(declarations[0][len("dev_compose=("):-1])
rendered = subprocess.run(command + ["config", "--format", "json"], cwd=str(checkout),
                          stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
assert rendered.returncode == 0, "legacy development Compose command does not render: " + rendered.stderr
assert set(json.loads(rendered.stdout)["services"]) == {"postgres"}
PY

# The shared base is the policy source: deliberate image/limit edits are valid.
cp "$checkout/compose.middleware.yml" "$scratch/middleware.yml"
python3 - "$checkout/compose.middleware.yml" <<'PY'
from pathlib import Path
import sys
path = Path(sys.argv[1])
path.write_text(path.read_text().replace("postgres:15", "postgres:16")
                .replace("meilisearch:v1.42", "meilisearch:v1.43")
                .replace("512m", "768m").replace("1g", "2g").replace("cpus: 1.0", "cpus: 1.5"))
PY
"$checkout/scripts/check-config.sh" > "$scratch/tuned.log" 2>&1 || fail "deliberate middleware image/resource changes were rejected"
cp "$scratch/middleware.yml" "$checkout/compose.middleware.yml"

# Fixed identities and disposable volumes remain safety boundaries.
cp "$checkout/compose.test.yml" "$scratch/test.yml"
for change in identity mount external gpu; do
    cp "$scratch/test.yml" "$checkout/compose.test.yml"
    python3 - "$checkout/compose.test.yml" "$change" <<'PY'
from pathlib import Path
import sys
path = Path(sys.argv[1])
before, after = {
    "identity": ("POSTGRES_DB: luna_test", "POSTGRES_DB: other_database"),
    "mount": ("postgres-data:/var/lib/postgresql/data", "/tmp/unrelated-data:/var/lib/postgresql/data"),
    "external": ("  postgres-data:", "  postgres-data:\n    external: true"),
    "gpu": ("  postgres:", "  postgres:\n    gpus: all"),
}[sys.argv[2]]
assert before in path.read_text()
path.write_text(path.read_text().replace(before, after))
PY
    expect_failure "$checkout/scripts/check-config.sh" test
done
cp "$scratch/test.yml" "$checkout/compose.test.yml"

# A selected literal env file takes precedence over terminal redirection settings.
env DATABASE_URL=postgres://production:synthetic-secret@production:5432/production \
    COMPOSE_PROJECT_NAME=production COMPOSE_FILE=/does/not/exist COMPOSE_PROFILES=hybrid \
    COMPOSE_ENV_FILES=/does/not/exist \
    "$checkout/scripts/env-run.sh" .env.test bash -c \
    '[[ "$DATABASE_URL" == postgres://luna_test:luna_test@127.0.0.1:5433/luna_test && -z "${COMPOSE_FILE+x}" && -z "${COMPOSE_PROFILES+x}" && -z "${COMPOSE_ENV_FILES+x}" ]]' \
    || fail "inherited settings escaped the environment boundary"

printf '\nJWT_SECRET_KEY=$(touch %s/executed)\nMEILI_MASTER_KEY=literal-${HOME}-`touch %s/backticks`\n' \
    "$scratch" "$scratch" >> "$checkout/.env"
"$checkout/scripts/check-config.sh" dev > "$scratch/literal.log" 2>&1
render dev search > "$scratch/literal.json"
[[ ! -e "$scratch/executed" && ! -e "$scratch/backticks" ]] || fail "dotenv shell syntax executed"
python3 - "$scratch/literal.json" "$scratch/literal.log" <<'PY'
import json
import pathlib
import sys
key = json.loads(pathlib.Path(sys.argv[1]).read_text())["services"]["meilisearch"]["environment"]["MEILI_MASTER_KEY"].replace("$$", "$")
assert key.startswith("literal-${HOME}-`touch "), "Compose changed a literal dotenv value"
assert key not in pathlib.Path(sys.argv[2]).read_text(), "config check disclosed a secret"
PY

cp "$checkout/.env.test" "$scratch/valid.env"
for value in \
    'APP_ENV=prod' \
    'DATABASE_URL=postgres://production:synthetic-secret@production:5432/production' \
    'DATABASE_URL=postgres://luna_test:luna_test@127.0.0.1:5433/luna_test?host=production' \
    'POSTGRES_USER=production' \
    'ASSETS_PUBLIC_PATH=/tmp/production-assets'; do
    cp "$scratch/valid.env" "$checkout/.env.test"
    printf '\n%s\n' "$value" >> "$checkout/.env.test"
    expect_failure "$checkout/scripts/check-config.sh" test
    if grep -q 'synthetic-secret' "$scratch/rejected.log"; then
        fail "configuration error disclosed a password"
    fi
done
cp "$scratch/valid.env" "$checkout/.env.test"
mkdir -p "$checkout/.local"
ln -s "$scratch/missing-target" "$checkout/.local/test"
expect_failure "$checkout/scripts/check-config.sh" test

echo "PASS: Compose configuration, environment isolation, and read-only guards"
