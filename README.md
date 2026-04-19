# Lunirelust

A clean architecture web application built with Axum and SeaORM.

## Features

- Clean architecture with SeaORM for database operations
- Automated database migrations
- User and device management
- File upload functionality
- JWT authentication
- RESTful API with Swagger documentation

## Dev prepare

### Install tools

#### Cargo deny

```bash
cargo install --locked cargo-deny
```

#### Sea-orm-cli

SeaORM CLI for database operations and migrations:

```bash
cargo install sea-orm-cli
```

#### Git cliff

Git cliff is used to generate changelogs from commit messages.

```bash
cargo install git-cliff
```

#### just

Just is the task runner used for local development and test workflows.

```bash
cargo install just
```

#### cargo-watch

Cargo Watch is used by `just dev` for hot reload during backend development.

```bash
cargo install cargo-watch
```

#### Pre-commit

Pre-commit is a framework for managing and maintaining multi-language pre-commit hooks.

```bash
brew install pre-commit
```

setup pre-commit hooks:

```bash
pre-commit install
```

The test hook reuses `just test`, so local hook checks and manual test runs follow the same workflow.

#### typos

Typos is a spell checker for source code.

```bash
cargo install typos-cli
```

#### cargo nextest

Nextest is a boosted test runner for Rust.

```bash
cargo install cargo-nextest
```

## Database Setup

This project uses SeaORM with automated migrations and persistent PostgreSQL storage. The database data is stored in a local `db/` directory that persists across container restarts.

### Environment Configuration

Copy the example environment file and modify as needed:

```bash
cp .env.example .env
```

### Starting the Database

```bash
# Start PostgreSQL with persistent storage
docker-compose up postgres -d

# Check if database is ready
./migrate.sh test-connection
```

### Running Migrations

The application automatically runs migrations on startup, but you can also run them manually:

```bash
# Apply all pending migrations
./migrate.sh up

# Check migration status
./migrate.sh status

# Rollback last migration
./migrate.sh down

# Complete database reset (removes all data)
./migrate.sh db-reset

# Generate new migration
./migrate.sh generate your_migration_name
```

For detailed database management instructions, see [DATABASE.md](DATABASE.md).

## Development

### Running the Application

1. Start PostgreSQL database:
   ```bash
   docker-compose up postgres -d
   ```

2. Run the application:
   ```bash
   cargo run
   ```

3. The API will be available at `http://localhost:8080`
4. Swagger UI is available at `http://localhost:8080/swagger-ui/`

### Running with Docker

```bash
# Start all services
docker-compose up

# Start in detached mode
docker-compose up -d
```

For the full local stack used by search indexing and embeddings, run:

```bash
just build
```

This starts the app together with PostgreSQL, MeiliSearch, and a pinned `vllm/vllm-openai:v0.19.0` container. The vLLM service is configured with the current pooling-model CLI (`BAAI/bge-m3 --runner pooling`) instead of the removed `--task embed` flag, and caps GPU usage to about `3.6 GiB` on a `6 GiB` card via `--gpu-memory-utilization 0.6`, a shortened maximum sequence length (`--max-model-len 2048`), and reduced batching limits (`--max-num-batched-tokens 2048 --max-num-seqs 32`).

### Testing

```bash
# Prepare the test database and seed data
just test-prepare

# Run the default local test gate
# This prepares the test database, runs `cargo nextest`, and then runs doctests
just test

# Run the main Rust test suite only
just test-nextest

# Run doctests only
just test-doc

# Run all hooks with the same test flow used by pre-commit
pre-commit run --all-files
```

## Architecture

The project follows a clean architecture pattern:

- `src/entities/` - Database entities (generated from migrations)
- `src/domains/` - Business logic organized by domain
- `src/common/` - Shared utilities and configuration
- `migration/` - Database migration files

## API Documentation

Once the application is running, you can access the Swagger UI at:
`http://localhost:8080/swagger-ui/`
