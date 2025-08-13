use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// DTO for media access request parameters
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MediaAccessDto {
    /// The unique identifier for the media
    pub id: String,
    /// Optional sequence number for the media file (e.g., 1, 2, 3...)
    /// If not provided, returns the default image (id.png)
    pub n: Option<u32>,
}

impl MediaAccessDto {
    /// Creates a new `MediaAccessDto`
    pub fn new(id: String, n: Option<u32>) -> Self {
        Self { id, n }
    }

    /// Generates the expected filename based on id and optional sequence number
    pub fn get_filename(&self) -> String {
        match self.n {
            Some(seq) => format!("{}-{}.png", self.id, seq),
            None => format!("{}.png", self.id),
        }
    }
}
