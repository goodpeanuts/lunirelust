use crate::domains::luna::{
    domain::{Idol, IdolRepository},
    dto::{
        CreateIdolDto, EntityCountDto, PaginatedResponse, PaginationQuery, SearchIdolDto,
        UpdateIdolDto,
    },
};
use crate::entities::{idol, idol_participation, IdolEntity, IdolParticipationEntity};
use sea_orm::{
    ActiveModelTrait as _, ColumnTrait as _, EntityTrait as _, PaginatorTrait as _,
    QueryFilter as _,
};

impl_named_entity_repo!(
    paginated;
    IdolRepo,
    Idol,
    idol,
    IdolEntity,
    IdolRepository,
    SearchIdolDto,
    CreateIdolDto,
    UpdateIdolDto,
    get_idol_record_counts,
    IdolParticipationEntity,
    idol_participation,
    IdolId
);
