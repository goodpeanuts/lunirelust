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

validated_test_root() {
    local expected="$repo_root/.local/test"
    local resolved
    # BSD realpath (macOS) lacks -m; perl's abs_path resolves symlinks and
    # canonicalizes even when the final component does not exist yet.
    resolved="$(perl -MCwd=abs_path -e 'print abs_path($ARGV[0])' "$expected")"
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
