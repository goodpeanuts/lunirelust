use super::environment::{validate_database_target, AppEnv, DatabaseOperation, EnvironmentError};
use regex::Regex;
use sea_orm::{ConnectOptions, DatabaseConnection, DbErr};
use std::env;
use std::time::Duration;
use thiserror::Error;
use tokio::time::sleep;

/// Default page size for all paginated list endpoints.
/// Used when no `limit` query parameter is provided.
pub const DEFAULT_PAGE_SIZE: u64 = 20;

/// Default maximum file upload size (50 MB).
pub const DEFAULT_MAX_FILE_SIZE: usize = 50 * 1024 * 1024;

/// Database connection/acquire timeout in seconds.
const DB_CONNECT_TIMEOUT_SECS: u64 = 30;

/// Database idle timeout in seconds.
const DB_IDLE_TIMEOUT_SECS: u64 = 600;

/// Database max lifetime in seconds.
const DB_MAX_LIFETIME_SECS: u64 = 1800;

/// Config is a struct that holds the configuration for the application.
#[derive(Clone)]
pub struct Config {
    pub app_env: AppEnv,
    pub database_url: String,
    pub database_max_connections: u32,
    pub database_min_connections: u32,

    pub service_host: String,
    pub service_port: String,

    pub assets_public_path: String,
    pub assets_public_url: String,

    pub assets_private_path: String,
    pub assets_private_url: String,

    pub asset_allowed_extensions_pattern: Regex,
    pub asset_allowed_extensions: Vec<String>,
    pub asset_max_size: usize,

    pub cors_origins: Vec<String>,

    // MeiliSearch configuration
    pub meili_url: String,
    pub meili_master_key: String,

    // vLLM embedding configuration
    pub vllm_embedding_url: String,
    pub vllm_embedding_model: String,
    pub vllm_embedding_timeout_secs: u64,
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error(transparent)]
    Environment(#[from] EnvironmentError),

    #[error("required environment variable {name} is not set")]
    MissingVariable { name: &'static str },
}

/// Errors raised while preparing a database connection for integration tests.
#[derive(Debug, Error)]
pub enum TestDatabaseSetupError {
    /// Integration tests may only run with the fixed test environment identity.
    #[error(
        "database integration tests require APP_ENV=test; got APP_ENV={actual}; run `just test`"
    )]
    WrongEnvironment {
        /// Environment supplied by the caller.
        actual: AppEnv,
    },

    /// The configured test database target failed identity validation.
    #[error(transparent)]
    Environment(#[from] EnvironmentError),

    /// The validated test database could not be reached.
    #[error(transparent)]
    Database(#[from] DbErr),
}

fn required_var(name: &'static str) -> Result<String, ConfigError> {
    env::var(name).map_err(|_error| ConfigError::MissingVariable { name })
}

/// Read configuration from an environment loaded by an explicit entrypoint.
impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        let app_env = AppEnv::from_env()?;
        let database_url = required_var("DATABASE_URL")?;
        validate_database_target(app_env, &database_url, DatabaseOperation::Connect)?;
        let ext_val = required_var("ASSET_ALLOWED_EXTENSIONS")?;

        let asset_allowed_extensions: Vec<String> =
            ext_val.split('|').map(|s| s.to_lowercase()).collect();

        Ok(Self {
            app_env,
            database_url,

            database_max_connections: env::var("DATABASE_MAX_CONNECTIONS")
                .map(|s| s.parse::<u32>().unwrap_or(20))
                .unwrap_or(20),
            database_min_connections: env::var("DATABASE_MIN_CONNECTIONS")
                .map(|s| s.parse::<u32>().unwrap_or(5))
                .unwrap_or(5),

            service_host: required_var("SERVICE_HOST")?,
            service_port: required_var("SERVICE_PORT")?,

            assets_public_path: required_var("ASSETS_PUBLIC_PATH")?,
            assets_public_url: required_var("ASSETS_PUBLIC_URL")?,

            assets_private_path: required_var("ASSETS_PRIVATE_PATH")?,
            assets_private_url: required_var("ASSETS_PRIVATE_URL")?,

            asset_allowed_extensions_pattern: Regex::new(&format!(r"(?i)^.*\.({ext_val})$"))
                .unwrap_or_else(|_| {
                    eprintln!("Invalid ASSET_ALLOWED_EXTENSIONS regex pattern: {ext_val}");
                    Regex::new(r"(?i)^.*\.(jpg|jpeg|png|gif|webp)$")
                        .expect("Failed to compile default asset extensions regex")
                }),

            asset_allowed_extensions,

            asset_max_size: required_var("ASSET_MAX_SIZE")?
                .parse::<usize>()
                .unwrap_or(DEFAULT_MAX_FILE_SIZE),

            cors_origins: env::var("CORS_ORIGINS")
                .map(|s| s.split(',').map(|o| o.trim().to_owned()).collect())
                .unwrap_or_default(),

            meili_url: env::var("MEILI_URL").unwrap_or_else(|_| "http://localhost:7700".to_owned()),
            meili_master_key: env::var("MEILI_MASTER_KEY")
                .unwrap_or_else(|_| "meili_master_key_dev".to_owned()),
            vllm_embedding_url: env::var("VLLM_EMBEDDING_URL")
                .unwrap_or_else(|_| "http://localhost:8000".to_owned()),
            vllm_embedding_model: env::var("VLLM_EMBEDDING_MODEL")
                .unwrap_or_else(|_| "BAAI/bge-m3".to_owned()),
            vllm_embedding_timeout_secs: env::var("VLLM_EMBEDDING_TIMEOUT")
                .map(|s| s.parse::<u64>().unwrap_or(5))
                .unwrap_or(5),
        })
    }
}

/// `setup_database` initializes the database connection pool.
pub async fn setup_database(config: &Config) -> Result<DatabaseConnection, sea_orm::DbErr> {
    // Attempt to connect repeatedly, with a small delay, until success (or a max number of tries)
    let mut attempts = 0;
    let mut opt = ConnectOptions::new(&config.database_url);
    opt.min_connections(config.database_min_connections)
        .max_connections(config.database_max_connections)
        .connect_timeout(Duration::from_secs(DB_CONNECT_TIMEOUT_SECS))
        .acquire_timeout(Duration::from_secs(DB_CONNECT_TIMEOUT_SECS))
        .idle_timeout(Duration::from_secs(DB_IDLE_TIMEOUT_SECS))
        .max_lifetime(Duration::from_secs(DB_MAX_LIFETIME_SECS));
    // .sqlx_logging(true)
    // .sqlx_logging_level(tracing::Level::INFO);

    let pool = loop {
        attempts += 1;
        match sea_orm::Database::connect(opt.clone()).await {
            Ok(pool) => break pool,
            Err(err) => {
                if attempts >= 3 {
                    return Err(err);
                }
                eprintln!("Postgres not ready yet, retrying in 1s… (attempt {attempts}/3)");
                sleep(Duration::from_secs(1)).await;
            }
        }
    };

    Ok(pool)
}

/// Connect to the database only when the fixed integration-test identity is selected.
///
/// This guard deliberately runs before [`setup_database`] so a test invoked outside
/// `just test` cannot open a development or production database connection, even when
/// that target is otherwise valid for its declared environment.
pub async fn setup_test_database(
    config: &Config,
) -> Result<DatabaseConnection, TestDatabaseSetupError> {
    if config.app_env != AppEnv::Test {
        return Err(TestDatabaseSetupError::WrongEnvironment {
            actual: config.app_env,
        });
    }

    validate_database_target(
        AppEnv::Test,
        &config.database_url,
        DatabaseOperation::Connect,
    )?;

    setup_database(config)
        .await
        .map_err(TestDatabaseSetupError::Database)
}

#[cfg(test)]
mod tests {
    use regex::Regex;

    use super::{setup_test_database, Config, TestDatabaseSetupError, DEFAULT_MAX_FILE_SIZE};
    use crate::common::environment::AppEnv;

    fn config_for(app_env: AppEnv, database_url: &str) -> Config {
        Config {
            app_env,
            database_url: database_url.to_owned(),
            database_max_connections: 1,
            database_min_connections: 0,
            service_host: "127.0.0.1".to_owned(),
            service_port: "3000".to_owned(),
            assets_public_path: "/tmp/public".to_owned(),
            assets_public_url: "/public".to_owned(),
            assets_private_path: "/tmp/private".to_owned(),
            assets_private_url: "/private".to_owned(),
            asset_allowed_extensions_pattern: Regex::new(r"(?i)^.*\.(jpg|png)$")
                .expect("test asset extension regex must compile"),
            asset_allowed_extensions: vec!["jpg".to_owned(), "png".to_owned()],
            asset_max_size: DEFAULT_MAX_FILE_SIZE,
            cors_origins: vec![],
            meili_url: "http://127.0.0.1:7700".to_owned(),
            meili_master_key: "test-key".to_owned(),
            vllm_embedding_url: "http://127.0.0.1:8000".to_owned(),
            vllm_embedding_model: "BAAI/bge-m3".to_owned(),
            vllm_embedding_timeout_secs: 5,
        }
    }

    #[tokio::test]
    async fn test_setup_rejects_valid_dev_and_prod_identities_before_connecting() {
        let cases = [
            (
                AppEnv::Dev,
                "postgres://luna_dev:dev-secret@127.0.0.1:5434/luna_dev",
            ),
            (
                AppEnv::Prod,
                "postgres://luna_user:prod-secret@luna-db:5432/luna_db",
            ),
        ];

        for (app_env, database_url) in cases {
            let error = match setup_test_database(&config_for(app_env, database_url)).await {
                Ok(_connection) => panic!("integration test guard accepted APP_ENV={app_env}"),
                Err(error) => error,
            };

            assert!(matches!(
                &error,
                TestDatabaseSetupError::WrongEnvironment { actual } if *actual == app_env
            ));
            let message = error.to_string();
            assert!(message.contains("run `just test`"));
            assert!(!message.contains("secret"));
            assert!(!message.contains(database_url));
        }
    }
}
