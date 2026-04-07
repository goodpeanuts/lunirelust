use super::handlers::{
    __path_create_device, __path_delete_device, __path_get_device_by_id, __path_get_devices,
    __path_update_device, __path_update_many_devices, create_device, delete_device,
    get_device_by_id, get_devices, update_device, update_many_devices,
};
use crate::{
    common::app_state::AppState,
    domains::device::dto::device_dto::{CreateDeviceDto, DeviceDto, UpdateDeviceDto},
};
use axum::{
    routing::{delete, get, post, put},
    Router,
};

use utoipa::OpenApi;

use crate::common::openapi::SecurityAddon;

#[derive(OpenApi)]
#[openapi(
    paths(
        get_device_by_id,
        get_devices,
        create_device,
        update_device,
        update_many_devices,
        delete_device,
    ),
    components(schemas(DeviceDto, CreateDeviceDto, UpdateDeviceDto)),
    tags(
        (name = "Device", description = "Device management endpoints")
    ),
    security(
        ("bearer_auth" = [])
    ),
    modifiers(&SecurityAddon)
)]
/// This struct is used to generate `OpenAPI` documentation for the device routes.
pub struct DeviceApiDoc;

/// This function creates a router for the device routes.
/// It defines the routes and their corresponding handlers.
pub fn device_routes() -> Router<AppState> {
    Router::new()
        .route("/", get(get_devices))
        .route("/", post(create_device))
        .route("/{id}", get(get_device_by_id))
        .route("/{id}", put(update_device))
        .route("/{id}", delete(delete_device))
        .route("/batch/{user_id}", put(update_many_devices))
}
