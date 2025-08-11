//! This module defines service traits for luna (cards) domain entities,
//! responsible for business logic operations.

use async_trait::async_trait;
use sea_orm::DatabaseConnection;
use std::sync::Arc;

pub(super) mod director;
pub(super) mod genre;
pub(super) mod idol;
pub(super) mod label;
pub(super) mod record;
pub(super) mod series;
pub(super) mod studio;

#[async_trait]
/// Combined service trait that includes all luna domain services.
pub trait LunaServiceTrait: Send + Sync {
    /// Constructor for the service.
    fn create_service(db: DatabaseConnection) -> Arc<dyn LunaServiceTrait>
    where
        Self: Sized;

    /// Get director service
    fn director_service(&self) -> &dyn director::DirectorServiceTrait;

    /// Get genre service
    fn genre_service(&self) -> &dyn genre::GenreServiceTrait;

    /// Get label service
    fn label_service(&self) -> &dyn label::LabelServiceTrait;

    /// Get studio service
    fn studio_service(&self) -> &dyn studio::StudioServiceTrait;

    /// Get series service
    fn series_service(&self) -> &dyn series::SeriesServiceTrait;

    /// Get idol service
    fn idol_service(&self) -> &dyn idol::IdolServiceTrait;

    /// Get record service
    fn record_service(&self) -> &dyn record::RecordServiceTrait;
}
