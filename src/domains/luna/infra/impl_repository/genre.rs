use crate::domains::luna::{
    domain::{Genre, GenreRepository},
    dto::{
        CreateGenreDto, EntityCountDto, PaginatedResponse, PaginationQuery, SearchGenreDto,
        UpdateGenreDto,
    },
};
use crate::entities::{genre, record_genre, GenreEntity, RecordGenreEntity};
use sea_orm::{
    ActiveModelTrait as _, ColumnTrait as _, EntityTrait as _, PaginatorTrait as _,
    QueryFilter as _,
};

impl_named_entity_repo!(
    paginated;
    GenreRepo, Genre, genre, GenreEntity,
    GenreRepository, SearchGenreDto, CreateGenreDto, UpdateGenreDto,
    get_genre_record_counts, RecordGenreEntity, record_genre, GenreId
);
