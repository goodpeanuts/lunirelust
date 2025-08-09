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
pub mod series;
pub mod studio;
pub mod uploaded_files;
pub mod user_auth;
pub mod user_table;
pub mod users;

pub use devices::Entity as DevicesEntity;
pub use director::Entity as DirectorEntity;
pub use genre::Entity as GenreEntity;
pub use idol::Entity as IdolEntity;
pub use idol_participation::Entity as IdolParticipationEntity;
pub use label::Entity as LabelEntity;
pub use links::Entity as LinksEntity;
pub use record::Entity as RecordEntity;
pub use record_genre::Entity as RecordGenreEntity;
pub use series::Entity as SeriesEntity;
pub use studio::Entity as StudioEntity;
pub use uploaded_files::Entity as UploadedFilesEntity;
pub use user_auth::Entity as UserAuthEntity;
pub use user_table::Entity as UserTableEntity;
pub use users::Entity as UsersEntity;
