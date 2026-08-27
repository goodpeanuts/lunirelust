#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
cd "$repo_root"

legacy_dir="$(realpath -e "$repo_root/db")"
expected_legacy="$repo_root/db"
target_dir="$repo_root/.local/dev/postgres"
legacy_container="lunirelust-legacy-migration-db"
timestamp="${1:-$(date +%Y%m%d_%H%M%S)}"
migration_dir="$repo_root/.local/migration/$timestamp"

if [[ "$legacy_dir" != "$expected_legacy" ]]; then
    echo "legacy database path resolved outside the repository: $legacy_dir" >&2
    exit 1
fi
if [[ -d "$target_dir" && -n "$(find "$target_dir" -mindepth 1 -maxdepth 1 -print -quit)" ]]; then
    echo "refusing to overwrite non-empty dev database directory: $target_dir" >&2
    exit 1
fi
if docker container inspect "$legacy_container" >/dev/null 2>&1; then
    echo "temporary legacy container already exists: $legacy_container" >&2
    exit 1
fi

mkdir -p "$migration_dir" "$target_dir" "$repo_root/.local/dev/meilisearch"
chmod 0700 "$migration_dir"
if [[ -f .env ]]; then
    cp .env "$migration_dir/legacy.env"
    chmod 0600 "$migration_dir/legacy.env"
fi

stop_legacy() {
    docker stop "$legacy_container" >/dev/null 2>&1 || true
}
trap stop_legacy EXIT INT TERM

docker run --rm -d \
    --name "$legacy_container" \
    -p 127.0.0.1:5433:5432 \
    -e POSTGRES_USER=testuser \
    -e POSTGRES_PASSWORD=pass \
    -e POSTGRES_DB=testdb \
    -v "$legacy_dir:/var/lib/postgresql/data" \
    postgres:15 >/dev/null

for _ in {1..60}; do
    if docker exec "$legacy_container" pg_isready -U testuser -d testdb >/dev/null 2>&1; then
        break
    fi
    sleep 1
done
docker exec "$legacy_container" pg_isready -U testuser -d testdb >/dev/null
server_version="$(docker exec "$legacy_container" psql -U testuser -d testdb -Atqc "SHOW server_version_num")"
if [[ "$server_version" != 15* ]]; then
    echo "legacy database is not PostgreSQL 15: $server_version" >&2
    exit 1
fi

identity="$(docker exec "$legacy_container" psql -U testuser -d testdb -Atqc "SELECT current_user || '|' || current_database()")"
if [[ "$identity" != "testuser|testdb" ]]; then
    echo "legacy database identity mismatch: $identity" >&2
    exit 1
fi

dump_file="$migration_dir/legacy.dump"
docker exec "$legacy_container" pg_dump -U testuser -d testdb -Fc > "$dump_file"
chmod 0600 "$dump_file"
sha256sum "$dump_file" > "$migration_dir/legacy.dump.sha256"
docker run --rm -v "$migration_dir:/backup:ro" postgres:15 \
    pg_restore --list /backup/legacy.dump > "$migration_dir/legacy.restore-list"

source_tables="$migration_dir/source-table-rows.tsv"
: > "$source_tables"
while IFS= read -r table; do
    [[ "$table" =~ ^[A-Za-z_][A-Za-z0-9_]*$ ]] || { echo "unsafe table name: $table" >&2; exit 1; }
    rows="$(docker exec "$legacy_container" psql -U testuser -d testdb -Atqc "SELECT count(*) FROM public.\"$table\"")"
    printf '%s|%s\n' "$table" "$rows" >> "$source_tables"
done < <(docker exec "$legacy_container" psql -U testuser -d testdb -Atqc "SELECT tablename FROM pg_tables WHERE schemaname = 'public' ORDER BY tablename")
docker exec "$legacy_container" psql -U testuser -d testdb -Atqc \
    "SELECT version || '|' || applied_at FROM seaql_migrations ORDER BY version" \
    > "$migration_dir/source-migrations.tsv"

stop_legacy
trap - EXIT INT TERM

dev_compose=(./scripts/env-run.sh .env.example docker compose --project-name lunirelust-dev --env-file .env.example -f compose.dev.yml)
"${dev_compose[@]}" up -d --wait postgres
"${dev_compose[@]}" exec -T postgres pg_restore \
    -U luna_dev -d luna_dev \
    --no-owner --no-privileges --single-transaction --exit-on-error \
    < "$dump_file"
./scripts/env-run.sh .env.example cargo run --quiet -p migration -- up

target_tables="$migration_dir/target-table-rows.tsv"
: > "$target_tables"
while IFS= read -r table; do
    [[ "$table" =~ ^[A-Za-z_][A-Za-z0-9_]*$ ]] || { echo "unsafe table name: $table" >&2; exit 1; }
    rows="$("${dev_compose[@]}" exec -T postgres psql -U luna_dev -d luna_dev -Atqc "SELECT count(*) FROM public.\"$table\"")"
    printf '%s|%s\n' "$table" "$rows" >> "$target_tables"
done < <("${dev_compose[@]}" exec -T postgres psql -U luna_dev -d luna_dev -Atqc "SELECT tablename FROM pg_tables WHERE schemaname = 'public' ORDER BY tablename")
"${dev_compose[@]}" exec -T postgres psql -U luna_dev -d luna_dev -Atqc \
    "SELECT version || '|' || applied_at FROM seaql_migrations ORDER BY version" \
    > "$migration_dir/target-migrations.tsv"

while IFS='|' read -r table expected_rows; do
    actual_rows="$(awk -F'|' -v table="$table" '$1 == table { print $2 }' "$target_tables")"
    if [[ "$actual_rows" != "$expected_rows" ]]; then
        echo "row-count mismatch for $table: source=$expected_rows target=${actual_rows:-missing}" >&2
        exit 1
    fi
done < "$source_tables"

cut -d'|' -f1 "$migration_dir/source-migrations.tsv" | sort > "$migration_dir/source-migration-versions.txt"
cut -d'|' -f1 "$migration_dir/target-migrations.tsv" | sort > "$migration_dir/target-migration-versions.txt"
missing_versions="$(comm -23 "$migration_dir/source-migration-versions.txt" "$migration_dir/target-migration-versions.txt")"
if [[ -n "$missing_versions" ]]; then
    echo "one or more legacy migration versions are missing from dev" >&2
    exit 1
fi

rewrite_dev_env() {
    local old_env="$repo_root/.env"
    local new_env="$migration_dir/dev.env"
    declare -A old_values=() managed=() template_keys=()
    local line key value

    for key in APP_ENV LUNA_ENV COMPOSE_PROJECT_NAME DEPLOY_ENV_FILE POSTGRES_USER POSTGRES_PASSWORD POSTGRES_DB POSTGRES_PORT POSTGRES_DATA_DIR DATABASE_URL MEILI_PORT MEILI_DATA_DIR MEILI_URL; do
        managed["$key"]=1
    done

    if [[ -f "$old_env" ]]; then
        while IFS= read -r line || [[ -n "$line" ]]; do
            line="${line%$'\r'}"
            [[ -z "$line" || "$line" =~ ^[[:space:]]*# || "$line" != *=* ]] && continue
            key="${line%%=*}"
            value="${line#*=}"
            [[ "$key" =~ ^[A-Za-z_][A-Za-z0-9_]*$ ]] || continue
            if [[ "$value" == \"*\" && "$value" == *\" ]]; then value="${value:1:${#value}-2}"; fi
            old_values["$key"]="$value"
        done < "$old_env"
    fi

    : > "$new_env"
    while IFS= read -r line || [[ -n "$line" ]]; do
        if [[ -z "$line" || "$line" =~ ^[[:space:]]*# || "$line" != *=* ]]; then
            printf '%s\n' "$line" >> "$new_env"
            continue
        fi
        key="${line%%=*}"
        template_keys["$key"]=1
        if [[ -z "${managed[$key]:-}" && -n "${old_values[$key]+set}" ]]; then
            printf '%s=%s\n' "$key" "${old_values[$key]}" >> "$new_env"
        else
            printf '%s\n' "$line" >> "$new_env"
        fi
    done < .env.example

    for key in "${!old_values[@]}"; do
        if [[ -z "${managed[$key]:-}" && -z "${template_keys[$key]:-}" ]]; then
            printf '%s=%s\n' "$key" "${old_values[$key]}" >> "$new_env"
        fi
    done

    chmod 0600 "$new_env"
    mv "$new_env" "$old_env"
}

rewrite_dev_env
"${dev_compose[@]}" up -d --wait

echo "legacy data migrated and verified"
echo "backup and manifests: $migration_dir"
echo "old data retained: $legacy_dir and $repo_root/meili_data_test"
echo "run 'just dev' to start the backend and rebuild the Meilisearch index"
