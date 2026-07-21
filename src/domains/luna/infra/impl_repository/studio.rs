use crate::domains::luna::{
    domain::{Studio, StudioAffinityRepository, StudioRepository},
    dto::{
        CreateStudioDto, EntityCountDto, PaginatedResponse, PaginationQuery, SearchStudioDto,
        UpdateStudioDto,
    },
};
use crate::entities::{record, studio, RecordEntity, StudioEntity};
use async_trait::async_trait;
use sea_orm::{
    ActiveModelTrait as _, ColumnTrait as _, DatabaseBackend, DatabaseConnection, DbErr,
    EntityTrait as _, FromQueryResult, PaginatorTrait as _, QueryFilter as _, Statement, Value,
};

impl_named_entity_repo!(
    paginated;
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

// Tunable hyper-parameters for the affinity score. These are fixed constants
// (not user input), interpolated into the SQL as literals. Two main knobs:
// `AFFINITY_M_V` controls how hard small samples are pulled toward the prior
// (raise it to push single-work studios further down); `AFFINITY_GAMMA`
// controls how much the absolute-volume factor lifts studios with many works
// (raise it to reward large catalogs more). See the score expression below for
// how they combine.
/// Prior mean for the viewed rate (shrinkage target when the sample is small).
const AFFINITY_C_V: f64 = 0.3;
/// Prior strength for the viewed rate (higher = small samples pulled harder).
const AFFINITY_M_V: f64 = 5.0;
/// Prior mean for the liked rate.
const AFFINITY_C_L: f64 = 0.2;
/// Prior strength for the liked rate.
const AFFINITY_M_L: f64 = 3.0;
/// Weight of the shrunk viewed rate in the ratio part.
const AFFINITY_W_V: f64 = 0.6;
/// Weight of the shrunk liked rate in the ratio part.
const AFFINITY_W_L: f64 = 0.4;
/// Extra weight of `liked` inside the absolute-volume factor.
const AFFINITY_BETA: f64 = 1.0;
/// Strength of the absolute-volume amplification.
const AFFINITY_GAMMA: f64 = 0.8;
/// Normalization reference for the volume factor (a "many works" scale).
const AFFINITY_N0: f64 = 50.0;

/// Raw row for the affinity-ordered studio query. Mirrors the `studio` columns
/// projected by the SELECT; the computed `score` column is used only for
/// `ORDER BY` and is intentionally not mapped here.
#[derive(Debug, FromQueryResult)]
struct AffinityStudioRow {
    id: i64,
    name: String,
    link: String,
    manual: bool,
}

impl From<AffinityStudioRow> for Studio {
    fn from(row: AffinityStudioRow) -> Self {
        Self {
            id: row.id,
            name: row.name,
            link: row.link,
            manual: row.manual,
        }
    }
}

/// Build the shared `WHERE` clause for the affinity query from the search DTO.
///
/// Filters match the macro's semantics exactly: `id` exact via `=`; `name` and
/// `link` case-sensitive substring via `LIKE '%' || $n || '%'` (`SeaORM`
/// `.contains()` emits `LIKE`, not `ILIKE`). The user value is always bound as a
/// parameter (never interpolated) to avoid SQL injection. Returns the clause
/// (empty string when no filter applies) and the ordered bind values; the
/// `$user_id` bind is prepended by the caller as `$1`, so placeholders here
/// start at `next_param`.
fn build_affinity_filter(search_dto: &SearchStudioDto, next_param: usize) -> (String, Vec<Value>) {
    let mut clauses: Vec<String> = Vec::new();
    let mut binds: Vec<Value> = Vec::new();
    let mut p = next_param;

    if let Some(id) = search_dto.id {
        clauses.push(format!("t.id = ${p}"));
        binds.push(id.into());
        p += 1;
    }
    if let Some(name) = search_dto.name.as_deref().filter(|s| !s.trim().is_empty()) {
        clauses.push(format!("t.name LIKE '%' || ${p} || '%'"));
        binds.push(name.into());
        p += 1;
    }
    if let Some(link) = search_dto.link.as_deref().filter(|s| !s.trim().is_empty()) {
        clauses.push(format!("t.link LIKE '%' || ${p} || '%'"));
        binds.push(link.into());
    }

    let clause = if clauses.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", clauses.join(" AND "))
    };
    (clause, binds)
}

#[async_trait]
impl StudioAffinityRepository for StudioRepo {
    async fn find_list_paginated_by_affinity(
        &self,
        db: &DatabaseConnection,
        search_dto: SearchStudioDto,
        pagination: PaginationQuery,
        user_id: &str,
    ) -> Result<PaginatedResponse<Studio>, DbErr> {
        // Pagination replicates the shared macro's page-number semantics
        // (entity_repo_macro.rs): page_size defaults to DEFAULT_PAGE_SIZE, and
        // the offset is snapped to a page boundary via integer division. A
        // negative offset is clamped to 0; without the clamp it would wrap
        // through u64 and overflow `page_num * page_size` (panic in debug,
        // negative SQL OFFSET in release), both surfacing as a 500.
        let page_size = pagination
            .limit
            .filter(|&l| l > 0)
            .unwrap_or(crate::common::config::DEFAULT_PAGE_SIZE as i64)
            as u64;
        let page_num = (pagination.offset.unwrap_or(0).max(0) / page_size as i64) as u64;
        let sql_offset = page_num * page_size;

        // $1 is always user_id; search filters bind $2.. .
        let (where_clause, filter_binds) = build_affinity_filter(&search_dto, 2);

        // Count over the same filter so `count` and `results` agree. The count
        // does not join user_record_interaction, so it takes no user_id bind;
        // its filters start at $1 via a separate build_affinity_filter call.
        let (count_where, count_binds) = build_affinity_filter(&search_dto, 1);
        let count_sql = format!("SELECT COUNT(*) AS cnt FROM studio t{count_where}");

        // Per-user affinity score. `liked` counts only works that are BOTH
        // liked and viewed, so liked <= viewed <= total. The score combines two
        // Bayesian-shrunk rates with a logarithmic absolute-volume factor:
        //   ratio  = W_V * (viewed + M_V*C_V)/(total + M_V)
        //          + W_L * (liked  + M_L*C_L)/(viewed + M_L)
        //   volume = 1 + GAMMA * (ln(1+viewed) + BETA*ln(1+liked)) / ln(1+N0)
        //   score  = ratio * volume        (0 when total = 0)
        // Shrinkage pulls small-sample rates toward the prior so a single
        // viewed+liked work no longer scores a perfect 1.0; the volume factor
        // (with diminishing returns) lifts studios with many viewed/liked works.
        // The score is a relative ordering key and is NOT bounded to [0, 1].
        // The outer CASE guards total=0; both rate denominators are always
        // positive (M_V, M_L > 0) so there is no division by zero.
        // Studios relate to records via record.studio_id (foreign key), so the
        // aggregate groups records directly (no junction table).
        let select_sql = format!(
            "SELECT t.id, t.name, t.link, t.manual, \
             COALESCE( \
               CASE WHEN agg.total > 0 \
                    THEN ( {W_V} * ((agg.viewed::float8 + {M_V} * {C_V}) / (agg.total::float8 + {M_V})) \
                         + {W_L} * ((agg.liked::float8 + {M_L} * {C_L}) / (agg.viewed::float8 + {M_L})) ) \
                       * ( 1 + {GAMMA} * ( \
                             (ln(1 + agg.viewed::float8) + {BETA} * ln(1 + agg.liked::float8)) \
                             / ln(1 + {N0}) ) ) \
                    ELSE 0 END, 0) AS score \
             FROM studio t \
             LEFT JOIN ( \
               SELECT r.studio_id AS entity_id, \
                      COUNT(DISTINCT r.id) AS total, \
                      COUNT(DISTINCT r.id) FILTER (WHERE uri.viewed) AS viewed, \
                      COUNT(DISTINCT r.id) FILTER (WHERE uri.liked AND uri.viewed) AS liked \
               FROM record r \
               LEFT JOIN user_record_interaction uri \
                      ON uri.record_id = r.id AND uri.user_id = $1 \
               GROUP BY r.studio_id \
             ) agg ON agg.entity_id = t.id\
             {where_clause} \
             ORDER BY score DESC, t.id ASC \
             LIMIT ${limit_param} OFFSET ${offset_param}",
            W_V = AFFINITY_W_V,
            W_L = AFFINITY_W_L,
            C_V = AFFINITY_C_V,
            C_L = AFFINITY_C_L,
            M_V = AFFINITY_M_V,
            M_L = AFFINITY_M_L,
            BETA = AFFINITY_BETA,
            GAMMA = AFFINITY_GAMMA,
            N0 = AFFINITY_N0,
            limit_param = 2 + filter_binds.len(),
            offset_param = 3 + filter_binds.len(),
        );

        let mut select_binds: Vec<Value> = Vec::with_capacity(filter_binds.len() + 3);
        select_binds.push(user_id.into());
        select_binds.extend(filter_binds);
        select_binds.push((page_size as i64).into());
        select_binds.push((sql_offset as i64).into());

        let rows = AffinityStudioRow::find_by_statement(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            &select_sql,
            select_binds,
        ))
        .all(db)
        .await?;
        let results: Vec<Studio> = rows.into_iter().map(Studio::from).collect();

        let count_row = CountRow::find_by_statement(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            &count_sql,
            count_binds,
        ))
        .one(db)
        .await?;
        let total_items = count_row.map_or(0, |r| r.cnt);
        let total_pages = (total_items as u64).div_ceil(page_size);

        let next = if page_num + 1 < total_pages {
            Some(format!(
                "?limit={page_size}&offset={}",
                (page_num + 1) * page_size
            ))
        } else {
            None
        };
        let previous = if page_num > 0 {
            Some(format!(
                "?limit={page_size}&offset={}",
                (page_num - 1) * page_size
            ))
        } else {
            None
        };

        Ok(PaginatedResponse {
            count: total_items,
            next,
            previous,
            results,
        })
    }
}

/// Raw row for the affinity count query.
#[derive(Debug, FromQueryResult)]
struct CountRow {
    cnt: i64,
}
