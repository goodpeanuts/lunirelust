use lunirelust::common::environment::{validate_database_target_from_env, DatabaseOperation};
use sea_orm_migration::prelude::*;
use std::{env, io};

fn requested_operation() -> Result<Option<DatabaseOperation>, io::Error> {
    let command = env::args().nth(1).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "migration command is required; use `up`, `status`, `down`, `fresh`, `reset`, `refresh`, or `generate`",
        )
    })?;

    match command.as_str() {
        "up" | "status" => Ok(Some(DatabaseOperation::Migrate)),
        "down" | "fresh" | "reset" | "refresh" => Ok(Some(DatabaseOperation::Destructive)),
        "generate" | "init" | "help" | "--help" | "-h" | "version" | "--version" | "-V" => Ok(None),
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("unsupported migration command: {command}"),
        )),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if let Some(operation) = requested_operation()? {
        validate_database_target_from_env(operation)?;
    }

    cli::run_cli(migration::Migrator).await;
    Ok(())
}
