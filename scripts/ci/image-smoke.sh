#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)"
cd "$repo_root"
if [[ $# != 1 || "${GITHUB_ACTIONS:-}" != true || "${APP_ENV:-}" != test ||
      "${DATABASE_URL:-}" != postgres://luna_test:luna_test@127.0.0.1:5433/luna_test ||
      "${POSTGRES_USER:-}" != luna_test || "${POSTGRES_DB:-}" != luna_test ]]; then
    echo 'Use the CI runner with env-run.sh .env.test and an accepted image artifact directory.' >&2
    exit 2
fi
project="lunirelust-image-${GITHUB_RUN_ID:?}-${GITHUB_RUN_ATTEMPT:?}-${GITHUB_JOB:?}"
[[ "$project" =~ ^lunirelust-image-[0-9]+-[0-9]+-[a-zA-Z0-9_-]+$ ]] || exit 2
artifacts="$1"

# Load the saved archive and compare image ID, source, version, and architecture
# before starting middleware. An incorrect image must never reach migrations.
python3 scripts/ci/release.py load-image --directory "$artifacts"
LUNIRELUST_CI_IMAGE_ID="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["image_id"])' "$artifacts/image.json")"
export LUNIRELUST_CI_IMAGE_ID
unset COMPOSE_PROFILES
compose=(docker compose --project-directory "$repo_root" --project-name "$project" --env-file /dev/null
    -f compose.middleware.yml -f compose.test.yml -f scripts/ci/compose.image.yml)
cleanup() {
    local status=$?
    trap - EXIT INT TERM
    if (( status != 0 )); then "${compose[@]}" logs --no-color --tail 100 app postgres || true; fi
    if ! "${compose[@]}" down --volumes --remove-orphans; then
        echo 'Image smoke cleanup failed for the isolated CI project.' >&2
        if (( status == 0 )); then status=1; fi
    fi
    exit "$status"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

"${compose[@]}" up --detach --wait --wait-timeout 90 postgres
"${compose[@]}" run --rm --no-deps --entrypoint /bin/sh app -c 'mkdir -p /app/assets/public /app/assets/private'
"${compose[@]}" run --rm --no-deps --entrypoint /app/migration app up
"${compose[@]}" exec -T postgres psql -U luna_test -d luna_test -v ON_ERROR_STOP=1 -c \
    "INSERT INTO director (id, name, link, manual) VALUES (999999, 'image-smoke-director', '', true);"
"${compose[@]}" up --detach --no-deps app
"${compose[@]}" exec -T app node < scripts/ci/image-api-smoke.cjs
python3 scripts/ci/release.py smoke-proof --directory "$artifacts"
