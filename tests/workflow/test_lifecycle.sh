#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

first="$scratch/first"
second="$scratch/second"
make_checkout "$first"
make_checkout "$second"
mkdir -p "$scratch/bin"
export REAL_DOCKER="$(command -v docker)"
export EVENTS="$scratch/events" STARTED="$scratch/started" RELEASE="$scratch/release"
touch "$EVENTS"
cat > "$scratch/bin/docker" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
for argument in "$@"; do
    if [[ "$argument" == config ]]; then exec "$REAL_DOCKER" "$@"; fi
done
if [[ "$*" == *"-f docker-compose.build.yml up"* ]]; then
    for directory in .local/dev/postgres .local/dev/meilisearch "$ASSETS_HOME_PATH" \
        "$ASSETS_DATA_PATH" "$ASSETS_PUBLIC_PATH" "$ASSETS_PRIVATE_PATH"; do
        [[ -d "$directory" && -w "$directory" ]] || {
            echo "development directory was not prepared before build startup" >&2
            exit 1
        }
    done
fi
printf '%s|docker|%s\n' "$PWD" "$*" >> "$EVENTS"
SH
cat > "$scratch/bin/cargo" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
trap 'exit 130' INT
trap 'exit 143' TERM
printf '%s|cargo|%s|%s\n' "$PWD" "$*" "$DATABASE_URL" >> "$EVENTS"
if [[ "$1" == nextest ]]; then
    touch "$STARTED"
    while [[ "${HOLD_TEST:-0}" == 1 && ! -f "$RELEASE" ]]; do sleep 0.05; done
    exit "${FAIL_TEST:-0}"
fi
SH
cat > "$scratch/bin/curl" <<'SH'
#!/usr/bin/env bash
exit 0
SH
chmod +x "$scratch/bin/"*
export PATH="$scratch/bin:$PATH"

run_workflow() {
    local checkout="$1" command="$2"
    "$checkout/scripts/env-run.sh" .env.test "$checkout/scripts/workflow.sh" "$command"
}

wait_started() {
    local attempt
    for attempt in {1..200}; do
        [[ -e "$STARTED" ]] && return
        kill -0 "$running" 2>/dev/null || fail "workflow exited before nextest"
        sleep 0.05
    done
    fail "workflow did not reach nextest"
}

finish() {
    if [[ -n "${running:-}" ]]; then
        kill -TERM "$running" 2>/dev/null || true
        wait "$running" 2>/dev/null || true
    fi
    rm -rf -- "$scratch"
}
trap finish EXIT

# Parent components must not create intermediary directories in database storage.
unsafe="$scratch/parent-path"
make_checkout "$unsafe"
printf '\nASSETS_PUBLIC_PATH=.local/dev/postgres/review-missing/../../assets/public\n' >> "$unsafe/.env"
expect_failure "$unsafe/scripts/env-run.sh" .env "$unsafe/scripts/workflow.sh" dev-infra
[[ ! -e "$unsafe/.local" ]] || fail "unsafe asset input created development directories"
[[ ! -s "$EVENTS" ]] || fail "unsafe asset input reached Docker startup"

unsafe_test="$scratch/test-parent-path"
make_checkout "$unsafe_test"
printf '\nASSETS_PRIVATE_PATH=.local/dev/postgres/review-missing/../../../test/assets/private\n' \
    >> "$unsafe_test/.env.test"
expect_failure "$unsafe_test/scripts/env-run.sh" .env.test "$unsafe_test/scripts/workflow.sh" test-up
[[ ! -e "$unsafe_test/.local" ]] || fail "unsafe test asset input created local directories"
[[ ! -s "$EVENTS" ]] || fail "unsafe test asset input reached Docker or Cargo"

# The retained build path must prepare writable bind mounts before Docker runs.
"$first/scripts/env-run.sh" .env "$first/scripts/workflow.sh" build > "$scratch/build.log" 2>&1 \
    || fail "retained build did not prepare development directories"

legacy="$scratch/legacy"
make_checkout "$legacy"
printf '\nASSETS_HOME_PATH=assets\nASSETS_PUBLIC_PATH=assets/public\nASSETS_PRIVATE_PATH=assets/private\n' \
    >> "$legacy/.env"
rm -rf -- "$legacy/assets"
"$legacy/scripts/env-run.sh" .env "$legacy/scripts/workflow.sh" build > "$scratch/legacy-build.log" 2>&1 \
    || fail "configured legacy development asset directories were not prepared"
: > "$EVENTS"

# Invalid identities and asset paths must fail before the first destructive command.
cp "$first/.env.test" "$scratch/valid.env"
for value in 'APP_ENV=prod' \
    'DATABASE_URL=postgres://production:synthetic-secret@production:5432/production' \
    'ASSETS_PRIVATE_PATH=/tmp/production-assets'; do
    cp "$scratch/valid.env" "$first/.env.test"
    printf '\n%s\n' "$value" >> "$first/.env.test"
    expect_failure run_workflow "$first" test
    [[ ! -s "$EVENTS" ]] || fail "unsafe configuration reached Docker or Cargo"
done
cp "$scratch/valid.env" "$first/.env.test"
mkdir -p "$first/.local" "$scratch/outside"
printf 'keep\n' > "$scratch/outside/sentinel"
for destination in "$scratch/outside" "$scratch/not-created"; do
    ln -s "$destination" "$first/.local/test"
    expect_failure run_workflow "$first" test-down
    [[ ! -s "$EVENTS" ]] || fail "unsafe test-root symlink reached Docker cleanup"
    [[ -f "$scratch/outside/sentinel" ]] || fail "cleanup escaped the checkout"
    rm "$first/.local/test"
done

HOLD_TEST=1 "$first/scripts/env-run.sh" .env.test "$first/scripts/workflow.sh" test > "$scratch/first.log" 2>&1 &
running=$!
wait_started
cp "$EVENTS" "$scratch/before-contenders"
for command in test test-search test-up test-down; do
    expect_failure run_workflow "$second" "$command"
    grep -Eqi 'busy|already running|lock' "$scratch/rejected.log" || fail "busy failure was not helpful"
    cmp -s "$EVENTS" "$scratch/before-contenders" || fail "another worktree entered the locked lifecycle"
done
touch "$RELEASE"
wait "$running"
running=""
run_workflow "$second" test-down
grep -q 'nextest run --no-default-features' "$EVENTS" || fail "normal tests did not use production features"
grep -q 'test --quiet --workspace --doc --no-default-features' "$EVENTS" || fail "normal doctests did not use production features"

for mode in failure TERM INT; do
    : > "$EVENTS"
    rm -f "$STARTED" "$RELEASE"
    if [[ "$mode" == failure ]]; then
        set +e
        FAIL_TEST=47 run_workflow "$first" test > "$scratch/$mode.log" 2>&1
        status=$?
        set -e
        [[ "$status" == 47 ]] || fail "test failure status became $status"
    else
        HOLD_TEST=1 "$first/scripts/env-run.sh" .env.test "$first/scripts/workflow.sh" test > "$scratch/$mode.log" 2>&1 &
        running=$!
        wait_started
        kill -"$mode" "$running"
        set +e
        wait "$running"
        status=$?
        set -e
        running=""
        expected=143
        [[ "$mode" == TERM ]] || expected=130
        [[ "$status" == "$expected" ]] || fail "$mode status became $status"
    fi
    [[ "$(grep -c 'down -v --remove-orphans' "$EVENTS")" == 2 ]] || fail "$mode did not clean before and after tests"
    [[ ! -e "$first/.local/test" ]] || fail "$mode left disposable assets behind"
    run_workflow "$second" test-down
done

: > "$EVENTS"
env DATABASE_URL=postgres://production:synthetic-secret@production:5432/production \
    COMPOSE_PROJECT_NAME=production COMPOSE_FILE=/does/not/exist COMPOSE_PROFILES=hybrid \
    "$first/scripts/env-run.sh" .env.test "$first/scripts/workflow.sh" test-search > "$scratch/search.log" 2>&1
grep -q 'nextest run --all-features' "$EVENTS" || fail "search suite omitted feature coverage"
grep -q -- '--profile search up' "$EVENTS" || fail "search suite did not start MeiliSearch"
if grep -Eq 'production|hybrid' "$EVENTS"; then fail "terminal settings redirected test resources"; fi
if grep 'docker' "$EVENTS" | grep -v -- '--project-name lunirelust-test.*-f compose.middleware.yml -f compose.test.yml' >/dev/null; then
    fail "cleanup or startup targeted a different Compose project"
fi

echo "PASS: guarded lifecycle, shared lock, failure and signal cleanup"
