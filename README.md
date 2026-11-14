# Lunirelust

A Rust media-catalog API built with Axum, SeaORM, and PostgreSQL.

The backend provides JWT authentication, user/device/file management, catalog records
and related entities, likes and viewing history, search, and persistent crawl tasks
with progress events. The integrated `luneth` crawler uses Node.js and Playwright;
its browser starts when the crawl module is initialized.

Search uses PostgreSQL by default, including case-insensitive literal text matching,
related names, filters, and stable pagination. No external search worker or per-write
indexing events are needed in this mode. MeiliSearch keyword search and vLLM/BGE-M3
hybrid search remain explicit options. When re-enabled after PostgreSQL-only use,
the optional index is refreshed before serving results; unavailable search services
fall back to PostgreSQL. See the mode table in [DEVELOPMENT.md](DEVELOPMENT.md).

## Development

See [DEVELOPMENT.md](DEVELOPMENT.md) for prerequisites, environment isolation,
contribution rules, and verification. The [justfile](justfile) is the command catalog.

```bash
just dev   # Local infrastructure, explicit migration, native Rust server
just test  # Disposable integration-test environment
just ci    # Static checks and tests
```

The application itself never applies migrations. Development and deployment scripts
run them explicitly. Swagger UI is available at `/docs` with the `swagger` feature,
which `just dev` enables.

## Repository layout

| Path | Purpose |
| --- | --- |
| `src/domains/` | Auth, user, device, file, catalog (`luna`), search, and crawl domains |
| `src/entities/` | SeaORM models |
| `src/common/` | Configuration, database guards, HTTP handling, and shared utilities |
| `migration/` | Versioned schema changes; [migration guide](migration/README.md) |
| `scripts/`, `compose.*.yml` | Local development and isolated tests |
| `subm/` | Private documentation, deployment scripts, and the standalone `lust` CLI |

The public repository contains reusable source and non-secret examples. Private
deployment context and historical design records live in the access-controlled
[private documentation index](subm/README.md). Code, migrations, and executable
workflow files define current behavior; archived proposals do not.
