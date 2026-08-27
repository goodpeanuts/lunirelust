//! Runtime environment and database target safety checks.

use std::{env, fmt};

use thiserror::Error;
use url::Url;

/// Runtime environments supported by this deployment.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppEnv {
    /// Persistent local development.
    Dev,
    /// Disposable automated tests.
    Test,
    /// The production container stack.
    Prod,
}

impl AppEnv {
    /// Read and validate `APP_ENV` from the process environment.
    pub fn from_env() -> Result<Self, EnvironmentError> {
        env::var("APP_ENV")
            .map_err(|_error| EnvironmentError::MissingAppEnv)?
            .parse()
    }
}

impl std::str::FromStr for AppEnv {
    type Err = EnvironmentError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "dev" => Ok(Self::Dev),
            "test" => Ok(Self::Test),
            "prod" => Ok(Self::Prod),
            other => Err(EnvironmentError::InvalidAppEnv(other.to_owned())),
        }
    }
}

impl fmt::Display for AppEnv {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Dev => "dev",
            Self::Test => "test",
            Self::Prod => "prod",
        })
    }
}

/// The class of database operation being authorized.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DatabaseOperation {
    /// Open a normal application or test connection.
    Connect,
    /// Inspect or apply forward migrations.
    Migrate,
    /// Roll back or recreate schema state.
    Destructive,
}

impl fmt::Display for DatabaseOperation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Connect => "connect",
            Self::Migrate => "migrate",
            Self::Destructive => "destructive migration",
        })
    }
}

/// A password-free representation of a parsed `PostgreSQL` target.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DatabaseTarget {
    scheme: String,
    host: String,
    port: u16,
    username: String,
    database: String,
}

impl DatabaseTarget {
    fn parse(database_url: &str) -> Result<Self, EnvironmentError> {
        let parsed =
            Url::parse(database_url).map_err(|_error| EnvironmentError::InvalidDatabaseUrl {
                reason: "the value is not an absolute URL",
            })?;

        if !matches!(parsed.scheme(), "postgres" | "postgresql") {
            return Err(EnvironmentError::InvalidDatabaseUrl {
                reason: "the scheme must be postgres or postgresql",
            });
        }

        let host = parsed
            .host_str()
            .filter(|value| !value.is_empty())
            .ok_or(EnvironmentError::InvalidDatabaseUrl {
                reason: "the host is missing",
            })?
            .to_ascii_lowercase();
        let username = parsed.username();
        if username.is_empty() {
            return Err(EnvironmentError::InvalidDatabaseUrl {
                reason: "the username is missing",
            });
        }

        let database = parsed.path().strip_prefix('/').unwrap_or_default();
        if database.is_empty() || database.contains('/') {
            return Err(EnvironmentError::InvalidDatabaseUrl {
                reason: "the database name is missing or invalid",
            });
        }

        Ok(Self {
            scheme: parsed.scheme().to_owned(),
            host,
            port: parsed.port().unwrap_or(5432),
            username: username.to_owned(),
            database: database.to_owned(),
        })
    }
}

impl fmt::Display for DatabaseTarget {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}://{}@{}:{}/{}",
            self.scheme, self.username, self.host, self.port, self.database
        )
    }
}

/// Errors raised before an unsafe database connection can be opened.
#[derive(Debug, Error)]
pub enum EnvironmentError {
    /// `APP_ENV` was not provided.
    #[error("APP_ENV is required; use one of: dev, test, prod")]
    MissingAppEnv,
    /// `APP_ENV` was not recognized.
    #[error("unsupported APP_ENV '{0}'; use one of: dev, test, prod")]
    InvalidAppEnv(String),
    /// `DATABASE_URL` was not provided.
    #[error("DATABASE_URL is required")]
    MissingDatabaseUrl,
    /// `DATABASE_URL` could not be interpreted safely.
    #[error("DATABASE_URL is invalid: {reason}")]
    InvalidDatabaseUrl {
        /// Password-free parse failure reason.
        reason: &'static str,
    },
    /// The parsed target does not belong to the selected environment.
    #[error(
        "unsafe database target for APP_ENV={app_env} during {operation}: {target}; expected {expected}"
    )]
    UnsafeDatabaseTarget {
        /// Selected application environment.
        app_env: AppEnv,
        /// Requested operation class.
        operation: DatabaseOperation,
        /// Sanitized target without a password.
        target: Box<DatabaseTarget>,
        /// Sanitized expected identity.
        expected: &'static str,
    },
    /// Production never permits destructive schema commands.
    #[error("destructive database migrations are forbidden when APP_ENV=prod")]
    ProductionDestructiveOperation,
}

/// Validate a database URL against the exact identity for an environment.
pub fn validate_database_target(
    app_env: AppEnv,
    database_url: &str,
    operation: DatabaseOperation,
) -> Result<DatabaseTarget, EnvironmentError> {
    if app_env == AppEnv::Prod && operation == DatabaseOperation::Destructive {
        return Err(EnvironmentError::ProductionDestructiveOperation);
    }

    let target = DatabaseTarget::parse(database_url)?;
    let accepted = match app_env {
        AppEnv::Dev => {
            target.username == "luna_dev"
                && target.database == "luna_dev"
                && ((matches!(target.host.as_str(), "127.0.0.1" | "localhost")
                    && target.port == 5434)
                    || (target.host == "postgres" && target.port == 5432))
        }
        AppEnv::Test => {
            target.username == "luna_test"
                && target.database == "luna_test"
                && matches!(target.host.as_str(), "127.0.0.1" | "localhost")
                && target.port == 5433
        }
        AppEnv::Prod => {
            target.username == "luna_user"
                && target.database == "luna_db"
                && target.host == "luna-db"
                && target.port == 5432
        }
    };

    if accepted {
        return Ok(target);
    }

    let expected = match app_env {
        AppEnv::Dev => "luna_dev@127.0.0.1:5434/luna_dev or luna_dev@postgres:5432/luna_dev",
        AppEnv::Test => "luna_test@127.0.0.1:5433/luna_test",
        AppEnv::Prod => "luna_user@luna-db:5432/luna_db",
    };

    Err(EnvironmentError::UnsafeDatabaseTarget {
        app_env,
        operation,
        target: Box::new(target),
        expected,
    })
}

/// Read and validate `APP_ENV` and `DATABASE_URL` from the process environment.
pub fn validate_database_target_from_env(
    operation: DatabaseOperation,
) -> Result<(AppEnv, DatabaseTarget), EnvironmentError> {
    let app_env = AppEnv::from_env()?;
    let database_url =
        env::var("DATABASE_URL").map_err(|_error| EnvironmentError::MissingDatabaseUrl)?;
    let target = validate_database_target(app_env, &database_url, operation)?;
    Ok((app_env, target))
}

#[cfg(test)]
mod tests {
    use super::{validate_database_target, AppEnv, DatabaseOperation, EnvironmentError};

    #[test]
    fn accepts_fixed_environment_targets() {
        let cases = [
            (
                AppEnv::Dev,
                "postgres://luna_dev:dev-secret@127.0.0.1:5434/luna_dev",
            ),
            (
                AppEnv::Dev,
                "postgresql://luna_dev:dev-secret@postgres:5432/luna_dev",
            ),
            (
                AppEnv::Test,
                "postgres://luna_test:test-secret@localhost:5433/luna_test",
            ),
            (
                AppEnv::Prod,
                "postgres://luna_user:prod-secret@luna-db:5432/luna_db",
            ),
        ];

        for (app_env, database_url) in cases {
            validate_database_target(app_env, database_url, DatabaseOperation::Connect)
                .expect("fixed target must be accepted");
        }
    }

    #[test]
    fn rejects_cross_environment_targets_without_leaking_passwords() {
        let production_url = "postgres://luna_user:do-not-leak@luna-db:5432/luna_db";
        let error =
            validate_database_target(AppEnv::Test, production_url, DatabaseOperation::Connect)
                .expect_err("test must reject production");
        let message = error.to_string();

        assert!(!message.contains("do-not-leak"));
        assert!(!message.contains(production_url));
        assert!(message.contains("luna_user@luna-db:5432/luna_db"));
    }

    #[test]
    fn rejects_invalid_environment_values() {
        let error = "staging"
            .parse::<AppEnv>()
            .expect_err("staging is deliberately not supported");
        assert!(matches!(error, EnvironmentError::InvalidAppEnv(_)));
    }

    #[test]
    fn rejects_invalid_or_incomplete_urls() {
        for value in [
            "not-a-url",
            "mysql://luna_test:secret@localhost:5433/luna_test",
            "postgres://localhost:5433/luna_test",
            "postgres://luna_test:secret@localhost:5433/",
        ] {
            assert!(
                validate_database_target(AppEnv::Test, value, DatabaseOperation::Connect).is_err()
            );
        }
    }

    #[test]
    fn rejects_every_mismatched_identity_component() {
        let cases = [
            (
                AppEnv::Dev,
                "postgres://wrong:secret@127.0.0.1:5434/luna_dev",
            ),
            (
                AppEnv::Dev,
                "postgres://luna_dev:secret@127.0.0.1:5434/wrong",
            ),
            (
                AppEnv::Dev,
                "postgres://luna_dev:secret@127.0.0.1:5432/luna_dev",
            ),
            (
                AppEnv::Dev,
                "postgres://luna_dev:secret@luna-db:5432/luna_dev",
            ),
            (
                AppEnv::Test,
                "postgres://wrong:secret@127.0.0.1:5433/luna_test",
            ),
            (
                AppEnv::Test,
                "postgres://luna_test:secret@127.0.0.1:5433/wrong",
            ),
            (
                AppEnv::Test,
                "postgres://luna_test:secret@127.0.0.1:5434/luna_test",
            ),
            (
                AppEnv::Test,
                "postgres://luna_test:secret@postgres:5432/luna_test",
            ),
            (AppEnv::Prod, "postgres://wrong:secret@luna-db:5432/luna_db"),
            (
                AppEnv::Prod,
                "postgres://luna_user:secret@luna-db:5432/wrong",
            ),
            (
                AppEnv::Prod,
                "postgres://luna_user:secret@luna-db:5433/luna_db",
            ),
            (
                AppEnv::Prod,
                "postgres://luna_user:secret@127.0.0.1:5432/luna_db",
            ),
        ];

        for (app_env, database_url) in cases {
            let error = validate_database_target(app_env, database_url, DatabaseOperation::Connect)
                .expect_err("a mismatched identity component must be rejected");
            let message = error.to_string();
            assert!(matches!(
                error,
                EnvironmentError::UnsafeDatabaseTarget { .. }
            ));
            assert!(!message.contains("secret"));
            assert!(!message.contains(database_url));
        }
    }

    #[test]
    fn test_environment_rejects_the_development_target() {
        let error = validate_database_target(
            AppEnv::Test,
            "postgres://luna_dev:secret@127.0.0.1:5434/luna_dev",
            DatabaseOperation::Connect,
        );
        assert!(error.is_err());
    }

    #[test]
    fn rejects_production_destructive_operations() {
        let error = validate_database_target(
            AppEnv::Prod,
            "postgres://luna_user:secret@luna-db:5432/luna_db",
            DatabaseOperation::Destructive,
        )
        .expect_err("production destructive operation must fail");
        assert!(matches!(
            error,
            EnvironmentError::ProductionDestructiveOperation
        ));
    }
}
