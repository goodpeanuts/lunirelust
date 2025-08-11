use crate::domains::luna::domain::{
    DirectorServiceTrait, GenreServiceTrait, IdolServiceTrait, LabelServiceTrait, LunaServiceTrait,
    RecordServiceTrait, SeriesServiceTrait, StudioServiceTrait,
};
use async_trait::async_trait;
use sea_orm::DatabaseConnection;
use std::sync::Arc;

mod director;
mod genre;
mod idol;
mod label;
mod record;
mod series;
mod studio;

/// Combined Luna service that includes all domain services.
#[derive(Clone)]
pub struct LunaService {
    pub director_service: Arc<dyn DirectorServiceTrait>,
    pub genre_service: Arc<dyn GenreServiceTrait>,
    pub label_service: Arc<dyn LabelServiceTrait>,
    pub studio_service: Arc<dyn StudioServiceTrait>,
    pub series_service: Arc<dyn SeriesServiceTrait>,
    pub idol_service: Arc<dyn IdolServiceTrait>,
    pub record_service: Arc<dyn RecordServiceTrait>,
}

#[async_trait]
impl LunaServiceTrait for LunaService {
    /// Constructor for the service.
    fn create_service(db: DatabaseConnection) -> Arc<dyn LunaServiceTrait> {
        Arc::new(Self {
            director_service: director::DirectorService::create_service(db.clone()),
            genre_service: genre::GenreService::create_service(db.clone()),
            label_service: label::LabelService::create_service(db.clone()),
            studio_service: studio::StudioService::create_service(db.clone()),
            series_service: series::SeriesService::create_service(db.clone()),
            idol_service: idol::IdolService::create_service(db.clone()),
            record_service: record::RecordService::create_service(db),
        })
    }

    /// Get director service
    fn director_service(&self) -> &dyn DirectorServiceTrait {
        &*self.director_service
    }

    /// Get genre service
    fn genre_service(&self) -> &dyn GenreServiceTrait {
        &*self.genre_service
    }

    /// Get label service
    fn label_service(&self) -> &dyn LabelServiceTrait {
        &*self.label_service
    }

    /// Get studio service
    fn studio_service(&self) -> &dyn StudioServiceTrait {
        &*self.studio_service
    }

    /// Get series service
    fn series_service(&self) -> &dyn SeriesServiceTrait {
        &*self.series_service
    }

    /// Get idol service
    fn idol_service(&self) -> &dyn IdolServiceTrait {
        &*self.idol_service
    }

    /// Get record service
    fn record_service(&self) -> &dyn RecordServiceTrait {
        &*self.record_service
    }
}
