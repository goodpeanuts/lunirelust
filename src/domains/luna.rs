mod api {
    mod handlers {
        mod director;
        mod genre;
        mod idol;
        mod label;
        mod record;
        mod series;
        mod statistics;
        mod studio;

        pub use director::*;
        pub use genre::*;
        pub use idol::*;
        pub use label::*;
        pub use record::*;
        pub use series::*;
        pub use statistics::*;
        pub use studio::*;
    }
    pub mod routes;
}

mod domain {
    mod model {
        pub(super) mod director;
        pub(super) mod genre;
        pub(super) mod idol;
        pub(super) mod label;
        pub(super) mod links;
        pub(super) mod record;
        pub(super) mod series;
        pub(super) mod studio;
    }
    mod repository {
        //! This module defines repository traits for luna (cards) domain entities,
        //! which abstract the database operations.
        pub(super) mod director;
        pub(super) mod genre;
        pub(super) mod idol;
        pub(super) mod label;
        pub(super) mod record;
        pub(super) mod series;
        pub(super) mod studio;
    }

    mod service;

    pub use model::{
        director::*, genre::*, idol::*, label::*, links::*, record::*, series::*, studio::*,
    };
    pub use service::{
        director::DirectorServiceTrait, genre::GenreServiceTrait, idol::IdolServiceTrait,
        label::LabelServiceTrait, record::RecordServiceTrait, series::SeriesServiceTrait,
        studio::StudioServiceTrait, LunaServiceTrait,
    };

    pub use repository::{
        director::DirectorRepository, genre::GenreRepository, idol::IdolRepository,
        label::LabelRepository, record::RecordRepository, series::SeriesRepository,
        studio::StudioRepository,
    };
}

pub mod dto {
    mod director;
    mod genre;
    mod idol;
    mod label;
    mod link;
    mod pagination;
    mod record;
    mod series;
    mod statistics;
    mod studio;

    pub use director::*;
    pub use genre::*;
    pub use idol::*;
    pub use label::*;
    pub use link::*;
    pub use pagination::*;
    pub use record::*;
    pub use series::*;
    pub use statistics::*;
    pub use studio::*;
}

mod infra {
    mod impl_repository {
        pub(super) mod director;
        pub(super) mod genre;
        pub(super) mod idol;
        pub(super) mod label;
        pub(super) mod record;
        pub(super) mod series;
        pub(super) mod studio;
    }
    pub use impl_repository::{
        director::*, genre::*, idol::*, label::*, record::*, series::*, studio::*,
    };

    pub mod impl_service;
}

// Re-export commonly used items for convenience
pub use api::routes::{luna_routes, LunaApiDoc};
pub use domain::LunaServiceTrait;
pub use infra::impl_service::LunaService;
