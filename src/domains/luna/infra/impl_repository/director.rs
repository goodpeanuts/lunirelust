use crate::domains::luna::{
    domain::{Director, DirectorRepository},
    dto::{
        CreateDirectorDto, EntityCountDto, PaginatedResponse, PaginationQuery, SearchDirectorDto,
        UpdateDirectorDto,
    },
};
use crate::entities::{director, record, DirectorEntity, RecordEntity};
use sea_orm::{
    ActiveModelTrait as _, ColumnTrait as _, EntityTrait as _, PaginatorTrait as _,
    QueryFilter as _,
};

impl_named_entity_repo!(
    paginated;
    DirectorRepo, Director, director, DirectorEntity,
    DirectorRepository, SearchDirectorDto, CreateDirectorDto, UpdateDirectorDto,
    get_director_record_counts, RecordEntity, record, DirectorId
);
