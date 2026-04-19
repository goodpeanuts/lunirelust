//! Search domain module: unified full-text + vector search with Meilisearch.
//!
//! Clippy's `allow_attributes` lint fires on `#[expect(dead_code)]` annotations
//! used for API-contract scaffolding (batch embedding, entity types, sync events).
//! These items are intentionally kept for future feature evolution.
#![allow(clippy::allow_attributes)]

pub mod constants;

mod api {
    mod handlers {
        pub mod search_handler;
    }
    pub mod routes;
}

mod domain {
    pub mod model {
        pub mod search_document;
    }
    pub mod repository {
        pub mod outbox_repo;
        pub mod search_repo;
        pub mod tombstone_repo;
    }
    pub mod service {
        pub mod search_service;
    }

    pub use service::search_service::SearchServiceTrait;
}

pub mod dto {
    mod search_dto;
    pub use search_dto::*;
}

mod infra {
    pub mod meilisearch {
        pub(super) mod index_setup;
        pub mod meilisearch_client;
        pub mod meilisearch_repo;
    }
    pub mod embedding {
        pub mod embedding_client;
        pub mod embedding_service;
    }
    pub mod impl_service {
        pub(super) mod filter_utils;
        pub(super) mod rrf;
        pub mod search_service_impl;
        pub(super) mod sql_fallback;
    }
    pub mod indexer {
        pub(super) mod event_processor;
        pub(super) mod full_sync;
        pub mod indexer_service;
        pub(super) mod reconciliation;
    }
    pub mod outbox_repo_impl;
    pub mod tombstone_repo_impl;
}

// Re-export commonly used items
pub use api::routes::{search_routes, SearchApiDoc};
pub use domain::model::search_document::SearchEntityType;
pub use domain::repository::outbox_repo::OutboxRepository;
pub use domain::repository::tombstone_repo::TombstoneRepository;
pub use domain::SearchServiceTrait;
pub use infra::impl_service::search_service_impl::SearchService;
pub use infra::outbox_repo_impl::OutboxRepo;
pub use infra::tombstone_repo_impl::TombstoneRepo;
