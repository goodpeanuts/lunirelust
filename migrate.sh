#!/usr/bin/env bash
set -euo pipefail

if (( $# == 0 )); then
    echo "usage: $0 {up|status|down|fresh|reset|refresh|generate} [args...]" >&2
    exit 2
fi

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
cd "$repo_root"
command_name="$1"

case "$command_name" in
    generate)
        exec cargo run -p migration -- "$@"
        ;;
    up|status|down|fresh|reset|refresh)
        env_file="${MIGRATION_ENV_FILE:-.env}"
        exec ./scripts/env-run.sh "$env_file" cargo run -p migration -- "$@"
        ;;
    *)
        echo "unsupported migration command: $command_name" >&2
        exit 2
        ;;
esac
