use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub enum MediaType {
    RecordImage,
    IdolImage,
}

impl MediaType {
    pub fn get_sub_dir_name(&self) -> String {
        match self {
            Self::RecordImage => "record".to_owned(),
            Self::IdolImage => "idol".to_owned(),
        }
    }
}

/// DTO for media access request parameters
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MediaAccessDto {
    /// The unique identifier for the media
    pub id: String,
    /// The type of the media (e.g., "image", "video")
    pub media_type: MediaType,
    /// Optional sequence number for the media file (e.g., 1, 2, 3...)
    /// If not provided, returns the default image (id.jpg)
    pub n: Option<u32>,
}

impl MediaAccessDto {
    /// Creates a new `MediaAccessDto`
    pub fn new(id: String, media_type: MediaType, n: Option<u32>) -> Self {
        Self { id, media_type, n }
    }

    /// Generates the expected filename based on id and optional sequence number
    /// Returns filename without extension (e.g., "`id_1`" or "id")
    pub fn get_filename(&self) -> String {
        match self.n {
            Some(seq) => format!("{}_{}", self.id, seq),
            None => self.id.clone(),
        }
    }
}
