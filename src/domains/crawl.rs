mod api {
    mod handlers;
    pub mod routes;
}

mod domain {
    pub mod model;
    pub mod repository;
    pub mod service;
}

pub mod dto {
    pub mod task_dto;
}

pub(crate) mod infra {
    pub mod crawler;
    pub mod impl_repository;
    pub mod impl_service;
    pub mod luneth_crawler;
}

pub use infra::impl_repository::CrawlRepo;

pub use api::routes::{crawl_routes, CrawlApiDoc};
pub use domain::repository::CrawlTaskRepository;
pub use domain::service::{CrawlServiceTrait, CrawlerTrait};
pub use infra::crawler::CrawlTaskManager;
pub use infra::impl_service::CrawlService;
