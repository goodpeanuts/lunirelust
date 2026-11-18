/// Macro to generate the common repository implementation for named entities.
///
/// # Parameters (all must be simple identifiers, imported in the calling module)
/// - `$repo`: Repo struct name (e.g. `DirectorRepo`)
/// - `$domain`: Domain type (e.g. `Director`)
/// - `$entity_mod`: Entity module (e.g. `director`)
/// - `$entity_struct`: Entity struct (e.g. `DirectorEntity`)
/// - `$trait_name`: Repository trait name (e.g. `DirectorRepository`)
/// - `$create_dto`: Create DTO type (e.g. `CreateDirectorDto`)
/// - `$update_dto`: Update DTO type (e.g. `UpdateDirectorDto`)
/// - `$count_method`: Count method name (e.g. `get_director_record_counts`)
/// - `$count_entity_struct`: Count source entity struct (e.g. `RecordEntity`)
/// - `$count_entity_mod`: Count source entity module (e.g. `record`)
/// - `$count_fk_column`: FK column in count entity (e.g. `DirectorId`)
///
/// All named entities share CRUD and counts. Entity-specific methods follow the semicolon.
macro_rules! impl_named_entity_repo {
    (
        $repo:ident, $domain:ident, $entity_mod:ident, $entity_struct:ident,
        $trait_name:ident, $create_dto:ident, $update_dto:ident,
        $count_method:ident, $count_entity_struct:ident, $count_entity_mod:ident,
        $count_fk_column:ident;
        $($extra_methods:item)*
    ) => {
        pub struct $repo;

        #[async_trait::async_trait]
        impl $trait_name for $repo {
            $($extra_methods)*

            async fn find_by_id(
                &self,
                db: &sea_orm::DatabaseConnection,
                id: i64,
            ) -> Result<Option<$domain>, sea_orm::DbErr> {
                let item = $entity_struct::find_by_id(id).one(db).await?;
                Ok(item.map(<$domain>::from))
            }

            async fn create(
                &self,
                txn: &sea_orm::DatabaseTransaction,
                dto: $create_dto,
            ) -> Result<(i64, bool), sea_orm::DbErr> {
                let name = dto.name;
                let link = dto.link.unwrap_or_default();
                let manual = dto.manual.unwrap_or(false);

                let existing = $entity_struct::find()
                    .filter($entity_mod::Column::Name.eq(&name))
                    .filter($entity_mod::Column::Link.eq(&link))
                    .filter($entity_mod::Column::Manual.eq(manual))
                    .one(txn)
                    .await?;
                if let Some(e) = existing {
                    return Ok((e.id, false));
                }

                let active_model = $entity_mod::ActiveModel {
                    name: sea_orm::Set(name),
                    link: sea_orm::Set(link),
                    manual: sea_orm::Set(manual),
                    ..Default::default()
                };
                let inserted = active_model.insert(txn).await?;
                Ok((inserted.id, true))
            }

            async fn update(
                &self,
                txn: &sea_orm::DatabaseTransaction,
                id: i64,
                dto: $update_dto,
            ) -> Result<Option<$domain>, sea_orm::DbErr> {
                let Some(existing) = $entity_struct::find_by_id(id).one(txn).await? else {
                    return Ok(None);
                };

                let new_name = dto.name.clone().unwrap_or(existing.name.clone());
                let new_link = dto.link.clone().unwrap_or(existing.link.clone());
                let new_manual = dto.manual.unwrap_or(existing.manual);

                let matching = $entity_struct::find()
                    .filter($entity_mod::Column::Name.eq(&new_name))
                    .filter($entity_mod::Column::Link.eq(&new_link))
                    .filter($entity_mod::Column::Manual.eq(new_manual))
                    .filter($entity_mod::Column::Id.ne(id))
                    .one(txn)
                    .await?;
                if let Some(m) = matching {
                    $entity_struct::delete_by_id(id).exec(txn).await?;
                    return Ok(Some(<$domain>::from(m)));
                }

                let mut active_model: $entity_mod::ActiveModel = existing.into();
                if let Some(name) = dto.name {
                    active_model.name = sea_orm::Set(name);
                }
                if let Some(link) = dto.link {
                    active_model.link = sea_orm::Set(link);
                }
                if let Some(manual) = dto.manual {
                    active_model.manual = sea_orm::Set(manual);
                }
                let updated = active_model.update(txn).await?;
                Ok(Some(<$domain>::from(updated)))
            }

            async fn delete(
                &self,
                txn: &sea_orm::DatabaseTransaction,
                id: i64,
            ) -> Result<bool, sea_orm::DbErr> {
                let result = $entity_struct::delete_by_id(id).exec(txn).await?;
                Ok(result.rows_affected > 0)
            }

            async fn $count_method(
                &self,
                db: &sea_orm::DatabaseConnection,
            ) -> Result<Vec<EntityCountDto>, sea_orm::DbErr> {
                use sea_orm::{FromQueryResult, QuerySelect as _};
                use std::collections::HashMap;

                #[derive(FromQueryResult)]
                struct CountRow {
                    entity_id: i64,
                    count: i64,
                }

                let counts: Vec<CountRow> = $count_entity_struct::find()
                    .select_only()
                    .column_as($count_entity_mod::Column::$count_fk_column, "entity_id")
                    .column_as($count_entity_mod::Column::Id.count(), "count")
                    .group_by($count_entity_mod::Column::$count_fk_column)
                    .into_model::<CountRow>()
                    .all(db)
                    .await?;

                let count_map: HashMap<i64, i64> =
                    counts.into_iter().map(|c| (c.entity_id, c.count)).collect();

                let entities = $entity_struct::find().all(db).await?;
                let mut result = Vec::new();
                for entity in entities {
                    let count = count_map.get(&entity.id).copied().unwrap_or(0);
                    result.push(EntityCountDto {
                        id: entity.id,
                        name: entity.name,
                        count,
                    });
                }

                result.sort_by(|a, b| b.count.cmp(&a.count));
                Ok(result)
            }
        }
    };

}
