#!/bin/bash

# Database migration script for SeaORM
# Usage: ./migrate.sh [command]
# Commands: up, down, status, fresh, reset, generate, db-reset

# Load environment variables from .env if it exists
if [ -f .env ]; then
    export $(cat .env | grep -v '#' | xargs)
fi

# Default DATABASE_URL if not set
if [ -z "$DATABASE_URL" ]; then
    export DATABASE_URL="postgres://testuser:pass@localhost:5432/testdb"
fi

COMMAND=${1:-up}

case $COMMAND in
    up)
        echo "Running migrations..."
        cargo run -p migration -- up
        ;;
    down)
        echo "Rolling back migrations..."
        cargo run -p migration -- down
        ;;
    status)
        echo "Checking migration status..."
        cargo run -p migration -- status
        ;;
    fresh)
        echo "Running fresh migrations (drop all tables and recreate)..."
        cargo run -p migration -- fresh
        ;;
    reset)
        echo "Resetting database (rollback all migrations)..."
        cargo run -p migration -- reset
        ;;
    generate)
        if [ -z "$2" ]; then
            echo "Usage: ./migrate.sh generate <migration_name>"
            exit 1
        fi
        echo "Generating migration: $2"
        sea-orm-cli migrate generate "$2"
        ;;
    db-reset)
        echo "Performing complete database reset..."
        echo "This will:"
        echo "1. Stop the database container"
        echo "2. Remove all database data"
        echo "3. Start fresh database"
        echo "4. Run migrations"
        echo ""
        read -p "Are you sure? (y/N): " -n 1 -r
        echo
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            echo "Stopping database..."
            docker-compose down postgres
            echo "Removing database data..."
            sudo rm -rf db/
            echo "Starting fresh database..."
            docker-compose up postgres -d
            echo "Waiting for database to be ready..."
            sleep 5
            echo "Running migrations..."
            cargo run -p migration -- up
            echo "Database reset complete!"
        else
            echo "Operation cancelled."
        fi
        ;;
    test-connection)
        echo "Testing database connection..."
        if command -v psql &> /dev/null; then
            psql "$DATABASE_URL" -c "SELECT version();"
        else
            echo "psql not found. Testing via Docker..."
            docker exec -it clean_axum_app_postgres psql -U testuser -d testdb -c "SELECT version();"
        fi
        ;;
    backup)
        BACKUP_FILE="backup_$(date +%Y%m%d_%H%M%S).sql"
        echo "Creating backup: $BACKUP_FILE"
        docker exec -t clean_axum_app_postgres pg_dump -U testuser testdb > "$BACKUP_FILE"
        echo "Backup created: $BACKUP_FILE"
        ;;
    *)
        echo "SeaORM Migration Manager"
        echo "======================="
        echo ""
        echo "Available commands:"
        echo "  up           - Apply pending migrations"
        echo "  down         - Rollback last migration"
        echo "  status       - Check migration status"
        echo "  fresh        - Drop all tables and reapply migrations"
        echo "  reset        - Rollback all migrations"
        echo "  generate     - Generate a new migration file"
        echo "  db-reset     - Complete database reset (removes data directory)"
        echo "  test-connection - Test database connection"
        echo "  backup       - Create database backup"
        echo ""
        echo "Usage: ./migrate.sh [command]"
        echo ""
        echo "Current DATABASE_URL: $DATABASE_URL"
        exit 1
        ;;
esac
