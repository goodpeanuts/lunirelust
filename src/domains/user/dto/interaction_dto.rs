use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::ToSchema;

/// Response DTO for `toggle_like` endpoint.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ToggleLikeResponse {
    pub liked: bool,
}

/// Response DTO for `mark_viewed` endpoint.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MarkViewedResponse {
    pub viewed: bool,
}

/// Response DTO for interaction status of a single record.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct InteractionStatusDto {
    pub liked: bool,
    pub viewed: bool,
}

/// Request DTO for batch interaction status query.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BatchStatusRequestDto {
    pub record_ids: Vec<String>,
}

/// Response DTO for batch interaction status query, keyed by record ID.
pub type BatchStatusResponse = HashMap<String, InteractionStatusDto>;
