#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
cd "$repo_root"

dev_compose=(docker compose --project-name lunirelust-dev --env-file .env -f compose.dev.yml)
test_compose=(docker compose --project-name lunirelust-test --env-file .env.test -f compose.test.yml)

start_dev_infrastructure() {
    mkdir -p "$repo_root/.local/dev/postgres" "$repo_root/.local/dev/meilisearch"
    "${dev_compose[@]}" up -d --wait
}

canonicalize_path() {
    # Portable canonicalization that tolerates a missing final component
    # (GNU `realpath -m` behavior): resolve the deepest existing ancestor
    # with cd -P/pwd -P, then append the lexical remainder.
    local path="$1" suffix="" resolved
    # Split declaration: bash 3.2 expands every word of one `local` command
    # before assigning, so `ancestor="$path"` must be a separate statement.
    local ancestor="$path"
    while [ ! -e "$ancestor" ] && [ "$ancestor" != "/" ]; do
        suffix="/$(basename "$ancestor")$suffix"
        ancestor="$(dirname "$ancestor")"
    done
    if ! resolved="$(cd -P "$ancestor" 2>/dev/null && pwd -P)"; then
        return 1
    fi
    printf '%s%s\n' "$resolved" "$suffix"
}

validated_test_root() {
    local expected="$repo_root/.local/test"
    local resolved
    resolved="$(canonicalize_path "$expected")" || resolved=""
    if [[ "$resolved" != "$expected" ]]; then
        echo "refusing to clean test data outside $expected (resolved to $resolved)" >&2
        return 1
    fi
    printf '%s\n' "$expected"
}

cleanup_test() {
    local test_root
    test_root="$(validated_test_root)"
    "${test_compose[@]}" down -v --remove-orphans
    rm -rf -- "$test_root"
}

start_test_infrastructure() {
    cleanup_test
    mkdir -p "$repo_root/.local/test/assets/public" "$repo_root/.local/test/assets/private"
    mkdir -p "$repo_root/.local/test/assets/private/profile_picture"
    install -m 0644 "$repo_root/assets/public/images.jpeg" \
        "$repo_root/.local/test/assets/public/images.jpeg"
    install -m 0644 "$repo_root/assets/private/profile_picture/images.jpeg" \
        "$repo_root/.local/test/assets/private/profile_picture/images.jpeg"
    "${test_compose[@]}" up -d --wait
}

run_test_suite() {
    start_test_infrastructure
    cleanup_on_exit() {
        local status=$?
        trap - EXIT INT TERM
        cleanup_test || true
        exit "$status"
    }
    trap cleanup_on_exit EXIT INT TERM

    cargo run --quiet -p migration -- up
    cargo nextest run --all-features
    cargo test --quiet --workspace --doc
}

case "${1:-}" in
    dev)
        start_dev_infrastructure
        cargo run --quiet -p migration -- up
        if cargo watch --version >/dev/null 2>&1; then
            exec cargo watch -w src -w Cargo.toml -w Cargo.lock -x "run -F swagger"
        fi
        echo "cargo-watch is not installed; starting without hot reload" >&2
        exec cargo run -F swagger
        ;;
    dev-infra)
        start_dev_infrastructure
        ;;
    dev-down)
        "${dev_compose[@]}" down --remove-orphans
        ;;
    test)
        run_test_suite
        ;;
    test-up)
        start_test_infrastructure
        cargo run --quiet -p migration -- up
        ;;
    test-down)
        cleanup_test
        ;;
    migrate)
        shift
        if (( $# == 0 )); then
            echo "migration command is required" >&2
            exit 2
        fi
        exec cargo run -p migration -- "$@"
        ;;
    *)
        echo "usage: $0 {dev|dev-infra|dev-down|test|test-up|test-down|migrate}" >&2
        exit 2
        ;;
esac
