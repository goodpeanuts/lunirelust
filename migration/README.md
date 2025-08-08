# SeaORM Database Migrations

This directory contains the SeaORM migration system for the lunirelust project.

## Overview

The migration system has been implemented to replace the manual SQL seed files with a programmatic approach using SeaORM migrations. This provides better version control, rollback capabilities, and integration with the Rust type system.

## Migration Files

The following migrations are currently implemented:

1. `m20250807_073903_create_users_table.rs` - Creates the users table
2. `m20250807_073910_create_devices_table.rs` - Creates the devices table with foreign key to users
3. `m20250807_073914_create_uploaded_files_table.rs` - Creates the uploaded_files table
4. `m20250807_073919_create_user_auth_table.rs` - Creates the user_auth table
5. `m20250807_074248_seed_initial_data.rs` - Seeds initial test data

## Running Migrations

### From project root directory

Use the convenient shell script:

```bash
# Apply all pending migrations
./migrate.sh up

# Rollback last migration
./migrate.sh down

# Check migration status
./migrate.sh status

# Drop all tables and reapply migrations
./migrate.sh fresh

# Rollback all migrations
./migrate.sh reset

# Generate a new migration file
./migrate.sh generate create_new_table
```

### Using cargo directly

```bash
# Apply all pending migrations
cargo run -p migration

# Or with specific commands
cargo run -p migration -- up
cargo run -p migration -- down
cargo run -p migration -- status
cargo run -p migration -- fresh
cargo run -p migration -- reset
```

### Generate new migrations

```bash
# Generate a new migration file
sea-orm-cli migrate generate MIGRATION_NAME
```

## Environment Variables

Make sure to set the `DATABASE_URL` environment variable:

```bash
export DATABASE_URL="postgres://username:password@localhost:5432/database_name"
```

Or create a `.env` file in the project root:

```env
DATABASE_URL=postgres://testuser:pass@localhost:5432/testdb
```

## Integration with Application

The migrations are automatically run when the application starts via the `setup_database` function in `src/common/config.rs`. This ensures the database schema is always up to date.

## Docker and CI/CD

The `docker-compose.yml` has been updated to remove the volume mount for SQL seed files, as migrations are now handled programmatically.

The CI pipeline (`.github/workflows/test.yml`) has been updated to run migrations instead of manually executing SQL files.

## Development Workflow

1. When adding new database tables or modifying existing ones:
   ```bash
   ./migrate.sh generate your_migration_name
   ```

2. Edit the generated migration file to implement the desired changes

3. Test the migration:
   ```bash
   ./migrate.sh up
   ```

4. If needed, test rollback:
   ```bash
   ./migrate.sh down
   ```

5. Commit the migration file to version control

## Troubleshooting

- If migrations fail, check the database connection and ensure PostgreSQL is running
- Use `./migrate.sh status` to see which migrations have been applied
- Use `./migrate.sh fresh` to start with a clean database state
- Check the SeaORM migration logs for detailed error messages

## Original CLI Commands

- Generate a new migration file
    ```sh
    cargo run -- generate MIGRATION_NAME
    ```
- Apply all pending migrations
    ```sh
    cargo run
    ```
    ```sh
    cargo run -- up
    ```
- Apply first 10 pending migrations
    ```sh
    cargo run -- up -n 10
    ```
- Rollback last applied migrations
    ```sh
    cargo run -- down
    ```
- Rollback last 10 applied migrations
    ```sh
    cargo run -- down -n 10
    ```
- Drop all tables from the database, then reapply all migrations
    ```sh
    cargo run -- fresh
    ```
- Rollback all applied migrations, then reapply all migrations
    ```sh
    cargo run -- refresh
    ```
- Rollback all applied migrations
    ```sh
    cargo run -- reset
    ```
- Check the status of all migrations
    ```sh
    cargo run -- status
    ```
