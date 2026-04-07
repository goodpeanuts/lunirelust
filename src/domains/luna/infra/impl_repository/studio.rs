use crate::domains::luna::{
    domain::{Studio, StudioRepository},
    dto::{CreateStudioDto, EntityCountDto, SearchStudioDto, UpdateStudioDto},
};
use crate::entities::{record, studio, RecordEntity, StudioEntity};
use sea_orm::{ActiveModelTrait as _, ColumnTrait as _, EntityTrait as _, QueryFilter as _};

impl_named_entity_repo!(
    StudioRepo,
    Studio,
    studio,
    StudioEntity,
    StudioRepository,
    SearchStudioDto,
    CreateStudioDto,
    UpdateStudioDto,
    get_studio_record_counts,
    RecordEntity,
    record,
    StudioId
);
