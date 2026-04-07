use crate::domains::luna::{
    domain::{Series, SeriesRepository},
    dto::{
        CreateSeriesDto, EntityCountDto, PaginatedResponse, PaginationQuery, SearchSeriesDto,
        UpdateSeriesDto,
    },
};
use crate::entities::{record, series, RecordEntity, SeriesEntity};
use sea_orm::{
    ActiveModelTrait as _, ColumnTrait as _, EntityTrait as _, PaginatorTrait as _,
    QueryFilter as _,
};

impl_named_entity_repo!(
    paginated;
    SeriesRepo,
    Series,
    series,
    SeriesEntity,
    SeriesRepository,
    SearchSeriesDto,
    CreateSeriesDto,
    UpdateSeriesDto,
    get_series_record_counts,
    RecordEntity,
    record,
    SeriesId
);
