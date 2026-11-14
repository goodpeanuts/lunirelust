#!/usr/bin/env bash
set -euo pipefail

if [[ -z "${GIT_CREDENTIALS:-}" ]]; then
    echo "Private dependency access is required for full Rust verification; use a reviewed branch with repository access." >&2
    exit 1
fi

# Git reads this process-only helper for GitHub HTTPS requests. The credential
# stays out of URLs, Git configuration files, Cargo caches, and later build steps.
helper='!f() { if [ "$1" = get ]; then printf "%s\n" "username=x-access-token" "password=$GIT_CREDENTIALS"; fi; }; f'
exec env GIT_CONFIG_COUNT=1 \
    GIT_CONFIG_KEY_0=credential.https://github.com.helper \
    GIT_CONFIG_VALUE_0="$helper" \
    GIT_TERMINAL_PROMPT=0 CARGO_NET_GIT_FETCH_WITH_CLI=true \
    cargo fetch --locked "$@"
