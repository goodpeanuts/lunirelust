#!/usr/bin/env bash
# Small fixtures shared by the configuration and lifecycle regression tests.
set -euo pipefail

source_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)"
scratch="$(mktemp -d "${TMPDIR:-/tmp}/lunirelust-workflow.XXXXXX")"
scratch="$(cd "$scratch" && pwd -P)"
trap 'rm -rf -- "$scratch"' EXIT

fail() {
    echo "FAIL: $*" >&2
    exit 1
}

make_checkout() {
    local destination="$1" path
    mkdir -p "$destination/scripts" "$destination/assets/public" \
        "$destination/assets/private/profile_picture"
    for path in "$source_root"/scripts/*.sh "$source_root"/scripts/*.py \
        "$source_root"/compose*.yml; do
        [[ -f "$path" ]] || continue
        case "$path" in
            */scripts/*) cp "$path" "$destination/scripts/" ;;
            *) cp "$path" "$destination/" ;;
        esac
    done
    cp "$source_root/.env.example" "$destination/.env"
    cp "$source_root/.env.example" "$destination/.env.example"
    cp "$source_root/.env.test" "$destination/.env.test"
    printf 'public fixture\n' > "$destination/assets/public/images.jpeg"
    printf 'private fixture\n' > "$destination/assets/private/profile_picture/images.jpeg"
}

expect_failure() {
    if "$@" > "$scratch/rejected.log" 2>&1; then
        fail "unsafe configuration was accepted"
    fi
}
