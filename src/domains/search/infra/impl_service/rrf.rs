//! RRF (Reciprocal Rank Fusion) and `MeiliSearch` hit conversion utilities.

use std::str::FromStr as _;

use crate::domains::search::domain::repository::search_repo::SearchHit;
use crate::domains::search::dto::SearchResultItem;
use crate::domains::search::SearchEntityType;

/// Convert a search hit to a `SearchResultItem`.
pub(super) fn hit_to_search_item(hit: &SearchHit) -> SearchResultItem {
    let doc = &hit.document;

    // Extract highlight from formatted response
    let highlight = hit.formatted.as_ref().and_then(|f| {
        f.get("title")
            .and_then(|v| v.as_str())
            .map(|s| s.to_owned())
    });

    SearchResultItem {
        id: doc
            .get("entity_id")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_owned(),
        entity_type: doc
            .get("entity_type")
            .and_then(|v| v.as_str())
            .and_then(|s| SearchEntityType::from_str(s).ok())
            .unwrap_or(SearchEntityType::Record),
        title: doc
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_owned(),
        score: Some(hit.score),
        highlight,
        date: doc
            .get("date")
            .and_then(|v| v.as_str())
            .map(|s| s.to_owned()),
        director_name: doc
            .get("director_name")
            .and_then(|v| v.as_str())
            .map(|s| s.to_owned()),
        studio_name: doc
            .get("studio_name")
            .and_then(|v| v.as_str())
            .map(|s| s.to_owned()),
        label_name: doc
            .get("label_name")
            .and_then(|v| v.as_str())
            .map(|s| s.to_owned()),
        series_name: doc
            .get("series_name")
            .and_then(|v| v.as_str())
            .map(|s| s.to_owned()),
        genre_names: doc
            .get("genre_names")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_owned()))
                    .collect()
            }),
        idol_names: doc.get("idol_names").and_then(|v| v.as_array()).map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_owned()))
                .collect()
        }),
    }
}

/// RRF (Reciprocal Rank Fusion) to merge keyword and vector search results.
pub(super) fn rrf_fusion(
    keyword_hits: &[SearchHit],
    vector_hits: &[SearchHit],
    k: i64,
    limit: usize,
) -> Vec<SearchResultItem> {
    use std::collections::HashMap;

    let mut scores: HashMap<String, f64> = HashMap::new();
    let mut docs: HashMap<String, SearchHit> = HashMap::new();

    // Score keyword results
    for (rank, hit) in keyword_hits.iter().enumerate() {
        let score = 1.0 / (k as f64 + rank as f64 + 1.0);
        *scores.entry(hit.doc_id.clone()).or_default() += score;
        docs.entry(hit.doc_id.clone())
            .or_insert_with(|| hit.clone());
    }

    // Score vector results
    for (rank, hit) in vector_hits.iter().enumerate() {
        let score = 1.0 / (k as f64 + rank as f64 + 1.0);
        *scores.entry(hit.doc_id.clone()).or_default() += score;
        docs.entry(hit.doc_id.clone())
            .or_insert_with(|| hit.clone());
    }

    // Sort by combined score (descending)
    let mut ranked: Vec<_> = scores.into_iter().collect();
    ranked.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.0.cmp(&b.0)) // deterministic tie-break by doc ID
    });

    ranked
        .into_iter()
        .take(limit)
        .map(|(doc_id, score)| {
            let hit = docs.get(&doc_id).expect("doc should exist");
            let mut item = hit_to_search_item(hit);
            item.score = Some(score);
            item
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_hit(doc_id: &str, score: f64, entity_type: &str, title: &str) -> SearchHit {
        SearchHit {
            doc_id: doc_id.to_owned(),
            score,
            document: serde_json::json!({
                "id": doc_id,
                "entity_type": entity_type,
                "entity_id": doc_id.rsplit("__").next().unwrap_or(doc_id),
                "title": title,
            }),
            formatted: None,
        }
    }

    #[test]
    fn test_rrf_fusion_keyword_only() {
        let keyword_hits = vec![
            make_hit("record__1", 0.9, "record", "A"),
            make_hit("record__2", 0.8, "record", "B"),
        ];
        let vector_hits = vec![];
        let results = rrf_fusion(&keyword_hits, &vector_hits, 60, 10);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].id, "1");
        assert!(results[0].score.is_some());
    }

    #[test]
    fn test_rrf_fusion_vector_only() {
        let keyword_hits = vec![];
        let vector_hits = vec![make_hit("record__1", 0.9, "record", "A")];
        let results = rrf_fusion(&keyword_hits, &vector_hits, 60, 10);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_rrf_fusion_hybrid_boosts_overlap() {
        // doc appearing in both keyword and vector should rank highest
        let keyword_hits = vec![
            make_hit("record__1", 0.9, "record", "A"),
            make_hit("record__2", 0.8, "record", "B"),
        ];
        let vector_hits = vec![
            make_hit("record__2", 0.95, "record", "B"),
            make_hit("record__3", 0.7, "record", "C"),
        ];
        let results = rrf_fusion(&keyword_hits, &vector_hits, 60, 10);
        assert_eq!(results.len(), 3);
        // record__2 appears in both, should have highest combined score
        assert_eq!(results[0].id, "2");
    }

    #[test]
    fn test_rrf_fusion_respects_limit() {
        let hits: Vec<SearchHit> = (0..20)
            .map(|i| {
                make_hit(
                    &format!("record__{i}"),
                    1.0 - i as f64 * 0.01,
                    "record",
                    "T",
                )
            })
            .collect();
        let results = rrf_fusion(&hits, &[], 60, 5);
        assert_eq!(results.len(), 5);
    }

    #[test]
    fn test_hit_to_search_item_basic() {
        let hit = SearchHit {
            doc_id: "record__abc".to_owned(),
            score: 0.95,
            document: serde_json::json!({
                "entity_id": "abc",
                "entity_type": "record",
                "title": "Test Title",
                "date": "2024-01-01",
                "director_name": "Dir",
            }),
            formatted: Some(serde_json::json!({"title": "<em>Test</em> Title"})),
        };
        let item = hit_to_search_item(&hit);
        assert_eq!(item.id, "abc");
        assert_eq!(item.entity_type, SearchEntityType::Record);
        assert_eq!(item.title, "Test Title");
        assert_eq!(item.highlight.as_deref(), Some("<em>Test</em> Title"));
        assert_eq!(item.date.as_deref(), Some("2024-01-01"));
        assert_eq!(item.director_name.as_deref(), Some("Dir"));
    }

    #[test]
    fn test_hit_to_search_item_missing_fields() {
        let hit = SearchHit {
            doc_id: "genre__1".to_owned(),
            score: 0.5,
            document: serde_json::json!({
                "entity_id": "1",
                "entity_type": "genre",
                "title": "Action",
            }),
            formatted: None,
        };
        let item = hit_to_search_item(&hit);
        assert_eq!(item.id, "1");
        assert!(item.highlight.is_none());
        assert!(item.date.is_none());
        assert!(item.director_name.is_none());
    }

    #[test]
    fn test_hit_to_search_item_array_fields() {
        let hit = SearchHit {
            doc_id: "record__1".to_owned(),
            score: 0.8,
            document: serde_json::json!({
                "entity_id": "1",
                "entity_type": "record",
                "title": "Test",
                "genre_names": ["Action", "Drama"],
                "idol_names": ["Idol A"],
            }),
            formatted: None,
        };
        let item = hit_to_search_item(&hit);
        assert_eq!(
            item.genre_names.as_deref(),
            Some(&["Action".to_owned(), "Drama".to_owned()][..])
        );
        assert_eq!(item.idol_names.as_deref(), Some(&["Idol A".to_owned()][..]));
    }

    #[test]
    fn test_rrf_fusion_deduplicates_docs() {
        // Same doc in both branches should appear once with boosted score
        let keyword_hits = vec![make_hit("record__1", 0.9, "record", "A")];
        let vector_hits = vec![make_hit("record__1", 0.95, "record", "A")];
        let results = rrf_fusion(&keyword_hits, &vector_hits, 60, 10);
        assert_eq!(results.len(), 1);
        // Score should be sum of both RRF contributions
        let expected = 1.0 / (60.0 + 1.0) + 1.0 / (60.0 + 1.0);
        assert!((results[0].score.expect("score should exist") - expected).abs() < 1e-10);
    }

    #[test]
    fn test_rrf_fusion_preserves_order_for_non_overlapping() {
        // Non-overlapping sets should be ordered by individual rank
        let keyword_hits = vec![
            make_hit("record__1", 0.9, "record", "A"),
            make_hit("record__2", 0.8, "record", "B"),
        ];
        let vector_hits = vec![
            make_hit("record__3", 0.7, "record", "C"),
            make_hit("record__4", 0.6, "record", "D"),
        ];
        let results = rrf_fusion(&keyword_hits, &vector_hits, 60, 10);
        assert_eq!(results.len(), 4);
    }
}
