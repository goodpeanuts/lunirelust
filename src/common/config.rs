use migration::{Migrator, MigratorTrait as _};
use regex::Regex;
use sea_orm::{ConnectOptions, DatabaseConnection};
use std::env;
use std::time::Duration;
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
#[derive(Clone, Debug)]
pub struct Config {
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
}

/// `from_env` reads the environment variables and returns a Config struct.
/// It uses the dotenv crate to load environment variables from a .env file if it exists.
/// It returns a Result with the Config struct or an error if any of the environment variables are missing.
impl Config {
    pub fn from_env() -> Result<Self, env::VarError> {
        dotenvy::dotenv().ok();

        let ext_val = env::var("ASSET_ALLOWED_EXTENSIONS")?;

        let asset_allowed_extensions: Vec<String> =
            ext_val.split('|').map(|s| s.to_lowercase()).collect();

        Ok(Self {
            database_url: env::var("DATABASE_URL")?,

            database_max_connections: env::var("DATABASE_MAX_CONNECTIONS")
                .map(|s| s.parse::<u32>().unwrap_or(20))
                .unwrap_or(20),
            database_min_connections: env::var("DATABASE_MIN_CONNECTIONS")
                .map(|s| s.parse::<u32>().unwrap_or(5))
                .unwrap_or(5),

            service_host: env::var("SERVICE_HOST")?,
            service_port: env::var("SERVICE_PORT")?,

            assets_public_path: env::var("ASSETS_PUBLIC_PATH")?,
            assets_public_url: env::var("ASSETS_PUBLIC_URL")?,

            assets_private_path: env::var("ASSETS_PRIVATE_PATH")?,
            assets_private_url: env::var("ASSETS_PRIVATE_URL")?,

            asset_allowed_extensions_pattern: Regex::new(&format!(r"(?i)^.*\.({ext_val})$"))
                .unwrap_or_else(|_| {
                    eprintln!("Invalid ASSET_ALLOWED_EXTENSIONS regex pattern: {ext_val}");
                    Regex::new(r"(?i)^.*\.(jpg|jpeg|png|gif|webp)$")
                        .expect("Failed to compile default asset extensions regex")
                }),

            asset_allowed_extensions,

            asset_max_size: env::var("ASSET_MAX_SIZE")
                .map(|s| s.parse::<usize>().unwrap_or(DEFAULT_MAX_FILE_SIZE))?,

            cors_origins: env::var("CORS_ORIGINS")
                .map(|s| s.split(',').map(|o| o.trim().to_owned()).collect())
                .unwrap_or_default(),
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
                eprintln!(
                    "Postgres not ready yet ({:?}), retrying in 1s… (attempt {}/{})",
                    err, attempts, 3
                );
                sleep(Duration::from_secs(1)).await;
            }
        }
    };

    // Run pending migrations
    Migrator::up(&pool, None).await?;

    Ok(pool)
}
