//! `SearchDocument` model for `MeiliSearch` index documents.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{json, Value as JsonValue};
use utoipa::ToSchema;

fn explicit_vector_opt_out() -> JsonValue {
    json!({
        "default": JsonValue::Null
    })
}

fn is_explicit_vector_opt_out(value: &JsonValue) -> bool {
    value.get("default").is_some_and(serde_json::Value::is_null)
}

fn serialize_vectors<S>(vectors: &Option<JsonValue>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match vectors {
        Some(value) => value.serialize(serializer),
        None => explicit_vector_opt_out().serialize(serializer),
    }
}

fn deserialize_vectors<'de, D>(deserializer: D) -> Result<Option<JsonValue>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<JsonValue>::deserialize(deserializer)?;
    Ok(match value {
        Some(value) if is_explicit_vector_opt_out(&value) => None,
        other => other,
    })
}

/// Represents a document in the `MeiliSearch` unified index `luna_search`.
/// Record documents include all name fields and permission. Named entities use `permission = 0`.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct SearchDocument {
    /// Unique document ID: "{`entity_type}__{entity_id`}" (e.g., "`record__abc123`")
    #[serde(rename = "id")]
    pub doc_id: String,
    /// Display title for the document
    pub title: String,
    /// Entity type: "record", "idol", "director", "genre", "label", "studio", "series"
    pub entity_type: SearchEntityType,
    /// Entity ID in the database
    pub entity_id: String,
    /// Monotonically increasing version for staleness detection
    pub entity_version: i64,
    /// Permission level: records use actual value, named entities use 0 (always visible)
    pub permission: i32,
    // Record-specific fields (null/empty for named entities)
    pub date: Option<String>,
    pub duration: Option<i32>,
    pub director_name: Option<String>,
    pub studio_name: Option<String>,
    pub label_name: Option<String>,
    pub series_name: Option<String>,
    pub genre_names: Option<Vec<String>>,
    pub idol_names: Option<Vec<String>>,
    /// Vector embeddings for semantic search (only for records)
    /// Serialized as `{"default": [...]}` keyed by embedder name.
    #[serde(
        default,
        rename = "_vectors",
        serialize_with = "serialize_vectors",
        deserialize_with = "deserialize_vectors"
    )]
    pub vectors: Option<JsonValue>,
}

/// Event types for outbox sync events.
// allow: test 构建下 dead_code 不触发，expect 会报 unfulfilled
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SyncEventType {
    Upsert,
    Delete,
}

/// Entity types in the search index.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum SearchEntityType {
    Record,
    Idol,
    Director,
    Genre,
    Label,
    Studio,
    Series,
}

impl SearchEntityType {
    /// All entity type variants, used for iteration.
    pub const ALL: &[Self] = &[
        Self::Record,
        Self::Idol,
        Self::Director,
        Self::Genre,
        Self::Label,
        Self::Studio,
        Self::Series,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Record => "record",
            Self::Idol => "idol",
            Self::Director => "director",
            Self::Genre => "genre",
            Self::Label => "label",
            Self::Studio => "studio",
            Self::Series => "series",
        }
    }

    pub fn parse_name(s: &str) -> Option<Self> {
        match s {
            "record" => Some(Self::Record),
            "idol" => Some(Self::Idol),
            "director" => Some(Self::Director),
            "genre" => Some(Self::Genre),
            "label" => Some(Self::Label),
            "studio" => Some(Self::Studio),
            "series" => Some(Self::Series),
            _ => None,
        }
    }
}

impl fmt::Display for SearchEntityType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for SearchEntityType {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse_name(s).ok_or_else(|| format!("unknown entity type: {s}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_document() -> SearchDocument {
        SearchDocument {
            doc_id: "record__abc123".to_owned(),
            title: "Test Record".to_owned(),
            entity_type: SearchEntityType::Record,
            entity_id: "abc123".to_owned(),
            entity_version: 1,
            permission: 0,
            date: Some("2024-01-01".to_owned()),
            duration: Some(120),
            director_name: Some("Director A".to_owned()),
            studio_name: None,
            label_name: None,
            series_name: None,
            genre_names: Some(vec!["Action".to_owned(), "Drama".to_owned()]),
            idol_names: None,
            vectors: None,
        }
    }

    #[test]
    fn test_search_document_serializes_doc_id_as_id() {
        let doc = sample_document();
        let json = serde_json::to_value(&doc).expect("serializing SearchDocument should not fail");
        assert!(
            json.get("id").is_some(),
            "serialized JSON should have 'id' field"
        );
        assert_eq!(json["id"], "record__abc123");
        assert!(
            json.get("doc_id").is_none(),
            "serialized JSON should not have 'doc_id' field"
        );
    }

    #[test]
    fn test_search_document_roundtrip_serde() {
        let doc = sample_document();
        let json = serde_json::to_string(&doc).expect("serializing SearchDocument should not fail");
        let deserialized: SearchDocument =
            serde_json::from_str(&json).expect("deserializing SearchDocument should not fail");
        assert_eq!(doc.doc_id, deserialized.doc_id);
        assert_eq!(doc.title, deserialized.title);
        assert_eq!(doc.entity_type, deserialized.entity_type);
        assert_eq!(doc.genre_names, deserialized.genre_names);
    }

    #[test]
    fn test_search_document_serializes_none_vectors_as_explicit_opt_out() {
        let doc = sample_document();
        let json = serde_json::to_value(&doc).expect("serializing SearchDocument should not fail");
        assert!(
            json.get("_vectors").is_some(),
            "_vectors should be present even when no embedding exists"
        );
        assert_eq!(
            json["_vectors"],
            serde_json::json!({"default": null}),
            "documents without embeddings must explicitly opt out for userProvided embedders"
        );
    }

    #[test]
    fn test_search_document_includes_vectors_when_present() {
        let mut doc = sample_document();
        doc.vectors = Some(serde_json::json!({"default": [0.1, 0.2, 0.3]}));
        let json = serde_json::to_value(&doc).expect("serializing SearchDocument should not fail");
        assert!(
            json.get("_vectors").is_some(),
            "_vectors should appear when set"
        );
    }

    #[test]
    fn test_search_document_deserializes_explicit_opt_out_as_none() {
        let json = serde_json::json!({
            "id": "record__abc123",
            "title": "Test Record",
            "entity_type": "record",
            "entity_id": "abc123",
            "entity_version": 1,
            "permission": 0,
            "_vectors": {
                "default": null
            }
        });

        let doc: SearchDocument =
            serde_json::from_value(json).expect("deserializing SearchDocument should not fail");

        assert!(doc.vectors.is_none());
    }

    #[test]
    fn test_search_entity_type_roundtrip() {
        let types = [
            "record", "idol", "director", "genre", "label", "studio", "series",
        ];
        for t in types {
            let et = SearchEntityType::from_str(t).expect("known entity type should parse");
            assert_eq!(et.as_str(), t);
        }
    }

    #[test]
    fn test_search_entity_type_unknown_returns_none() {
        assert!(SearchEntityType::from_str("unknown").is_err());
        assert!(SearchEntityType::from_str("").is_err());
    }

    #[test]
    fn test_sync_event_type_serde() {
        let upsert_json =
            serde_json::to_string(&SyncEventType::Upsert).expect("serializing SyncEventType");
        assert_eq!(upsert_json, "\"upsert\"");
        let delete_json =
            serde_json::to_string(&SyncEventType::Delete).expect("serializing SyncEventType");
        assert_eq!(delete_json, "\"delete\"");

        let parsed: SyncEventType =
            serde_json::from_str("\"upsert\"").expect("deserializing SyncEventType");
        assert_eq!(parsed, SyncEventType::Upsert);
    }
}
