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

#### Pre-commit

Pre-commit is a framework for managing and maintaining multi-language pre-commit hooks.

```bash
brew install pre-commit
```

setup pre-commit hooks:

```bash
pre-commit install
```

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

### Testing

```bash
# Run tests
cargo test

# Run tests with nextest
cargo nextest run
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
