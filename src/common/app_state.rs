use std::sync::Arc;

use crate::domains::{
    auth::AuthServiceTrait, device::DeviceServiceTrait, file::FileServiceTrait,
    luna::LunaServiceTrait, search::SearchServiceTrait, user::UserServiceTrait,
};

use super::config::Config;

/// `AppState` is a struct that holds the application-wide shared state.
/// It is passed to request handlers via Axum's extension mechanism.
#[derive(Clone)]
pub struct AppState {
    /// Global application configuration.
    pub config: Config,
    /// Service handling authentication-related logic.
    pub auth_service: Arc<dyn AuthServiceTrait>,
    /// Service handling user-related logic.
    pub user_service: Arc<dyn UserServiceTrait>,
    /// Service handling device-related logic.
    pub device_service: Arc<dyn DeviceServiceTrait>,
    /// Service handling file-related logic.
    pub file_service: Arc<dyn FileServiceTrait>,
    /// Service handling luna (cards) related logic.
    pub luna_service: Arc<dyn LunaServiceTrait>,
    /// Service handling search-related logic.
    pub search_service: Arc<dyn SearchServiceTrait>,
}

impl AppState {
    /// Creates a new instance of `AppState` with the provided dependencies.
    pub fn new(
        config: Config,
        auth_service: Arc<dyn AuthServiceTrait>,
        user_service: Arc<dyn UserServiceTrait>,
        device_service: Arc<dyn DeviceServiceTrait>,
        file_service: Arc<dyn FileServiceTrait>,
        luna_service: Arc<dyn LunaServiceTrait>,
        search_service: Arc<dyn SearchServiceTrait>,
    ) -> Self {
        Self {
            config,
            auth_service,
            user_service,
            device_service,
            file_service,
            luna_service,
            search_service,
        }
    }
}
