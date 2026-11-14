# Development Guide

Use the [justfile](justfile) as the command catalog. Application code, registered
migrations, and executable workflow files are the sources of truth. Private
deployment details are in [subm/README.md](subm/README.md).

## Setup

The native workflow supports Linux, macOS, and WSL with Bash, Python 3 (standard
library only), the Rust version pinned in [rust-toolchain](rust-toolchain), Docker
with Compose v2, and `just`. Python validates local configuration and holds the
shared test lock; no background service is installed. Development hot reload uses
`cargo-watch` when installed;
otherwise the same command runs the backend with `cargo run`. Tests require
`cargo-nextest` and Node.js 22 with npm (the same Node major version as the
[Dockerfile](Dockerfile)). The crawler startup regression requires Node even though
it does not launch a browser; actual crawling also needs the browser dependencies
used by `luneth`. Some Git dependencies and the submodule require repository access.

```bash
cargo install --locked just cargo-watch cargo-nextest
python3 --version
node --version
npm --version
git submodule update --init --recursive
just dev
```

`just dev` copies [.env.example](.env.example) to a private `.env` only when missing,
sets mode `0600`, starts PostgreSQL, applies forward migrations, and runs the backend
natively with Swagger enabled. It preserves existing configuration. The example
binds the API to `127.0.0.1:8012`; Swagger UI is at `/docs`.

`just dev-infra` starts only selected middleware. `just dev-down` stops all optional
middleware too and preserves `.local/dev`. `just build` builds and starts the optional
containerized application with the same selected middleware; `just build-down` stops
it while preserving data. Both startup paths prepare local bind-mount directories
as the current user before Docker starts. Use the native loop for routine development.

[compose.middleware.yml](compose.middleware.yml) defines shared middleware images,
resource limits, environment, and health checks. Workflows explicitly combine it
with [compose.dev.yml](compose.dev.yml) or [compose.test.yml](compose.test.yml).
The overlays own ports and storage. Development keeps PostgreSQL on
`127.0.0.1:5434`, optional MeiliSearch on `127.0.0.1:7700`, and middleware data under
`.local/dev`. Tests use ports `5433` and `7701` and disposable named volumes.
Only the development overlay offers the optional GPU service.

| `SEARCH_BACKEND` in `.env` | Middleware started | Search behavior |
| --- | --- | --- |
| `postgres` (default) | PostgreSQL | Literal, case-insensitive SQL search |
| `meili-keyword` | PostgreSQL + MeiliSearch (`search` profile) | Keyword search, SQL fallback |
| `meili-hybrid` | PostgreSQL + MeiliSearch + vLLM (`hybrid` profile) | Optional semantic ranking, keyword/SQL fallback |

Switch modes by stopping the dev stack, editing `.env`, and starting it again.
Hybrid mode requires a compatible GPU, model cache/download, and additional memory;
it is optional. `COMPOSE_PROFILES` inherited from the terminal is cleared so backend
selection is explicit. Production uses the equivalent private Compose profiles.

PostgreSQL responses retain the API mode string `sql_fallback` for compatibility.
SQL results use record date descending and ID ascending, followed by director,
studio, label, series, genre, and idol results ordered by name and ID. Page size is
capped at 100. Unknown nonempty entity types and invalid/reversed date ranges return
validation errors. `%`, `_`, and backslashes in search text are literal characters.
Single-character terms remain supported but can require broad scans. Trigram indexes
accelerate selective text searches; they do not provide semantic ranking.

## Resource limits

[Cargo configuration](.cargo/config.toml) limits compilation to two jobs. Dev/test
profiles omit full debug information and avoid optimizing every dependency during
local compilation. Set `CARGO_BUILD_JOBS=1` for a tighter memory budget; override
profile debug settings explicitly when source-level debugging is needed. Release
optimization remains enabled.

Development/test PostgreSQL has a 512 MiB container limit. Optional MeiliSearch and
vLLM are bounded separately in Compose. The default application database pool is
1–5 connections. Nextest runs one test process at a time because integration tests
share a fixed disposable database; process-local mutexes cannot isolate them.
All fixed test infrastructure commands acquire the same process-held lock at
`/tmp/lunirelust-test.lock`, including commands run from other worktrees. A busy
command exits with a helpful error before cleanup or startup. The lock remains held
through migrations, tests, and cleanup; the operating system releases it when its
holders exit. The lock file stays in place and must not be deleted to bypass a busy
workflow.

## Database isolation and migrations

| Environment | Database identity | Data lifecycle |
| --- | --- | --- |
| Development | `luna_dev@127.0.0.1:5434/luna_dev` | Persistent middleware in `.local/dev`, configured checkout assets |
| Test | `luna_test@127.0.0.1:5433/luna_test` | Disposable named volumes and `.local/test` |

The safe environment loader clears inherited database, all `COMPOSE_*`, search,
and asset variables before loading the selected file. Values are literal: shell
commands, variable references, and quotes are not evaluated. Compose receives those
loaded values with an empty Compose env file, so it does not reinterpret `.env`.
Use the recipes instead of sourcing dotenv files. Project names, middleware ports,
and middleware storage paths are fixed by the workflow and overlays; the samples omit unused
settings for overriding them. Existing `.env` files are not rewritten.

New development configuration stores assets under `.local/dev/assets`. Existing
native layouts such as `assets/public` and `assets/private` remain supported.
Configured asset directories must resolve inside the checkout and must not overlap
repository metadata, the private submodule, or middleware/test data. Startup prepares
those configured directories without moving existing files. Both development and
test asset paths reject parent (`..`) components before directory creation, even
when the final resolved path is allowed. Ordinary relative and absolute paths are supported.
Symlinks that escape the checkout are rejected. Test asset directories remain fixed
under `.local/test`.

Before starting or removing middleware, the workflow validates database identity,
asset paths, and the rendered Compose model without printing credentials. Test
paths must resolve inside the checkout's `.local/test`; symlinks that escape that
location, including dangling symlinks, are rejected before cleanup. Connections and
migrations validate `APP_ENV`, protocol, host, port,
user, and database. Database integration tests additionally require `APP_ENV=test`.
Keep these guards when changing workflows.

The application never migrates on startup. Use `just migrate-up` and
`just migrate-status` for development; [migration/README.md](migration/README.md)
describes the migration entry points. Destructive migration commands are rejected
in production.

Never point tests at production or mount a production data directory into a test
container. An explicitly authorized read-only export may be restored into a separate
disposable database for investigation. Keep its credentials, volumes, assets, and
ports isolated, and do not commit or publish the copied data.

## Verification

```bash
just config-check # Read-only Compose/environment validation; no daemon required
just workflow-test # Configuration and lifecycle regressions; no daemon required
just check  # Formatting check and clippy (which also type-checks all targets)
just test   # PostgreSQL, migrations, nextest, and doctests
just test-search # Also run the live MeiliSearch integration coverage
just ci     # check followed by test
just audit  # cargo-deny and typos; requires those tools
```

`just test` destroys the fixed test project and volumes before starting, then cleans
them after the suite, including test failures, Ctrl-C, and termination. The original
failure or signal exit status is preserved. `just test-db` creates the same
disposable environment for manual debugging; its lock is released when setup
finishes, so it does not reserve the database against a later workflow. Coordinate
manual sessions accordingly. `just test-db-down` removes that environment under the
same lock. Direct database integration tests must use the test configuration.

The default suite runs nextest and doctests with `--no-default-features`, matching
the production feature set, and intentionally omits live MeiliSearch coverage.
`just test-search` starts MeiliSearch, waits for health, and runs nextest and doctests
with `--all-features`. Semantic-service failure and recovery cases use local
deterministic fakes; no GPU is needed for tests. Both commands apply migrations
before testing and retain the single-process nextest limit.

`just config-check` checks the existing `.env` (or `.env.example` when absent) and
`.env.test`, renders all supported local profiles, and leaves files and containers
unchanged. `just workflow-test` renders real Compose configurations using synthetic
env files and substitutes Docker mutations and Cargo only for lifecycle tests. It
checks cross-worktree contention, unsafe paths, inherited settings, and cleanup
after failures and signals. It uses the same shared test lock, so run it separately
from other test workflows.

The root [.pre-commit-config.yaml](.pre-commit-config.yaml) runs repository hygiene
checks and Rust formatting for Rust changes. It does not start containers or run the
full suite for a documentation commit. Install `pre-commit` and run `pre-commit install`
to enable it; `pre-commit run --all-files` checks the whole checkout. Full validation
remains `just ci`, plus `just test-search` when search behavior changes.

GitHub CI runs static checks once, spelling checks, and the reusable database test
workflow with MeiliSearch coverage. Tagged image publication depends on static checks,
database tests, and platform builds. Passing CI does not by itself establish that
remote branch protection is enabled.

## Contributions and the private submodule

Use a topic branch (normally `codex/`, `fix/`, or `feat/`) and a focused reviewable
diff. New comments, documentation, and commit messages are English. New documentation
filenames use Chinese; established entry points such as `README.md` remain stable.
Follow workspace Rust lints and the existing directory-plus-same-name-file module
layout. Conventional Commit subjects use `type(scope): summary` with optional scope.

Keep credentials, deployment configuration, host details, backups, and real data
out of the public repository and build context. The public contributor rules in
[CLAUDE.md](CLAUDE.md) work without private documentation access. OpenSpec is retired;
its private archive is historical evidence, not an active change workflow. Keep
current documentation short and update it with the behavior it describes.

`subm` is a separate Git repository; the parent records a commit SHA, not a branch.
Check out a topic branch inside it before editing. When publishing changes, push the
private commit first, verify that a remote branch contains it, and only then record
the gitlink in the public repository. A private commit that exists only locally
breaks fresh submodule checkouts. Recover detached commits through the submodule
reflog before changing the recorded pointer.

Release and deployment procedures are in the [private index](subm/README.md) and
[deployment guide](subm/luna/README.md). Generated OpenAPI snapshots are documented
in [the API generation guide](subm/docs/api/生成说明.md); edit Rust API declarations
and regenerate snapshots from a local server instead of hand-editing schemas.
