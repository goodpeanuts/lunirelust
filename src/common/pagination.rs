use crate::domains::luna::dto::{PaginatedResponse, PaginationQuery};

/// Generic pagination helper for in-memory slices.
///
/// Takes a full list of items, a pagination query, and a mapping function,
/// returns a `PaginatedResponse<U>` with the sliced results and next/previous URLs.
pub fn paginate<T, U>(
    items: Vec<T>,
    pagination: &PaginationQuery,
    map_fn: impl Fn(T) -> U,
) -> PaginatedResponse<U> {
    let limit = pagination.limit.unwrap_or(10) as usize;
    let offset = pagination.offset.unwrap_or(0) as usize;

    let total_count = items.len();
    let results: Vec<U> = items
        .into_iter()
        .skip(offset)
        .take(limit)
        .map(map_fn)
        .collect();

    let next = if offset + limit < total_count {
        Some(format!("?limit={limit}&offset={}", offset + limit))
    } else {
        None
    };

    let previous = if offset > 0 {
        Some(format!(
            "?limit={limit}&offset={}",
            offset.saturating_sub(limit)
        ))
    } else {
        None
    };

    PaginatedResponse {
        count: total_count as i64,
        next,
        previous,
        results,
    }
}
