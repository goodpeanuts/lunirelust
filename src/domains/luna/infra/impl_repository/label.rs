use crate::domains::luna::{
    domain::{Label, LabelRepository},
    dto::{
        CreateLabelDto, EntityCountDto, PaginatedResponse, PaginationQuery, SearchLabelDto,
        UpdateLabelDto,
    },
};
use crate::entities::{label, record, LabelEntity, RecordEntity};
use sea_orm::{
    ActiveModelTrait as _, ColumnTrait as _, EntityTrait as _, PaginatorTrait as _,
    QueryFilter as _,
};

impl_named_entity_repo!(
    paginated;
    LabelRepo, Label, label, LabelEntity,
    LabelRepository, SearchLabelDto, CreateLabelDto, UpdateLabelDto,
    get_label_record_counts, RecordEntity, record, LabelId
);
