#!/usr/bin/env bash
set -euo pipefail

if (( $# < 2 )); then
    echo "usage: $0 <env-file> <command> [args...]" >&2
    exit 2
fi

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
env_file="$1"
shift

if [[ "$env_file" != /* ]]; then
    env_file="$repo_root/$env_file"
fi

if [[ ! -f "$env_file" ]]; then
    echo "environment file does not exist: $env_file" >&2
    exit 2
fi

# Remove inherited values that can redirect services or data before loading the
# explicitly selected file. Names are inspected only; values are never printed.
while IFS='=' read -r name _; do
    case "$name" in
        APP_ENV|LUNA_ENV|DATABASE_URL|COMPOSE_PROJECT_NAME|DEPLOY_ENV_FILE|POSTGRES_*|MEILI_*|ASSETS_*)
            unset "$name"
            ;;
    esac
done < <(env)

while IFS= read -r line || [[ -n "$line" ]]; do
    line="${line%$'\r'}"
    [[ -z "$line" || "$line" =~ ^[[:space:]]*# ]] && continue

    if [[ "$line" != *=* ]]; then
        echo "invalid environment entry in $env_file: expected KEY=VALUE" >&2
        exit 2
    fi

    key="${line%%=*}"
    value="${line#*=}"
    if [[ ! "$key" =~ ^[A-Za-z_][A-Za-z0-9_]*$ ]]; then
        echo "invalid environment variable name in $env_file: $key" >&2
        exit 2
    fi
    if [[ "$key" == "LUNA_ENV" ]]; then
        echo "LUNA_ENV is no longer supported; use APP_ENV in $env_file" >&2
        exit 2
    fi

    export "$key=$value"
done < "$env_file"

cd "$repo_root"
exec "$@"
