use crate::{
    common::error::AppError,
    domains::luna::{
        domain::{
            model::Record,
            repository::{
                DirectorRepository, GenreRepository, IdolRepository, LabelRepository,
                RecordRepository, SeriesRepository, StudioRepository,
            },
            service::{
                DirectorServiceTrait, GenreServiceTrait, IdolServiceTrait, LabelServiceTrait,
                LunaServiceTrait, RecordServiceTrait, SeriesServiceTrait, StudioServiceTrait,
            },
        },
        dto::luna_dto::{
            CreateDirectorDto, CreateGenreDto, CreateIdolDto, CreateLabelDto, CreateRecordDto,
            CreateSeriesDto, CreateStudioDto, DirectorDto, EntityCountDto, GenreDto, IdolDto,
            LabelDto, PaginatedResponse, PaginationQuery, RecordDto, SearchDirectorDto,
            SearchGenreDto, SearchIdolDto, SearchLabelDto, SearchRecordDto, SearchSeriesDto,
            SearchStudioDto, SeriesDto, StudioDto, UpdateDirectorDto, UpdateGenreDto,
            UpdateIdolDto, UpdateLabelDto, UpdateRecordDto, UpdateSeriesDto, UpdateStudioDto,
        },
        infra::impl_repository::{
            DirectorRepo, GenreRepo, IdolRepo, LabelRepo, RecordRepo, SeriesRepo, StudioRepo,
        },
    },
};
use async_trait::async_trait;
use sea_orm::{DatabaseConnection, TransactionTrait as _};
use std::sync::Arc;

/// Service struct for handling director-related operations.
#[derive(Clone)]
pub struct DirectorService {
    pub db: DatabaseConnection,
    pub repo: Arc<dyn DirectorRepository + Send + Sync>,
}

#[async_trait]
impl DirectorServiceTrait for DirectorService {
    /// Constructor for the service.
    fn create_service(db: DatabaseConnection) -> Arc<dyn DirectorServiceTrait> {
        Arc::new(Self {
            db,
            repo: Arc::new(DirectorRepo {}),
        })
    }

    /// Retrieves a director by their ID.
    async fn get_director_by_id(&self, id: i64) -> Result<DirectorDto, AppError> {
        match self.repo.find_by_id(&self.db, id).await {
            Ok(Some(director)) => Ok(DirectorDto::from(director)),
            Ok(None) => Err(AppError::NotFound("Director not found".into())),
            Err(err) => {
                tracing::error!("Error retrieving director: {err}");
                Err(AppError::DatabaseError(err))
            }
        }
    }

    /// Retrieves director list by condition
    async fn get_director_list(
        &self,
        search_dto: SearchDirectorDto,
    ) -> Result<Vec<DirectorDto>, AppError> {
        match self.repo.find_list(&self.db, search_dto).await {
            Ok(directors) => {
                let director_dtos: Vec<DirectorDto> =
                    directors.into_iter().map(Into::into).collect();
                Ok(director_dtos)
            }
            Err(err) => {
                tracing::error!("Error fetching directors: {err}");
                Err(AppError::DatabaseError(err))
            }
        }
    }

    /// Retrieves director list with pagination
    async fn get_director_list_paginated(
        &self,
        search_dto: SearchDirectorDto,
        pagination: PaginationQuery,
    ) -> Result<PaginatedResponse<DirectorDto>, AppError> {
        match self
            .repo
            .find_list_paginated(&self.db, search_dto, pagination)
            .await
        {
            Ok(paginated_response) => {
                let director_dtos: Vec<DirectorDto> = paginated_response
                    .results
                    .into_iter()
                    .map(Into::into)
                    .collect();
                Ok(PaginatedResponse {
                    count: paginated_response.count,
                    next: paginated_response.next,
                    previous: paginated_response.previous,
                    results: director_dtos,
                })
            }
            Err(err) => {
                tracing::error!("Error retrieving paginated director list: {err}");
                Err(AppError::DatabaseError(err))
            }
        }
    }

    /// Retrieves all directors.
    async fn get_directors(&self) -> Result<Vec<DirectorDto>, AppError> {
        match self.repo.find_all(&self.db).await {
            Ok(directors) => {
                let director_dtos: Vec<DirectorDto> =
                    directors.into_iter().map(Into::into).collect();
                Ok(director_dtos)
            }
            Err(err) => {
                tracing::error!("Error fetching directors: {err}");
                Err(AppError::DatabaseError(err))
            }
        }
    }

    /// Creates a new director.
    async fn create_director(
        &self,
        create_dto: CreateDirectorDto,
    ) -> Result<DirectorDto, AppError> {
        let txn = self.db.begin().await?;

        let director_id = match self.repo.create(&txn, create_dto).await {
            Ok(director_id) => director_id,
            Err(err) => {
                tracing::error!("Error creating director: {err}");
                txn.rollback().await?;
                return Err(AppError::DatabaseError(err));
            }
        };

        txn.commit().await?;

        match self.repo.find_by_id(&self.db, director_id).await {
            Ok(Some(director)) => Ok(DirectorDto::from(director)),
            Ok(None) => Err(AppError::NotFound("Director not found".into())),
            Err(err) => {
                tracing::error!("Error retrieving director: {err}");
                Err(AppError::DatabaseError(err))
            }
        }
    }

    /// Updates an existing director.
    async fn update_director(
        &self,
        id: i64,
        payload: UpdateDirectorDto,
    ) -> Result<DirectorDto, AppError> {
        let txn = self.db.begin().await?;

        match self.repo.update(&txn, id, payload).await {
            Ok(Some(director)) => {
                txn.commit().await?;
                Ok(DirectorDto::from(director))
            }
            Ok(None) => {
                txn.rollback().await?;
                Err(AppError::NotFound("Director not found".into()))
            }
            Err(err) => {
                tracing::error!("Error updating director: {err}");
                txn.rollback().await?;
                Err(AppError::DatabaseError(err))
            }
        }
    }

    /// Deletes a director by their ID.
    async fn delete_director(&self, id: i64) -> Result<String, AppError> {
        let txn = self.db.begin().await?;

        match self.repo.delete(&txn, id).await {
            Ok(true) => {
                txn.commit().await?;
                Ok("Director deleted".into())
            }
            Ok(false) => {
                txn.rollback().await?;
                Err(AppError::NotFound("Director not found".into()))
            }
            Err(err) => {
                tracing::error!("Error deleting director: {err}");
                txn.rollback().await?;
                Err(AppError::DatabaseError(err))
            }
        }
    }

    /// Gets record counts grouped by directors.
    async fn get_director_record_counts(&self) -> Result<Vec<EntityCountDto>, AppError> {
        self.repo
            .get_director_record_counts(&self.db)
            .await
            .map_err(AppError::DatabaseError)
    }
}

/// Service struct for handling genre-related operations.
#[derive(Clone)]
pub struct GenreService {
    pub db: DatabaseConnection,
    pub repo: Arc<dyn GenreRepository + Send + Sync>,
}

#[async_trait]
impl GenreServiceTrait for GenreService {
    /// Constructor for the service.
    fn create_service(db: DatabaseConnection) -> Arc<dyn GenreServiceTrait> {
        Arc::new(Self {
            db,
            repo: Arc::new(GenreRepo {}),
        })
    }

    /// Retrieves a genre by their ID.
    async fn get_genre_by_id(&self, id: i64) -> Result<GenreDto, AppError> {
        match self.repo.find_by_id(&self.db, id).await {
            Ok(Some(genre)) => Ok(GenreDto::from(genre)),
            Ok(None) => Err(AppError::NotFound("Genre not found".into())),
            Err(err) => {
                tracing::error!("Error retrieving genre: {err}");
                Err(AppError::DatabaseError(err))
            }
        }
    }

    /// Retrieves genre list by condition
    async fn get_genre_list(&self, search_dto: SearchGenreDto) -> Result<Vec<GenreDto>, AppError> {
        match self.repo.find_list(&self.db, search_dto).await {
            Ok(genres) => {
                let genre_dtos: Vec<GenreDto> = genres.into_iter().map(Into::into).collect();
                Ok(genre_dtos)
            }
            Err(err) => {
                tracing::error!("Error retrieving genre list: {err}");
                Err(AppError::DatabaseError(err))
            }
        }
    }

    /// Retrieves genre list with pagination
    async fn get_genre_list_paginated(
        &self,
        search_dto: SearchGenreDto,
        pagination: PaginationQuery,
    ) -> Result<PaginatedResponse<GenreDto>, AppError> {
        match self
            .repo
            .find_list_paginated(&self.db, search_dto, pagination)
            .await
        {
            Ok(paginated_response) => {
                let genre_dtos: Vec<GenreDto> = paginated_response
                    .results
                    .into_iter()
                    .map(Into::into)
                    .collect();
                Ok(PaginatedResponse {
                    count: paginated_response.count,
                    next: paginated_response.next,
                    previous: paginated_response.previous,
                    results: genre_dtos,
                })
            }
            Err(err) => {
                tracing::error!("Error retrieving paginated genre list: {err}");
                Err(AppError::DatabaseError(err))
            }
        }
    }

    /// Retrieves all genres from the database.
    async fn get_genres(&self) -> Result<Vec<GenreDto>, AppError> {
        match self.repo.find_all(&self.db).await {
            Ok(genres) => {
                let genre_dtos: Vec<GenreDto> = genres.into_iter().map(Into::into).collect();
                Ok(genre_dtos)
            }
            Err(err) => {
                tracing::error!("Error retrieving all genres: {err}");
                Err(AppError::DatabaseError(err))
            }
        }
    }

    /// Creates a new genre.
    async fn create_genre(&self, create_dto: CreateGenreDto) -> Result<GenreDto, AppError> {
        let txn = self.db.begin().await.map_err(AppError::DatabaseError)?;

        match self.repo.create(&txn, create_dto).await {
            Ok(genre_id) => {
                txn.commit().await.map_err(AppError::DatabaseError)?;
                self.get_genre_by_id(genre_id).await
            }
            Err(err) => {
                txn.rollback().await?;
                tracing::error!("Error creating genre: {err}");
                Err(AppError::DatabaseError(err))
            }
        }
    }

    /// Updates an existing genre.
    async fn update_genre(
        &self,
        id: i64,
        update_dto: UpdateGenreDto,
    ) -> Result<GenreDto, AppError> {
        let txn = self.db.begin().await.map_err(AppError::DatabaseError)?;

        match self.repo.update(&txn, id, update_dto).await {
            Ok(Some(genre)) => {
                txn.commit().await.map_err(AppError::DatabaseError)?;
                Ok(GenreDto::from(genre))
            }
            Ok(None) => {
                txn.rollback().await?;
                Err(AppError::NotFound("Genre not found".into()))
            }
            Err(err) => {
                txn.rollback().await?;
                tracing::error!("Error updating genre: {err}");
                Err(AppError::DatabaseError(err))
            }
        }
    }

    /// Deletes a genre by their ID.
    async fn delete_genre(&self, id: i64) -> Result<String, AppError> {
        let txn = self.db.begin().await.map_err(AppError::DatabaseError)?;

        match self.repo.delete(&txn, id).await {
            Ok(true) => {
                txn.commit().await.map_err(AppError::DatabaseError)?;
                Ok("Genre deleted successfully".to_owned())
            }
            Ok(false) => {
                txn.rollback().await?;
                Err(AppError::NotFound("Genre not found".into()))
            }
            Err(err) => {
                txn.rollback().await?;
                tracing::error!("Error deleting genre: {err}");
                Err(AppError::DatabaseError(err))
            }
        }
    }

    /// Gets record counts grouped by genres.
    async fn get_genre_record_counts(&self) -> Result<Vec<EntityCountDto>, AppError> {
        self.repo
            .get_genre_record_counts(&self.db)
            .await
            .map_err(AppError::DatabaseError)
    }
}

/// Service struct for handling label-related operations.
#[derive(Clone)]
pub struct LabelService {
    pub db: DatabaseConnection,
    pub repo: Arc<dyn LabelRepository + Send + Sync>,
}

#[async_trait]
impl LabelServiceTrait for LabelService {
    /// Constructor for the service.
    fn create_service(db: DatabaseConnection) -> Arc<dyn LabelServiceTrait> {
        Arc::new(Self {
            db,
            repo: Arc::new(LabelRepo {}),
        })
    }

    /// Retrieves a label by their ID.
    async fn get_label_by_id(&self, id: i64) -> Result<LabelDto, AppError> {
        match self.repo.find_by_id(&self.db, id).await {
            Ok(Some(label)) => Ok(LabelDto::from(label)),
            Ok(None) => Err(AppError::NotFound("Label not found".into())),
            Err(err) => {
                tracing::error!("Error retrieving label: {err}");
                Err(AppError::DatabaseError(err))
            }
        }
    }

    /// Retrieves label list by condition
    async fn get_label_list(&self, search_dto: SearchLabelDto) -> Result<Vec<LabelDto>, AppError> {
        match self.repo.find_list(&self.db, search_dto).await {
            Ok(labels) => {
                let label_dtos: Vec<LabelDto> = labels.into_iter().map(Into::into).collect();
                Ok(label_dtos)
            }
            Err(err) => {
                tracing::error!("Error retrieving label list: {err}");
                Err(AppError::DatabaseError(err))
            }
        }
    }

    /// Retrieves label list with pagination
    async fn get_label_list_paginated(
        &self,
        search_dto: SearchLabelDto,
        pagination: PaginationQuery,
    ) -> Result<PaginatedResponse<LabelDto>, AppError> {
        match self
            .repo
            .find_list_paginated(&self.db, search_dto, pagination)
            .await
        {
            Ok(paginated_response) => {
                let label_dtos: Vec<LabelDto> = paginated_response
                    .results
                    .into_iter()
                    .map(Into::into)
                    .collect();
                Ok(PaginatedResponse {
                    count: paginated_response.count,
                    next: paginated_response.next,
                    previous: paginated_response.previous,
                    results: label_dtos,
                })
            }
            Err(err) => {
                tracing::error!("Error retrieving paginated label list: {err}");
                Err(AppError::DatabaseError(err))
            }
        }
    }

    /// Retrieves all labels from the database.
    async fn get_labels(&self) -> Result<Vec<LabelDto>, AppError> {
        match self.repo.find_all(&self.db).await {
            Ok(labels) => {
                let label_dtos: Vec<LabelDto> = labels.into_iter().map(Into::into).collect();
                Ok(label_dtos)
            }
            Err(err) => {
                tracing::error!("Error retrieving all labels: {err}");
                Err(AppError::DatabaseError(err))
            }
        }
    }

    /// Creates a new label.
    async fn create_label(&self, create_dto: CreateLabelDto) -> Result<LabelDto, AppError> {
        let txn = self.db.begin().await.map_err(AppError::DatabaseError)?;

        match self.repo.create(&txn, create_dto).await {
            Ok(label_id) => {
                txn.commit().await.map_err(AppError::DatabaseError)?;
                self.get_label_by_id(label_id).await
            }
            Err(err) => {
                txn.rollback().await?;
                tracing::error!("Error creating label: {err}");
                Err(AppError::DatabaseError(err))
            }
        }
    }

    /// Updates an existing label.
    async fn update_label(
        &self,
        id: i64,
        update_dto: UpdateLabelDto,
    ) -> Result<LabelDto, AppError> {
        let txn = self.db.begin().await.map_err(AppError::DatabaseError)?;

        match self.repo.update(&txn, id, update_dto).await {
            Ok(Some(label)) => {
                txn.commit().await.map_err(AppError::DatabaseError)?;
                Ok(LabelDto::from(label))
            }
            Ok(None) => {
                txn.rollback().await?;
                Err(AppError::NotFound("Label not found".into()))
            }
            Err(err) => {
                txn.rollback().await?;
                tracing::error!("Error updating label: {err}");
                Err(AppError::DatabaseError(err))
            }
        }
    }

    /// Deletes a label by their ID.
    async fn delete_label(&self, id: i64) -> Result<String, AppError> {
        let txn = self.db.begin().await.map_err(AppError::DatabaseError)?;

        match self.repo.delete(&txn, id).await {
            Ok(true) => {
                txn.commit().await.map_err(AppError::DatabaseError)?;
                Ok("Label deleted successfully".to_owned())
            }
            Ok(false) => {
                txn.rollback().await?;
                Err(AppError::NotFound("Label not found".into()))
            }
            Err(err) => {
                txn.rollback().await?;
                tracing::error!("Error deleting label: {err}");
                Err(AppError::DatabaseError(err))
            }
        }
    }

    /// Gets record counts grouped by labels.
    async fn get_label_record_counts(&self) -> Result<Vec<EntityCountDto>, AppError> {
        self.repo
            .get_label_record_counts(&self.db)
            .await
            .map_err(AppError::DatabaseError)
    }
}

/// Service struct for handling studio-related operations.
#[derive(Clone)]
pub struct StudioService {
    pub db: DatabaseConnection,
    pub repo: Arc<dyn StudioRepository + Send + Sync>,
}

#[async_trait]
impl StudioServiceTrait for StudioService {
    /// Creates a new studio service.
    fn create_service(db: DatabaseConnection) -> Arc<dyn StudioServiceTrait> {
        Arc::new(Self {
            db: db.clone(),
            repo: Arc::new(StudioRepo),
        })
    }

    /// Retrieves a studio by ID.
    async fn get_studio_by_id(&self, id: i64) -> Result<StudioDto, AppError> {
        match self.repo.find_by_id(&self.db, id).await {
            Ok(Some(studio)) => Ok(StudioDto::from(studio)),
            Ok(None) => Err(AppError::NotFound("Studio not found".into())),
            Err(err) => {
                tracing::error!("Error finding studio by ID: {err}");
                Err(AppError::DatabaseError(err))
            }
        }
    }

    /// Retrieves studio list by condition.
    async fn get_studio_list(
        &self,
        search_dto: SearchStudioDto,
    ) -> Result<Vec<StudioDto>, AppError> {
        match self.repo.find_list(&self.db, search_dto).await {
            Ok(studios) => Ok(studios.into_iter().map(StudioDto::from).collect()),
            Err(err) => {
                tracing::error!("Error finding studio list: {err}");
                Err(AppError::DatabaseError(err))
            }
        }
    }

    /// Retrieves studio list with pagination.
    async fn get_studio_list_paginated(
        &self,
        search_dto: SearchStudioDto,
        pagination: PaginationQuery,
    ) -> Result<PaginatedResponse<StudioDto>, AppError> {
        // For consistency, always use paginated approach
        let limit = pagination.limit.unwrap_or(20);
        let offset = pagination.offset.unwrap_or(0);
        let mut query = vec![];

        if let Some(id) = search_dto.id {
            query.push(("id", id.to_string()));
        }

        if let Some(name) = search_dto.name.as_deref().filter(|s| !s.trim().is_empty()) {
            query.push(("name", name.to_owned()));
        }

        if let Some(link) = search_dto.link.as_deref().filter(|s| !s.trim().is_empty()) {
            query.push(("link", link.to_owned()));
        }

        // Build query string
        let query_string = if query.is_empty() {
            String::new()
        } else {
            format!(
                "?{}",
                query
                    .iter()
                    .map(|(k, v)| format!("{k}={v}"))
                    .collect::<Vec<_>>()
                    .join("&")
            )
        };

        match self.repo.find_list(&self.db, search_dto).await {
            Ok(all_studios) => {
                let total_count = all_studios.len() as i64;

                // Apply manual pagination
                let studios: Vec<_> = all_studios
                    .into_iter()
                    .skip(offset as usize)
                    .take(limit as usize)
                    .collect();

                // Calculate next and previous page URLs
                let base_url = format!("/cards/studios{query_string}");
                let has_next = (offset + limit) < total_count;
                let has_previous = offset > 0;

                let next = if has_next {
                    let sep = if query_string.is_empty() { "?" } else { "&" };
                    Some(format!(
                        "{}{}limit={}&offset={}",
                        base_url,
                        sep,
                        limit,
                        offset + limit
                    ))
                } else {
                    None
                };

                let previous = if has_previous {
                    let prev_offset = std::cmp::max(0, offset - limit);
                    let sep = if query_string.is_empty() { "?" } else { "&" };
                    Some(format!("{base_url}{sep}limit={limit}&offset={prev_offset}"))
                } else {
                    None
                };

                Ok(PaginatedResponse {
                    count: total_count,
                    next,
                    previous,
                    results: studios.into_iter().map(StudioDto::from).collect(),
                })
            }
            Err(err) => {
                tracing::error!("Error finding paginated studio list: {err}");
                Err(AppError::DatabaseError(err))
            }
        }
    }

    /// Retrieves all studios.
    async fn get_studios(&self) -> Result<Vec<StudioDto>, AppError> {
        match self.repo.find_all(&self.db).await {
            Ok(studios) => Ok(studios.into_iter().map(StudioDto::from).collect()),
            Err(err) => {
                tracing::error!("Error finding all studios: {err}");
                Err(AppError::DatabaseError(err))
            }
        }
    }

    /// Creates a new studio.
    async fn create_studio(&self, create_dto: CreateStudioDto) -> Result<StudioDto, AppError> {
        let txn = match self.db.begin().await {
            Ok(txn) => txn,
            Err(err) => {
                tracing::error!("Error starting transaction: {err}");
                return Err(AppError::DatabaseError(err));
            }
        };

        match self.repo.create(&txn, create_dto).await {
            Ok(studio_id) => {
                if let Err(err) = txn.commit().await {
                    tracing::error!("Error committing transaction: {err}");
                    return Err(AppError::DatabaseError(err));
                }

                // Fetch the created studio
                self.get_studio_by_id(studio_id).await
            }
            Err(err) => {
                txn.rollback().await?;
                tracing::error!("Error creating studio: {err}");
                Err(AppError::DatabaseError(err))
            }
        }
    }

    /// Updates an existing studio.
    async fn update_studio(
        &self,
        id: i64,
        update_dto: UpdateStudioDto,
    ) -> Result<StudioDto, AppError> {
        let txn = match self.db.begin().await {
            Ok(txn) => txn,
            Err(err) => {
                tracing::error!("Error starting transaction: {err}");
                return Err(AppError::DatabaseError(err));
            }
        };

        match self.repo.update(&txn, id, update_dto).await {
            Ok(Some(studio)) => {
                if let Err(err) = txn.commit().await {
                    tracing::error!("Error committing transaction: {err}");
                    return Err(AppError::DatabaseError(err));
                }
                Ok(StudioDto::from(studio))
            }
            Ok(None) => {
                txn.rollback().await?;
                Err(AppError::NotFound("Studio not found".into()))
            }
            Err(err) => {
                txn.rollback().await?;
                tracing::error!("Error updating studio: {err}");
                Err(AppError::DatabaseError(err))
            }
        }
    }

    /// Deletes a studio by their ID.
    async fn delete_studio(&self, id: i64) -> Result<String, AppError> {
        let txn = match self.db.begin().await {
            Ok(txn) => txn,
            Err(err) => {
                tracing::error!("Error starting transaction: {err}");
                return Err(AppError::DatabaseError(err));
            }
        };

        match self.repo.delete(&txn, id).await {
            Ok(true) => {
                if let Err(err) = txn.commit().await {
                    tracing::error!("Error committing transaction: {err}");
                    return Err(AppError::DatabaseError(err));
                }
                Ok("Studio deleted successfully".into())
            }
            Ok(false) => {
                txn.rollback().await?;
                Err(AppError::NotFound("Studio not found".into()))
            }
            Err(err) => {
                txn.rollback().await?;
                tracing::error!("Error deleting studio: {err}");
                Err(AppError::DatabaseError(err))
            }
        }
    }

    /// Gets record counts grouped by studios.
    async fn get_studio_record_counts(&self) -> Result<Vec<EntityCountDto>, AppError> {
        self.repo
            .get_studio_record_counts(&self.db)
            .await
            .map_err(AppError::DatabaseError)
    }
}

/// Service struct for handling series-related operations.
#[derive(Clone)]
pub struct SeriesService {
    db: DatabaseConnection,
    repo: Arc<dyn SeriesRepository + Send + Sync>,
}

#[async_trait]
impl SeriesServiceTrait for SeriesService {
    fn create_service(db: DatabaseConnection) -> Arc<dyn SeriesServiceTrait> {
        Arc::new(Self {
            db: db.clone(),
            repo: Arc::new(SeriesRepo),
        })
    }

    async fn get_series_by_id(&self, id: i64) -> Result<SeriesDto, AppError> {
        let series = self
            .repo
            .find_by_id(&self.db, id)
            .await
            .map_err(AppError::DatabaseError)?;

        series
            .map(SeriesDto::from)
            .ok_or_else(|| AppError::NotFound("Series not found".into()))
    }

    async fn get_series_list(
        &self,
        search_dto: SearchSeriesDto,
    ) -> Result<Vec<SeriesDto>, AppError> {
        let series = self
            .repo
            .find_list(&self.db, search_dto)
            .await
            .map_err(AppError::DatabaseError)?;

        Ok(series.into_iter().map(SeriesDto::from).collect())
    }

    async fn get_series_list_paginated(
        &self,
        search_dto: SearchSeriesDto,
        pagination: PaginationQuery,
    ) -> Result<PaginatedResponse<SeriesDto>, AppError> {
        // Implementation similar to director service
        let series = self
            .repo
            .find_list(&self.db, search_dto)
            .await
            .map_err(AppError::DatabaseError)?;

        let limit = pagination.limit.unwrap_or(1000) as usize;
        let offset = pagination.offset.unwrap_or(0) as usize;

        let total_count = series.len();
        let paginated_series: Vec<SeriesDto> = series
            .into_iter()
            .skip(offset)
            .take(limit)
            .map(SeriesDto::from)
            .collect();

        Ok(PaginatedResponse {
            count: total_count as i64,
            next: if offset + limit < total_count {
                Some(format!("?limit={}&offset={}", limit, offset + limit))
            } else {
                None
            },
            previous: if offset > 0 {
                Some(format!(
                    "?limit={}&offset={}",
                    limit,
                    (offset.saturating_sub(limit))
                ))
            } else {
                None
            },
            results: paginated_series,
        })
    }

    async fn get_series(&self) -> Result<Vec<SeriesDto>, AppError> {
        let series = self
            .repo
            .find_all(&self.db)
            .await
            .map_err(AppError::DatabaseError)?;

        Ok(series.into_iter().map(SeriesDto::from).collect())
    }

    async fn create_series(&self, create_dto: CreateSeriesDto) -> Result<SeriesDto, AppError> {
        let txn = self.db.begin().await.map_err(AppError::DatabaseError)?;

        let id = self
            .repo
            .create(&txn, create_dto)
            .await
            .map_err(AppError::DatabaseError)?;

        txn.commit().await.map_err(AppError::DatabaseError)?;

        self.get_series_by_id(id).await
    }

    async fn update_series(
        &self,
        id: i64,
        update_dto: UpdateSeriesDto,
    ) -> Result<SeriesDto, AppError> {
        let txn = self.db.begin().await.map_err(AppError::DatabaseError)?;

        let updated_series = self
            .repo
            .update(&txn, id, update_dto)
            .await
            .map_err(AppError::DatabaseError)?;

        txn.commit().await.map_err(AppError::DatabaseError)?;

        updated_series
            .map(SeriesDto::from)
            .ok_or_else(|| AppError::NotFound("Series not found".into()))
    }

    async fn delete_series(&self, id: i64) -> Result<String, AppError> {
        let txn = self.db.begin().await.map_err(AppError::DatabaseError)?;

        let deleted = self
            .repo
            .delete(&txn, id)
            .await
            .map_err(AppError::DatabaseError)?;

        if deleted {
            txn.commit().await.map_err(AppError::DatabaseError)?;
            Ok("Series deleted successfully".to_owned())
        } else {
            txn.rollback().await.map_err(AppError::DatabaseError)?;
            Err(AppError::NotFound("Series not found".into()))
        }
    }

    /// Gets record counts grouped by series.
    async fn get_series_record_counts(&self) -> Result<Vec<EntityCountDto>, AppError> {
        self.repo
            .get_series_record_counts(&self.db)
            .await
            .map_err(AppError::DatabaseError)
    }
}

/// Service struct for handling idol-related operations.
#[derive(Clone)]
pub struct IdolService {
    db: DatabaseConnection,
    repo: Arc<dyn IdolRepository + Send + Sync>,
}

#[async_trait]
impl IdolServiceTrait for IdolService {
    fn create_service(db: DatabaseConnection) -> Arc<dyn IdolServiceTrait> {
        Arc::new(Self {
            db: db.clone(),
            repo: Arc::new(IdolRepo),
        })
    }

    async fn get_idol_by_id(&self, id: i64) -> Result<IdolDto, AppError> {
        let idol = self
            .repo
            .find_by_id(&self.db, id)
            .await
            .map_err(AppError::DatabaseError)?;

        idol.map(IdolDto::from)
            .ok_or_else(|| AppError::NotFound("Idol not found".into()))
    }

    async fn get_idol_list(&self, search_dto: SearchIdolDto) -> Result<Vec<IdolDto>, AppError> {
        let idols = self
            .repo
            .find_list(&self.db, search_dto)
            .await
            .map_err(AppError::DatabaseError)?;

        Ok(idols.into_iter().map(IdolDto::from).collect())
    }

    async fn get_idol_list_paginated(
        &self,
        search_dto: SearchIdolDto,
        pagination: PaginationQuery,
    ) -> Result<PaginatedResponse<IdolDto>, AppError> {
        let idols = self
            .repo
            .find_list(&self.db, search_dto)
            .await
            .map_err(AppError::DatabaseError)?;

        let limit = pagination.limit.unwrap_or(1000) as usize;
        let offset = pagination.offset.unwrap_or(0) as usize;

        let total_count = idols.len();
        let paginated_idols: Vec<IdolDto> = idols
            .into_iter()
            .skip(offset)
            .take(limit)
            .map(IdolDto::from)
            .collect();

        Ok(PaginatedResponse {
            count: total_count as i64,
            next: if offset + limit < total_count {
                Some(format!("?limit={}&offset={}", limit, offset + limit))
            } else {
                None
            },
            previous: if offset > 0 {
                Some(format!(
                    "?limit={}&offset={}",
                    limit,
                    (offset.saturating_sub(limit))
                ))
            } else {
                None
            },
            results: paginated_idols,
        })
    }

    async fn get_idols(&self) -> Result<Vec<IdolDto>, AppError> {
        let idols = self
            .repo
            .find_all(&self.db)
            .await
            .map_err(AppError::DatabaseError)?;

        Ok(idols.into_iter().map(IdolDto::from).collect())
    }

    async fn create_idol(&self, create_dto: CreateIdolDto) -> Result<IdolDto, AppError> {
        let txn = self.db.begin().await.map_err(AppError::DatabaseError)?;

        let id = self
            .repo
            .create(&txn, create_dto)
            .await
            .map_err(AppError::DatabaseError)?;

        txn.commit().await.map_err(AppError::DatabaseError)?;

        self.get_idol_by_id(id).await
    }

    async fn update_idol(&self, id: i64, update_dto: UpdateIdolDto) -> Result<IdolDto, AppError> {
        let txn = self.db.begin().await.map_err(AppError::DatabaseError)?;

        let updated_idol = self
            .repo
            .update(&txn, id, update_dto)
            .await
            .map_err(AppError::DatabaseError)?;

        txn.commit().await.map_err(AppError::DatabaseError)?;

        updated_idol
            .map(IdolDto::from)
            .ok_or_else(|| AppError::NotFound("Idol not found".into()))
    }

    async fn delete_idol(&self, id: i64) -> Result<String, AppError> {
        let txn = self.db.begin().await.map_err(AppError::DatabaseError)?;

        let deleted = self
            .repo
            .delete(&txn, id)
            .await
            .map_err(AppError::DatabaseError)?;

        if deleted {
            txn.commit().await.map_err(AppError::DatabaseError)?;
            Ok("Idol deleted successfully".to_owned())
        } else {
            txn.rollback().await.map_err(AppError::DatabaseError)?;
            Err(AppError::NotFound("Idol not found".into()))
        }
    }

    /// Gets record counts grouped by idols.
    async fn get_idol_record_counts(&self) -> Result<Vec<EntityCountDto>, AppError> {
        self.repo
            .get_idol_record_counts(&self.db)
            .await
            .map_err(AppError::DatabaseError)
    }
}

/// Service struct for handling record-related operations.
#[derive(Clone)]
pub struct RecordService {
    db: DatabaseConnection,
    repo: Arc<dyn RecordRepository + Send + Sync>,
}

#[async_trait]
impl RecordServiceTrait for RecordService {
    fn create_service(db: DatabaseConnection) -> Arc<dyn RecordServiceTrait> {
        Arc::new(Self {
            db: db.clone(),
            repo: Arc::new(RecordRepo),
        })
    }

    async fn get_record_by_id(&self, id: &str) -> Result<RecordDto, AppError> {
        let record = self
            .repo
            .find_by_id(&self.db, id.to_owned())
            .await
            .map_err(AppError::DatabaseError)?;

        record
            .map(RecordDto::from)
            .ok_or_else(|| AppError::NotFound("Record not found".into()))
    }

    async fn get_record_list(
        &self,
        search_dto: SearchRecordDto,
    ) -> Result<Vec<RecordDto>, AppError> {
        let records = self
            .repo
            .find_list(&self.db, search_dto)
            .await
            .map_err(AppError::DatabaseError)?;

        Ok(records.into_iter().map(RecordDto::from).collect())
    }

    async fn get_record_list_paginated(
        &self,
        search_dto: SearchRecordDto,
        pagination: PaginationQuery,
    ) -> Result<PaginatedResponse<RecordDto>, AppError> {
        let records = self
            .repo
            .find_list(&self.db, search_dto)
            .await
            .map_err(AppError::DatabaseError)?;

        let limit = pagination.limit.unwrap_or(10) as usize;
        let offset = pagination.offset.unwrap_or(0) as usize;

        let total_count = records.len();
        let paginated_records: Vec<RecordDto> = records
            .into_iter()
            .skip(offset)
            .take(limit)
            .map(RecordDto::from)
            .collect();

        Ok(PaginatedResponse {
            count: total_count as i64,
            next: if offset + limit < total_count {
                Some(format!("?limit={}&offset={}", limit, offset + limit))
            } else {
                None
            },
            previous: if offset > 0 {
                Some(format!(
                    "?limit={}&offset={}",
                    limit,
                    (offset.saturating_sub(limit))
                ))
            } else {
                None
            },
            results: paginated_records,
        })
    }

    async fn get_records(&self) -> Result<Vec<RecordDto>, AppError> {
        let records = self
            .repo
            .find_all(&self.db)
            .await
            .map_err(AppError::DatabaseError)?;

        Ok(records.into_iter().map(RecordDto::from).collect())
    }

    async fn create_record(&self, create_dto: CreateRecordDto) -> Result<RecordDto, AppError> {
        let txn = self.db.begin().await.map_err(AppError::DatabaseError)?;

        let id = self
            .repo
            .create(&txn, create_dto)
            .await
            .map_err(AppError::DatabaseError)?;

        txn.commit().await.map_err(AppError::DatabaseError)?;

        self.get_record_by_id(&id).await
    }

    async fn update_record(
        &self,
        id: &str,
        update_dto: UpdateRecordDto,
    ) -> Result<RecordDto, AppError> {
        let txn = self.db.begin().await.map_err(AppError::DatabaseError)?;

        let updated_record = self
            .repo
            .update(&txn, id.to_owned(), update_dto)
            .await
            .map_err(AppError::DatabaseError)?;

        txn.commit().await.map_err(AppError::DatabaseError)?;

        updated_record
            .map(RecordDto::from)
            .ok_or_else(|| AppError::NotFound("Record not found".into()))
    }

    async fn delete_record(&self, id: &str) -> Result<String, AppError> {
        let txn = self.db.begin().await.map_err(AppError::DatabaseError)?;

        let deleted = self
            .repo
            .delete(&txn, id.to_owned())
            .await
            .map_err(AppError::DatabaseError)?;

        if deleted {
            txn.commit().await.map_err(AppError::DatabaseError)?;
            Ok("Record deleted successfully".to_owned())
        } else {
            txn.rollback().await.map_err(AppError::DatabaseError)?;
            Err(AppError::NotFound("Record not found".into()))
        }
    }

    async fn get_records_by_director(
        &self,
        director_id: i64,
        pagination: PaginationQuery,
    ) -> Result<PaginatedResponse<RecordDto>, AppError> {
        // Simplified implementation using search
        let search_dto = SearchRecordDto {
            id: None,
            title: None,
            director_id: None,
            studio_id: None,
            label_id: None,
            series_id: None,
            search: None,
        };

        let all_records = self
            .repo
            .find_list(&self.db, search_dto)
            .await
            .map_err(AppError::DatabaseError)?;

        // Filter by director
        let filtered_records: Vec<Record> = all_records
            .into_iter()
            .filter(|r| r.director.id == director_id)
            .collect();

        let limit = pagination.limit.unwrap_or(10) as usize;
        let offset = pagination.offset.unwrap_or(0) as usize;

        let total_count = filtered_records.len();
        let paginated_records: Vec<RecordDto> = filtered_records
            .into_iter()
            .skip(offset)
            .take(limit)
            .map(RecordDto::from)
            .collect();

        Ok(PaginatedResponse {
            count: total_count as i64,
            next: if offset + limit < total_count {
                Some(format!("?limit={}&offset={}", limit, offset + limit))
            } else {
                None
            },
            previous: if offset > 0 {
                Some(format!(
                    "?limit={}&offset={}",
                    limit,
                    (offset.saturating_sub(limit))
                ))
            } else {
                None
            },
            results: paginated_records,
        })
    }

    async fn get_records_by_studio(
        &self,
        studio_id: i64,
        pagination: PaginationQuery,
    ) -> Result<PaginatedResponse<RecordDto>, AppError> {
        // Similar implementation to get_records_by_director
        let search_dto = SearchRecordDto {
            id: None,
            title: None,
            director_id: None,
            studio_id: None,
            label_id: None,
            series_id: None,
            search: None,
        };

        let all_records = self
            .repo
            .find_list(&self.db, search_dto)
            .await
            .map_err(AppError::DatabaseError)?;

        let filtered_records: Vec<Record> = all_records
            .into_iter()
            .filter(|r| r.studio.id == studio_id)
            .collect();

        let limit = pagination.limit.unwrap_or(10) as usize;
        let offset = pagination.offset.unwrap_or(0) as usize;

        let total_count = filtered_records.len();
        let paginated_records: Vec<RecordDto> = filtered_records
            .into_iter()
            .skip(offset)
            .take(limit)
            .map(RecordDto::from)
            .collect();

        Ok(PaginatedResponse {
            count: total_count as i64,
            next: if offset + limit < total_count {
                Some(format!("?limit={}&offset={}", limit, offset + limit))
            } else {
                None
            },
            previous: if offset > 0 {
                Some(format!(
                    "?limit={}&offset={}",
                    limit,
                    (offset.saturating_sub(limit))
                ))
            } else {
                None
            },
            results: paginated_records,
        })
    }

    async fn get_records_by_label(
        &self,
        label_id: i64,
        pagination: PaginationQuery,
    ) -> Result<PaginatedResponse<RecordDto>, AppError> {
        let search_dto = SearchRecordDto {
            id: None,
            title: None,
            director_id: None,
            studio_id: None,
            label_id: None,
            series_id: None,
            search: None,
        };

        let all_records = self
            .repo
            .find_list(&self.db, search_dto)
            .await
            .map_err(AppError::DatabaseError)?;

        let filtered_records: Vec<Record> = all_records
            .into_iter()
            .filter(|r| r.label.id == label_id)
            .collect();

        let limit = pagination.limit.unwrap_or(10) as usize;
        let offset = pagination.offset.unwrap_or(0) as usize;

        let total_count = filtered_records.len();
        let paginated_records: Vec<RecordDto> = filtered_records
            .into_iter()
            .skip(offset)
            .take(limit)
            .map(RecordDto::from)
            .collect();

        Ok(PaginatedResponse {
            count: total_count as i64,
            next: if offset + limit < total_count {
                Some(format!("?limit={}&offset={}", limit, offset + limit))
            } else {
                None
            },
            previous: if offset > 0 {
                Some(format!(
                    "?limit={}&offset={}",
                    limit,
                    (offset.saturating_sub(limit))
                ))
            } else {
                None
            },
            results: paginated_records,
        })
    }

    async fn get_records_by_series(
        &self,
        series_id: i64,
        pagination: PaginationQuery,
    ) -> Result<PaginatedResponse<RecordDto>, AppError> {
        let search_dto = SearchRecordDto {
            id: None,
            title: None,
            director_id: None,
            studio_id: None,
            label_id: None,
            series_id: None,
            search: None,
        };

        let all_records = self
            .repo
            .find_list(&self.db, search_dto)
            .await
            .map_err(AppError::DatabaseError)?;

        let filtered_records: Vec<Record> = all_records
            .into_iter()
            .filter(|r| r.series.id == series_id)
            .collect();

        let limit = pagination.limit.unwrap_or(10) as usize;
        let offset = pagination.offset.unwrap_or(0) as usize;

        let total_count = filtered_records.len();
        let paginated_records: Vec<RecordDto> = filtered_records
            .into_iter()
            .skip(offset)
            .take(limit)
            .map(RecordDto::from)
            .collect();

        Ok(PaginatedResponse {
            count: total_count as i64,
            next: if offset + limit < total_count {
                Some(format!("?limit={}&offset={}", limit, offset + limit))
            } else {
                None
            },
            previous: if offset > 0 {
                Some(format!(
                    "?limit={}&offset={}",
                    limit,
                    (offset.saturating_sub(limit))
                ))
            } else {
                None
            },
            results: paginated_records,
        })
    }

    async fn get_records_by_genre(
        &self,
        genre_id: i64,
        pagination: PaginationQuery,
    ) -> Result<PaginatedResponse<RecordDto>, AppError> {
        let search_dto = SearchRecordDto {
            id: None,
            title: None,
            director_id: None,
            studio_id: None,
            label_id: None,
            series_id: None,
            search: None,
        };

        let all_records = self
            .repo
            .find_list(&self.db, search_dto)
            .await
            .map_err(AppError::DatabaseError)?;

        // Filter by genre using the genres relation
        let filtered_records: Vec<Record> = all_records
            .into_iter()
            .filter(|r| r.genres.iter().any(|rg| rg.genre.id == genre_id))
            .collect();

        let limit = pagination.limit.unwrap_or(10) as usize;
        let offset = pagination.offset.unwrap_or(0) as usize;

        let total_count = filtered_records.len();
        let paginated_records: Vec<RecordDto> = filtered_records
            .into_iter()
            .skip(offset)
            .take(limit)
            .map(RecordDto::from)
            .collect();

        Ok(PaginatedResponse {
            count: total_count as i64,
            next: if offset + limit < total_count {
                Some(format!("?limit={}&offset={}", limit, offset + limit))
            } else {
                None
            },
            previous: if offset > 0 {
                Some(format!(
                    "?limit={}&offset={}",
                    limit,
                    (offset.saturating_sub(limit))
                ))
            } else {
                None
            },
            results: paginated_records,
        })
    }

    async fn get_records_by_idol(
        &self,
        idol_id: i64,
        pagination: PaginationQuery,
    ) -> Result<PaginatedResponse<RecordDto>, AppError> {
        let search_dto = SearchRecordDto {
            id: None,
            title: None,
            director_id: None,
            studio_id: None,
            label_id: None,
            series_id: None,
            search: None,
        };

        let all_records = self
            .repo
            .find_list(&self.db, search_dto)
            .await
            .map_err(AppError::DatabaseError)?;

        // Filter by idol using the idols relation
        let filtered_records: Vec<Record> = all_records
            .into_iter()
            .filter(|r| r.idols.iter().any(|ip| ip.idol.id == idol_id))
            .collect();

        let limit = pagination.limit.unwrap_or(10) as usize;
        let offset = pagination.offset.unwrap_or(0) as usize;

        let total_count = filtered_records.len();
        let paginated_records: Vec<RecordDto> = filtered_records
            .into_iter()
            .skip(offset)
            .take(limit)
            .map(RecordDto::from)
            .collect();

        Ok(PaginatedResponse {
            count: total_count as i64,
            next: if offset + limit < total_count {
                Some(format!("?limit={}&offset={}", limit, offset + limit))
            } else {
                None
            },
            previous: if offset > 0 {
                Some(format!(
                    "?limit={}&offset={}",
                    limit,
                    (offset.saturating_sub(limit))
                ))
            } else {
                None
            },
            results: paginated_records,
        })
    }
}

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
            director_service: DirectorService::create_service(db.clone()),
            genre_service: GenreService::create_service(db.clone()),
            label_service: LabelService::create_service(db.clone()),
            studio_service: StudioService::create_service(db.clone()),
            series_service: SeriesService::create_service(db.clone()),
            idol_service: IdolService::create_service(db.clone()),
            record_service: RecordService::create_service(db),
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
