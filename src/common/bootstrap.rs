use std::sync::Arc;

use sea_orm::DatabaseConnection;
use tokio::sync::{broadcast, Mutex};

use crate::common::config::Config;
use crate::domains::auth::{AuthService, AuthServiceTrait};
use crate::domains::crawl::infra::crawler::RunnerCommand;
use crate::domains::crawl::{CrawlRepo, CrawlService, CrawlServiceTrait, CrawlTaskManager};
use crate::domains::device::{DeviceService, DeviceServiceTrait};
use crate::domains::file::{FileService, FileServiceTrait};
use crate::domains::luna::{
    infra::impl_service::file::FileService as LunaFileService, infra::RecordRepo, LunaService,
    LunaServiceTrait,
};
use crate::domains::search::{SearchService, SearchServiceTrait};
use crate::domains::user::{InteractionRepo, InteractionRepository, UserServiceTrait};
use crate::{common::app_state::AppState, domains::user::UserService};

use tracing_subscriber::{layer::SubscriberExt as _, util::SubscriberInitExt as _};

/// Constructs and wires all application services and returns a configured `AppState`.
pub fn build_app_state(pool: &DatabaseConnection, config: Config) -> AppState {
    let file_service: Arc<dyn FileServiceTrait> =
        FileService::create_service(config.clone(), pool.clone());
    let user_service: Arc<dyn UserServiceTrait> =
        UserService::create_service(pool.clone(), Arc::clone(&file_service));
    let auth_service: Arc<dyn AuthServiceTrait> =
        AuthService::create_service(pool.clone(), Arc::clone(&user_service));
    let device_service: Arc<dyn DeviceServiceTrait> = DeviceService::create_service(pool.clone());
    let luna_service: Arc<dyn LunaServiceTrait> =
        LunaService::create_service(config.clone(), pool.clone());
    let search_service: Arc<dyn SearchServiceTrait> =
        SearchService::create_service(config.clone(), pool.clone());

    // Crawl service wiring
    let interaction_repo: Arc<dyn InteractionRepository + Send + Sync> = Arc::new(InteractionRepo);
    let record_repo: Arc<dyn crate::domains::luna::RecordRepository + Send + Sync> =
        Arc::new(RecordRepo);
    let crawl_repo: Arc<dyn crate::domains::crawl::CrawlTaskRepository + Send + Sync> =
        Arc::new(CrawlRepo);

    let luna_file_service: Arc<dyn crate::domains::luna::FileServiceTrait + Send + Sync> =
        Arc::new(LunaFileService::new(config.clone()));

    let (broadcast_tx, _) = broadcast::channel(1024);
    let (runner_tx, runner_rx) = std::sync::mpsc::channel::<RunnerCommand>();

    let mut task_mgr = CrawlTaskManager::new(broadcast_tx);
    task_mgr.set_runner_tx(runner_tx);
    let task_manager = Arc::new(Mutex::new(task_mgr));

    let crawl_service: Arc<CrawlService> = Arc::new(CrawlService::new(
        pool.clone(),
        config.clone(),
        crawl_repo,
        interaction_repo,
        record_repo,
        luna_file_service,
        task_manager.clone(),
    ));

    let crawl_service_trait: Arc<dyn CrawlServiceTrait> = crawl_service.clone();

    // Spawn a dedicated thread with a LocalSet for !Send crawl futures.
    // The crawler lives exclusively on this thread.
    let crawl_svc_for_runner = crawl_service.clone();
    std::thread::Builder::new()
        .name("crawl-runner".to_owned())
        .spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to build crawl-runner runtime");
            let local = tokio::task::LocalSet::new();

            let crawler = match luneth::crawl::WebCrawler::new() {
                Ok(c) => crate::domains::crawl::infra::luneth_crawler::LunethCrawler::new(c),
                Err(e) => {
                    tracing::error!("Failed to create WebCrawler: {e}");
                    crate::domains::crawl::infra::luneth_crawler::LunethCrawler::new_noop()
                }
            };

            local.block_on(&rt, async move {
                while let Ok(cmd) = runner_rx.recv() {
                    match cmd {
                        RunnerCommand::Execute { task_id } => {
                            crawl_svc_for_runner
                                .dispatch_and_run(task_id, &crawler)
                                .await;
                        }
                        RunnerCommand::Shutdown => break,
                    }
                }
            });
        })
        .expect("Failed to spawn crawl-runner thread");

    AppState::new(
        config,
        auth_service,
        user_service,
        device_service,
        file_service,
        luna_service,
        search_service,
        crawl_service_trait,
    )
}

/// Setup tracing for the application.
pub fn setup_tracing() {
    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,sqlx=warn,tower_http=info,axum::rejection=trace".into()),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .with_file(true)
                .with_line_number(true)
                .with_thread_ids(true)
                .with_thread_names(true)
                .with_target(true)
                .with_span_events(tracing_subscriber::fmt::format::FmtSpan::CLOSE),
        )
        .init();
}

/// Shutdown signal handler
pub async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("Failed to install CTRL+C signal handler");
}
