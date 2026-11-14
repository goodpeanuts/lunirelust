# Contributor Instructions

Read [DEVELOPMENT.md](DEVELOPMENT.md) for the supported commands and environment
boundaries. The [justfile](justfile), source code, and migrations define current
behavior.

- Write new code comments, documentation, and commit messages in English. Use
  Chinese names for new documentation files; retain established entry-point names.
- Follow the workspace lints and the directory-plus-same-name-file Rust module
  layout. Avoid adding `mod.rs` files.
- Preserve database environment guards. Use `just test` for disposable integration
  tests and `just check` for static verification. The app does not migrate itself.
- Keep secrets, private host details, production configuration, and real data out of
  public files, logs, fixtures, and build contexts. Production operations require
  the user's explicit scope; tests must use isolated resources.
- Keep changes focused and document actual behavior and validation. OpenSpec is
  retired; historical proposals and checklists are not implementation requirements.
- Treat `subm` as a separate private repository. Read its
  [index](subm/README.md) and [local instructions](subm/CLAUDE.md) when working there.
  Push private commits before publishing parent gitlinks that reference them.
