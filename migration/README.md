# Database Migrations

The migration crate owns versioned PostgreSQL schema changes. The ordered list in
[src/lib.rs](src/lib.rs) is authoritative; `src/entities/` in the parent repository
contains the application models. Add a registered forward migration when changing
the schema, and keep models consistent with it.

The application opens a validated database connection and never applies migrations
on startup. The local development, disposable test, and private deployment scripts
invoke migrations explicitly.

From the repository root:

```bash
just migrate-status
just migrate-up
```

These recipes load the development environment through the safe loader.
`./migrate.sh status` and `./migrate.sh up` are compatibility entry points; a missing
command is rejected. `just test` creates an isolated test database, applies all
migrations, runs the suite, and removes its disposable resources.

The migration binary requires `APP_ENV` and a matching database identity. It
validates the protocol, host, port, user, and database before connecting. Destructive
commands such as `down`, `fresh`, `refresh`, and `reset` are forbidden in production.
Do not bypass the loader or point development/test commands at a production URL.

Use the SeaORM CLI to generate a migration file when needed:

```bash
sea-orm-cli migrate generate change_name
```

Review the generated file and registration, then verify forward application on an
isolated database. Deployment runs a one-off migration container after creating and
listing a pre-migration dump; restoring application images does not reverse schema
changes. See the [private deployment guide](../subm/luna/README.md).
