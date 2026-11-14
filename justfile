set shell := ["bash", "-euo", "pipefail", "-c"]

default:
    @just --list

# Create the private development environment once; never overwrite local choices.
[private]
ensure-dev-env:
    @if [[ ! -f .env ]]; then cp .env.example .env; echo "created .env from .env.example"; fi
    @chmod 0600 .env

# Start dev infrastructure, apply forward migrations, and run the backend.
dev: ensure-dev-env
    ./scripts/env-run.sh .env ./scripts/workflow.sh dev

dev-infra: ensure-dev-env
    ./scripts/env-run.sh .env ./scripts/workflow.sh dev-infra

dev-down: ensure-dev-env
    ./scripts/env-run.sh .env ./scripts/workflow.sh dev-down

# Disposable test stack: down -v before and after, migrate, nextest, doctest.
test:
    ./scripts/env-run.sh .env.test ./scripts/workflow.sh test

# Also exercise live Meilisearch integrations; embeddings use deterministic fakes.
test-search:
    ./scripts/env-run.sh .env.test ./scripts/workflow.sh test-search

test-db:
    ./scripts/env-run.sh .env.test ./scripts/workflow.sh test-up

test-db-down:
    ./scripts/env-run.sh .env.test ./scripts/workflow.sh test-down

# Static quality gates; no database is required.
check:
    cargo fmt --all -- --check
    cargo clippy --quiet --workspace --all-targets --all-features -- -D warnings -W clippy::all

# Read-only local configuration validation; does not require a Docker daemon.
config-check:
    ./scripts/check-config.sh

# Workflow regressions use real Compose rendering and stub destructive commands.
workflow-test:
    ./tests/workflow/test_config.sh
    ./tests/workflow/test_lifecycle.sh
    ./tests/workflow/test_image_commands.sh

ci: check test

migrate-up: ensure-dev-env
    ./scripts/env-run.sh .env ./scripts/workflow.sh migrate up

migrate-status: ensure-dev-env
    ./scripts/env-run.sh .env ./scripts/workflow.sh migrate status

migrate-down: ensure-dev-env
    ./scripts/env-run.sh .env ./scripts/workflow.sh migrate down

# Optional cached Cargo inputs; no application services are started.
vendor:
    cargo vendor --locked vendor/ > .cargo/config.vendor.toml

# Optional local application image; build and load only.
[positional-arguments]
image-build image *options:
    ./scripts/image.sh build "$@"

# Export an existing image without replacing an archive.
[positional-arguments]
image-export image output:
    ./scripts/image.sh export -- "$@"

# Legacy build-only alias; accepts the same options as image-build.
[positional-arguments]
build image="lunirelust:local" *options:
    @echo "just build is a build-only alias for just image-build."
    ./scripts/image.sh build "$@"

build-down:
    @echo "build-down is retired: image builds start no services. Use just dev-down for native development middleware." >&2
    @exit 2

audit:
    cargo deny check -d
    typos
