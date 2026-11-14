#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
cd "$repo_root"

if (( $# == 0 )); then
    "$0" dev
    "$0" test
    exit
fi

if [[ "$1" != --loaded ]]; then
    mode="$1"
    case "$mode" in
        dev)
            env_file="${2:-.env}"
            if [[ $# == 1 && ! -f "$env_file" ]]; then env_file=.env.example; fi
            ;;
        test) env_file="${2:-.env.test}" ;;
        *) echo "usage: $0 [dev|test [env-file]]" >&2; exit 2 ;;
    esac
    exec "$repo_root/scripts/env-run.sh" "$env_file" "$0" --loaded "$mode"
fi

mode="${2:-}"
python3 "$repo_root/scripts/check-config.py" environment "$mode" "$repo_root"
compose=(docker compose --project-name "lunirelust-$mode" --env-file /dev/null \
    -f compose.middleware.yml -f "compose.$mode.yml")
profiles=(default search)
[[ "$mode" != dev ]] || profiles+=(hybrid)
for profile in "${profiles[@]}"; do
    command=("${compose[@]}")
    [[ "$profile" == default ]] || command+=(--profile "$profile")
    # Rendered environment values and Compose parse errors may contain secrets.
    if ! rendered="$("${command[@]}" config --format json 2>/dev/null)"; then
        echo "configuration check failed: cannot render $mode/$profile Compose configuration" >&2
        exit 1
    fi
    printf '%s\n' "$rendered" | python3 "$repo_root/scripts/check-config.py" compose "$mode" "$repo_root" "$profile"
done
echo "$mode configuration OK"
