//! Search DTOs for request/response types.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::domains::search::SearchEntityType;

/// Query parameters for the search endpoint.
#[derive(Clone, Debug, Deserialize, ToSchema)]
pub struct SearchQuery {
    /// Search query string (required, must not be empty)
    pub q: String,
    /// Comma-separated entity types to filter by (e.g., "record,idol")
    #[serde(default)]
    pub entity_types: Option<String>,
    /// Filter by director name
    pub director: Option<String>,
    /// Filter by studio name
    pub studio: Option<String>,
    /// Filter by label name
    pub label: Option<String>,
    /// Filter by genre name
    pub genre: Option<String>,
    /// Filter by date range start (inclusive)
    pub date_from: Option<String>,
    /// Filter by date range end (inclusive)
    pub date_to: Option<String>,
    /// Maximum number of results to return (default 20)
    #[serde(default)]
    pub limit: Option<i64>,
    /// Number of results to skip (default 0)
    #[serde(default)]
    pub offset: Option<i64>,
}

/// A single search result item.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct SearchResultItem {
    /// Entity ID in the database
    pub id: String,
    /// Entity type
    pub entity_type: SearchEntityType,
    /// Display title
    pub title: String,
    /// Relevance score
    pub score: Option<f64>,
    /// Highlighted snippet with matching text wrapped in <em> tags
    pub highlight: Option<String>,
    /// Record-specific fields (null for named entities)
    pub date: Option<String>,
    pub director_name: Option<String>,
    pub studio_name: Option<String>,
    pub label_name: Option<String>,
    pub series_name: Option<String>,
    pub genre_names: Option<Vec<String>>,
    pub idol_names: Option<Vec<String>>,
}

/// Search response containing results and metadata.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct SearchResponse {
    /// Search mode used: `hybrid`, `keyword_only`, or `sql_fallback`
    pub search_mode: String,
    /// Total number of matching results
    pub total: i64,
    /// Maximum number of results per page
    pub limit: i64,
    /// Number of results skipped
    pub offset: i64,
    /// Search result items
    pub results: Vec<SearchResultItem>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_query_deserialize_required_only() {
        let query: SearchQuery =
            serde_json::from_str(r#"{"q":"test query"}"#).expect("deserializing SearchQuery");
        assert_eq!(query.q, "test query");
        assert!(query.entity_types.is_none());
        assert!(query.limit.is_none());
        assert!(query.offset.is_none());
    }

    #[test]
    fn test_search_query_deserialize_all_fields() {
        let query: SearchQuery = serde_json::from_str(
            r#"{"q":"test","entity_types":"record,idol","director":"Dir","limit":10,"offset":5}"#,
        )
        .expect("deserializing SearchQuery");
        assert_eq!(query.q, "test");
        assert_eq!(query.entity_types.as_deref(), Some("record,idol"));
        assert_eq!(query.director.as_deref(), Some("Dir"));
        assert_eq!(query.limit, Some(10));
        assert_eq!(query.offset, Some(5));
    }

    #[test]
    fn test_search_query_deserialize_missing_q_fails() {
        let result = serde_json::from_str::<SearchQuery>(r#"{"limit":10}"#);
        assert!(result.is_err());
    }

    #[test]
    fn test_search_response_serialize() {
        let response = SearchResponse {
            search_mode: "keyword_only".to_owned(),
            total: 42,
            limit: 20,
            offset: 0,
            results: vec![SearchResultItem {
                id: "abc".to_owned(),
                entity_type: SearchEntityType::Record,
                title: "Test".to_owned(),
                score: Some(0.95),
                highlight: Some("<em>Test</em>".to_owned()),
                date: None,
                director_name: None,
                studio_name: None,
                label_name: None,
                series_name: None,
                genre_names: None,
                idol_names: None,
            }],
        };
        let json = serde_json::to_value(&response).expect("serializing SearchResponse");
        assert_eq!(json["search_mode"], "keyword_only");
        assert_eq!(json["total"], 42);
        assert_eq!(
            json["results"]
                .as_array()
                .expect("results should be an array")
                .len(),
            1
        );
    }

    #[test]
    fn test_search_result_item_optional_fields() {
        let item = SearchResultItem {
            id: "rec1".to_owned(),
            entity_type: SearchEntityType::Record,
            title: "Title".to_owned(),
            score: None,
            highlight: None,
            date: Some("2024-01-01".to_owned()),
            director_name: Some("Dir".to_owned()),
            studio_name: None,
            label_name: None,
            series_name: None,
            genre_names: Some(vec!["Action".to_owned()]),
            idol_names: Some(vec!["Idol A".to_owned(), "Idol B".to_owned()]),
        };
        let json = serde_json::to_value(&item).expect("serializing SearchResultItem");
        assert_eq!(
            json["genre_names"]
                .as_array()
                .expect("genre_names should be an array")
                .len(),
            1
        );
        assert_eq!(
            json["idol_names"]
                .as_array()
                .expect("idol_names should be an array")
                .len(),
            2
        );
    }
}
