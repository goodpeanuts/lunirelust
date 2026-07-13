# Development Guide

This is the operational guide for Lunirelust: environment setup, local development,
testing, the contribution workflow. For the project overview, feature list, and tech stack, see
[README.md](README.md).

Most day-to-day tasks (dev server, tests, lint, audit, migrations, build) are recipes in
the [`justfile`](justfile). Run `just` with no arguments to list them; the justfile is the
canonical command catalog, so this guide describes tasks by purpose rather than
duplicating recipe names (which may change).

## Contents

- [First-time setup](#first-time-setup)
- [Running locally](#running-locally)
- [Database and migrations](#database-and-migrations)
- [Testing and quality](#testing-and-quality)
- [Contribution workflow](#contribution-workflow)
- [Working with the submodule](#working-with-the-submodule)
- [Changelog and release](#changelog-and-release)
- [Troubleshooting and common pitfalls](#troubleshooting-and-common-pitfalls)



## First-time setup

### Prerequisites

- **Rust** — version pinned by [`rust-toolchain`](rust-toolchain); `rustup` picks it up
  automatically inside the repo.
- **Docker + Docker Compose v2** — PostgreSQL, MeiliSearch, and vLLM run in containers.
  The [`justfile`](justfile) uses the v2 syntax (`docker compose`).
- **Node.js >= 18** — required by the integrated `luneth` crawler for Playwright-based
  browser automation (the bridge JS is embedded in the binary, Node runs at runtime).
- **Rust tooling** used by the `justfile` and pre-commit hooks:

  ```bash
  cargo install just                 # task runner (runs the justfile recipes)
  cargo install cargo-watch          # hot reload during development
  cargo install --locked cargo-nextest  # test runner
  cargo install sea-orm-cli          # migrations / codegen
  cargo install --locked cargo-deny  # dependency audit
  cargo install typos-cli            # spell check
  cargo install git-cliff            # changelog generation (release flow)
  ```

- **pre-commit**:

  ```bash
  brew install pre-commit   # or: pip install --user pre-commit
  pre-commit install
  ```

### Clone with submodules

From scratch:

```bash
git clone --recurse-submodules git@github.com:goodpeanuts/lunirelust.git
```

For an existing clone that was taken without `--recurse-submodules`:

```bash
git submodule update --init --recursive
```

> If `git submodule update` ever fails with
> `fatal: remote error: upload-pack: not our ref <sha>`, the superproject is pointing at
> a submodule commit that was never pushed. Jump to
> [Troubleshooting](#troubleshooting-and-common-pitfalls).

### Configure the environment

There is no root `.env.example`; the canonical template lives at
[`subm/luna/.env.example`](subm/luna/.env.example). Copy it to the repo root and adapt
the hostnames for local development — the deploy template uses Docker network hostnames
(`luna-db`, `luna-meilisearch`, `luna-vllm`), so for local dev switch them to
`localhost`:

```bash
cp subm/luna/.env.example .env
```


`.env` drives the dev and build stacks; `.env.test` drives the test stack. They are
nearly identical except for ports/project names.

## Running locally

The [`justfile`](justfile) provides recipes to start the dev stack (infrastructure +
migrations + hot-reload) and to stop it.

Once running:

- API base: `http://${SERVICE_HOST}:${SERVICE_PORT}`
- Swagger UI / OpenAPI docs: `http://${SERVICE_HOST}:${SERVICE_PORT}/docs`

> Swagger UI is served only when the `swagger` cargo feature is enabled. A plain
> `cargo run` (without the feature) will **not** expose `/docs`.

### Full local stack (with vLLM)

For the complete stack (app + PostgreSQL + MeiliSearch + vLLM serving the embedding
model), the [`justfile`](justfile) provides recipes to build and start it, and to stop
it. Pinned image tags and vLLM resource/CLI tuning live in
[`docker-compose.build.yml`](docker-compose.build.yml).

## Database and migrations

Migrations are managed by the `migration` workspace member (SeaORM); the
[`justfile`](justfile) provides recipes to apply and roll them back.

Pending migrations **also apply automatically on application startup** (see
[`src/common/config.rs`](src/common/config.rs)), so starting the dev server keeps the
schema current without a separate step.

For advanced operations (status, fresh, reset, generate, db-reset, backup, ...), use the
[`migrate.sh`](migrate.sh) helper — run it with no argument to list every subcommand,
and see the script header for what each does.

## Testing and quality

The [`justfile`](justfile) provides recipes to bring up the test stack, run the test
suite, and run the lint / dependency / typo gates. The search integration tests require
a live MeiliSearch instance and auto-skip when it is unreachable, so running the suite
without the compose stack does not fail.

### Pre-commit hooks

The hooks (configured in [`.pre-commit-config.yaml`](.pre-commit-config.yaml)) enforce
repo hygiene, formatting, linting, dependency, and typo gates across the workspace.
They do **not** replace the test suite — run the full suite via the justfile instead.
Run them on demand:

```bash
pre-commit run --all-files
```

The project keeps a strict lint stance (defined in the `Cargo.toml` workspace lints and
enforced by CI).


## Contribution workflow

### Branch model

- **Superproject**: default branch `master`. Work on topic branches such as `fix/...`,
  `feat/...`, or `dev/...`, and integrate via pull requests. Most history is linear
  (squash merges), with the PR number recorded in the merge title — e.g. branch
  `fix/vector-backfill` became `Fix/vector backfill (#6)`.
- **Submodule** (`subm`): default branch `main`, with a long-lived `dev` branch.
  History there is mostly direct Conventional-Commits pushes.

### Commit message conventions

The project uses Conventional Commits, in English. A `<type>: <summary>` subject,
optionally with a scope, and a bulleted body when the change is non-trivial. Types and
scopes observed in history:

- Types: `feat`, `fix`, `chore`, `perf`.
- Scopes (optional, in parens): `(crawl)`, `(backup)`, `(record)` — most commits omit
  the scope.

Examples from the actual logs:

```
fix: update luneth dependency to latest commit
feat(crawl): update API version to 1.0.0 and add append_page_path toggle for pagination
fix(backup): enhance restore functionality to handle separate snapshots for assets and database
perf: Resolve server performance bottlenecks and deployment loading issues ...
```

For larger changes, follow the bulleted-body format in
[`subm/docs/template/commit-message-template.md`](subm/docs/template/commit-message-template.md):
one summary sentence per module, no file paths, no business-domain jargon, no
self-promotion.

## Working with the submodule

The superproject stores `subm` as a raw commit SHA (a "gitlink"), not a branch name:
`git ls-tree HEAD subm` prints `160000 commit <sha> subm`. That SHA **must exist on the
submodule's remote**, otherwise any fresh clone fails at `git submodule update`.

### Avoid dangling commits

`git submodule update` checks out the recorded SHA, leaving the submodule in **detached
HEAD**. Commits made there advance only `HEAD` — no branch — so they become unreachable
from any ref and `git push` will not send them. If you then record that SHA in the
superproject, it points at a commit that exists only on your machine. Two habits prevent
this: **always check out a branch before editing the submodule, and push the submodule
before you record its SHA in the superproject.**

### Make a change in the submodule

```bash
cd subm
git checkout main            # never commit in detached HEAD
git pull
# ... edit, commit (Conventional Commits, English) ...
git push origin main         # push BEFORE recording the SHA in the superproject
git rev-parse HEAD           # this is the SHA the parent will record
cd ..

# defensive check: must list origin/main; empty output = STOP and push the submodule first
git -C subm branch -r --contains $(git -C subm rev-parse HEAD)

git add subm && git commit -m "chore: update submodule to latest commit"
git push origin master
```

### Pull submodule updates from others

```bash
git pull --recurse-submodules
```


## Changelog and release

Both processes are documented elsewhere; this section only points to the sources of
truth.

- **Changelog** — follow [`subm/docs/template/change-history-template.md`](subm/docs/template/change-history-template.md)
  and **prepend** each entry to [`subm/docs/changelog.md`](subm/docs/changelog.md). The
  template (and `subm/CLAUDE.md`) define the language and heading rules.
- **Release** — the full flow (version bump, tag, CI build/release/docker, deploy) lives
  in [`subm/README.md`](subm/README.md).


## Troubleshooting and common pitfalls

- **`git submodule update` fails with `upload-pack: not our ref <sha>`** — the
  superproject points at a submodule commit that was never pushed (typically made in
  detached HEAD). Diagnose and fix per
  [Working with the submodule](#working-with-the-submodule). The defensive check there
  prevents it.
- **`/docs` (Swagger) is not served** — the `swagger` cargo feature must be enabled; a
  plain `cargo run` (without it) will not expose `/docs`. See [Running locally](#running-locally).
- **Search integration tests silently skip** — expected when no live MeiliSearch is
  running. Use the justfile to bring up the `.env.test` stack.
- **Pre-commit inside the submodule may fail on the cargo hooks** —
  [`subm/.pre-commit-config.yaml`](subm/.pre-commit-config.yaml) runs the Rust hooks
  with `cd lustools/lust`, but the actual crate lives at `subm/lust/`. If you run
  pre-commit from within the submodule, correct that path first. (The superproject's
  own [`.pre-commit-config.yaml`](.pre-commit-config.yaml) is the canonical one and is
  not affected.)
- **"My submodule commits vanished after I switched branches"** — they were made in
  detached HEAD and are now only reachable via `git -C subm reflog`. Recover them, put
  them on `main`, and push — see the correct sequence above.
