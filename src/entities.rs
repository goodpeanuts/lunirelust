//! `SeaORM` entities module
//!
//! This module contains all database entities generated from the database schema.

pub mod devices;
pub mod director;
pub mod genre;
pub mod idol;
pub mod idol_participation;
pub mod label;
pub mod links;
pub mod record;
pub mod record_genre;
pub mod search_document_versions;
pub mod search_sync_events;
pub mod series;
pub mod studio;
pub mod uploaded_files;
pub mod user_auth;
pub mod user_ext;
pub mod user_record_interaction;
pub mod users;

pub use devices::{DevicesEntity, DevicesModel};
pub use director::{DirectorEntity, DirectorModel};
pub use genre::{GenreEntity, GenreModel};
pub use idol::{IdolEntity, IdolModel};
pub use idol_participation::{IdolParticipationEntity, IdolParticipationModel};
pub use label::{LabelEntity, LabelModel};
pub use links::{LinksEntity, LinksModel};
pub use record::{RecordEntity, RecordModel};
pub use record_genre::{RecordGenreEntity, RecordGenreModel};
pub use search_document_versions::{SearchDocumentVersionsEntity, SearchDocumentVersionsModel};
pub use search_sync_events::{SearchSyncEventsEntity, SearchSyncEventsModel};
pub use series::{SeriesEntity, SeriesModel};
pub use studio::{StudioEntity, StudioModel};
pub use uploaded_files::{UploadedFilesEntity, UploadedFilesModel};
pub use user_auth::{UserAuthEntity, UserAuthModel};
pub use user_ext::{UserExtEntity, UserExtModel};
pub use user_record_interaction::{UserRecordInteractionEntity, UserRecordInteractionModel};
pub use users::{UsersEntity, UsersModel};
