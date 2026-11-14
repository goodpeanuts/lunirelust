#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
cd "$repo_root"

dev_compose=(docker compose --project-name lunirelust-dev --env-file /dev/null -f compose.middleware.yml -f compose.dev.yml)
test_compose=(docker compose --project-name lunirelust-test --env-file /dev/null -f compose.middleware.yml -f compose.test.yml)

case "${1:-}" in
    test|test-search|test-up|test-down)
        if [[ -z "${LUNIRELUST_TEST_LOCK_FD:-}" ]]; then
            exec python3 "$repo_root/scripts/test-lock.py" "$0" "$@"
        fi
        python3 "$repo_root/scripts/test-lock.py" --check "$LUNIRELUST_TEST_LOCK_FD"
        "$repo_root/scripts/check-config.sh" --loaded test
        ;;
    build|build-down)
        echo "workflow.sh $1 is retired; use just image-build IMAGE (build-only) or just dev-down for native development middleware." >&2
        exit 2
        ;;
    dev|dev-infra|dev-down|migrate)
        "$repo_root/scripts/check-config.sh" --loaded dev
        ;;
esac

case "${SEARCH_BACKEND:-postgres}" in
    postgres) ;;
    meili-keyword) dev_compose+=(--profile search) ;;
    meili-hybrid) dev_compose+=(--profile hybrid) ;;
    *) echo "invalid SEARCH_BACKEND" >&2; exit 2 ;;
esac

prepare_dev_directories() {
    mkdir -p "$repo_root/.local/dev/postgres" "$repo_root/.local/dev/meilisearch"
    mkdir -p -- "$ASSETS_PUBLIC_PATH" "$ASSETS_PRIVATE_PATH"
}

start_dev_infrastructure() {
    prepare_dev_directories
    "${dev_compose[@]}" up -d --wait
}

validated_test_root() {
    # Recheck paths immediately before cleanup, including dangling symlinks.
    python3 "$repo_root/scripts/check-config.py" environment test "$repo_root" || return
    printf '%s\n' "$repo_root/.local/test"
}

cleanup_test() {
    local test_root
    test_root="$(validated_test_root)" || return
    "${test_compose[@]}" --profile search down -v --remove-orphans || return
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
    if [[ "${1:-}" == "search" ]]; then
        curl --fail --silent --show-error --retry 20 --retry-delay 1 \
            --retry-connrefused --max-time 2 "http://127.0.0.1:7701/health" >/dev/null
    fi
}

cleanup_on_exit() {
    local status=$? cleanup_status=0
    trap - EXIT
    trap '' INT TERM
    if [[ "$status" != 0 || "${keep_test_infrastructure:-0}" != 1 ]]; then
        cleanup_test || cleanup_status=$?
        if [[ "$status" == 0 ]]; then status="$cleanup_status"; fi
    fi
    exit "$status"
}

prepare_test_cleanup() {
    trap cleanup_on_exit EXIT
    trap 'exit 130' INT
    trap 'exit 143' TERM
}

run_test_suite() {
    local features=(--no-default-features)
    [[ "${1:-}" != search ]] || features=(--all-features)
    prepare_test_cleanup
    start_test_infrastructure "${1:-}"

    cargo run --quiet -p migration -- up
    cargo nextest run "${features[@]}"
    cargo test --quiet --workspace --doc "${features[@]}"
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
        "${dev_compose[@]}" --profile hybrid down --remove-orphans
        ;;
    test)
        run_test_suite
        ;;
    test-search)
        test_compose+=(--profile search)
        run_test_suite search
        ;;
    test-up)
        prepare_test_cleanup
        start_test_infrastructure
        cargo run --quiet -p migration -- up
        keep_test_infrastructure=1
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
        echo "usage: $0 {dev|dev-infra|dev-down|test|test-search|test-up|test-down|migrate}" >&2
        exit 2
        ;;
esac
