use crate::entities::{record, series};
use crate::{
    domains::luna::{
        domain::{Series, SeriesRepository},
        dto::{CreateSeriesDto, EntityCountDto, SearchSeriesDto, UpdateSeriesDto},
    },
    entities::{RecordEntity, SeriesEntity},
};
use async_trait::async_trait;
use sea_orm::{
    ActiveModelTrait as _, ColumnTrait as _, DatabaseConnection, DatabaseTransaction, DbErr,
    EntityTrait as _, PaginatorTrait as _, QueryFilter as _, Set,
};

// Series Repository Implementation
pub struct SeriesRepo;

#[async_trait]
impl SeriesRepository for SeriesRepo {
    async fn find_all(&self, db: &DatabaseConnection) -> Result<Vec<Series>, DbErr> {
        let series_models = SeriesEntity::find().all(db).await?;
        Ok(series_models.into_iter().map(Series::from).collect())
    }

    async fn find_by_id(&self, db: &DatabaseConnection, id: i64) -> Result<Option<Series>, DbErr> {
        let series = SeriesEntity::find_by_id(id).one(db).await?;
        Ok(series.map(Series::from))
    }

    async fn find_list(
        &self,
        db: &DatabaseConnection,
        search_dto: SearchSeriesDto,
    ) -> Result<Vec<Series>, DbErr> {
        let mut query = SeriesEntity::find();

        if let Some(id) = search_dto.id {
            query = query.filter(series::Column::Id.eq(id));
        }
        if let Some(name) = search_dto.name {
            query = query.filter(series::Column::Name.contains(&name));
        }
        if let Some(link) = search_dto.link {
            query = query.filter(series::Column::Link.contains(&link));
        }

        let series_models = query.all(db).await?;
        Ok(series_models.into_iter().map(Series::from).collect())
    }

    async fn create(
        &self,
        txn: &DatabaseTransaction,
        series: CreateSeriesDto,
    ) -> Result<i64, DbErr> {
        let active_series = series::ActiveModel {
            name: Set(series.name),
            link: Set(series.link),
            ..Default::default()
        };

        let result = active_series.insert(txn).await?;
        Ok(result.id)
    }

    async fn update(
        &self,
        txn: &DatabaseTransaction,
        id: i64,
        series: UpdateSeriesDto,
    ) -> Result<Option<Series>, DbErr> {
        match SeriesEntity::find_by_id(id).one(txn).await? {
            Some(existing) => {
                let mut active_series: series::ActiveModel = existing.into();
                active_series.name = Set(series.name);
                active_series.link = Set(series.link);

                let updated = active_series.update(txn).await?;
                Ok(Some(Series::from(updated)))
            }
            None => Ok(None),
        }
    }

    async fn delete(&self, txn: &DatabaseTransaction, id: i64) -> Result<bool, DbErr> {
        let result = SeriesEntity::delete_by_id(id).exec(txn).await?;
        Ok(result.rows_affected > 0)
    }

    async fn get_series_record_counts(
        &self,
        db: &DatabaseConnection,
    ) -> Result<Vec<EntityCountDto>, DbErr> {
        let series_list = SeriesEntity::find().all(db).await?;
        let mut result = Vec::new();

        for series in series_list {
            let count = RecordEntity::find()
                .filter(record::Column::SeriesId.eq(series.id))
                .count(db)
                .await? as i64;

            result.push(EntityCountDto {
                id: series.id,
                name: series.name,
                count,
            });
        }

        // Sort by count descending
        result.sort_by(|a, b| b.count.cmp(&a.count));
        Ok(result)
    }
}
