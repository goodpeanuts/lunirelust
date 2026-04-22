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
    pub mod interaction_dto;
    pub mod user_dto;
}

mod infra {
    pub mod impl_repository;
    pub mod impl_service;
}

// Re-export commonly used items for convenience
pub use api::routes::{user_routes, UserApiDoc};
pub use domain::service::interaction_service::InteractionServiceTrait;
pub use domain::service::user_service::UserServiceTrait;
pub use infra::impl_service::interaction_service::InteractionService;
pub use infra::impl_service::user_service::UserService;
