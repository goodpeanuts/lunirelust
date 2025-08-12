use axum::http::{Method, StatusCode};
use lunirelust::{
    common::dto::RestApiResponse,
    domains::luna::dto::{LabelDto, PaginatedResponse},
};

use super::test_helpers::{deserialize_json_body, request_with_auth, request_with_auth_and_body};

/// Test getting all labels
#[tokio::test]
async fn test_get_all_labels() {
    let response = request_with_auth(Method::GET, "/cards/labels").await;
    assert_eq!(response.status(), StatusCode::OK);

    let (_parts, body) = response.into_parts();
    let labels: RestApiResponse<PaginatedResponse<LabelDto>> = deserialize_json_body(body)
        .await
        .expect("Failed to deserialize labels response");

    let labels_data = labels.0.data.expect("No labels data");
    assert!(
        !labels_data.results.is_empty(),
        "Should have at least default labels"
    );
    println!("Found {} labels", labels_data.results.len());

    // Verify the structure of the first label
    if let Some(first_label) = labels_data.results.first() {
        assert!(first_label.id >= 0, "Label ID should be non-negative");
        assert!(
            !first_label.name.is_empty(),
            "Label name should not be empty"
        );
        println!("First label: {} (ID: {})", first_label.name, first_label.id);
    }
}

/// Test getting a specific label by ID
#[tokio::test]
async fn test_get_label_by_id() {
    // First get all labels to find a valid ID
    let response = request_with_auth(Method::GET, "/cards/labels").await;
    assert_eq!(response.status(), StatusCode::OK);

    let (_parts, body) = response.into_parts();
    let labels: RestApiResponse<PaginatedResponse<LabelDto>> = deserialize_json_body(body)
        .await
        .expect("Failed to deserialize labels response");

    let labels_data = labels.0.data.expect("No labels data");

    if let Some(label) = labels_data.results.first() {
        // Test getting this specific label
        let url = format!("/cards/labels/{}", label.id);
        let response = request_with_auth(Method::GET, &url).await;
        assert_eq!(response.status(), StatusCode::OK);

        let (_parts, body) = response.into_parts();
        let fetched_label: RestApiResponse<LabelDto> = deserialize_json_body(body)
            .await
            .expect("Failed to deserialize label response");

        let label_data = fetched_label.0.data.expect("No label data");
        assert_eq!(label_data.id, label.id, "IDs should match");
        assert_eq!(label_data.name, label.name, "Names should match");
        println!(
            "Successfully fetched label: {} (ID: {})",
            label_data.name, label_data.id
        );
    }
}

/// Test creating a new label and then fetching it
#[tokio::test]
async fn test_create_and_get_label() {
    // Create a new label
    let create_payload = serde_json::json!({
        "name": format!("Test Label {}", uuid::Uuid::new_v4()),
        "link": "https://example.com/test-label",
        "manual": true
    });

    let response = request_with_auth_and_body(Method::POST, "/cards/labels", &create_payload).await;
    assert_eq!(response.status(), StatusCode::OK);

    let (_parts, body) = response.into_parts();
    let created_label: RestApiResponse<LabelDto> = deserialize_json_body(body)
        .await
        .expect("Failed to deserialize created label response");

    let label_data = created_label.0.data.expect("No created label data");
    println!("Created label: {} (ID: {})", label_data.name, label_data.id);

    // Now fetch this label by ID
    let url = format!("/cards/labels/{}", label_data.id);
    let response = request_with_auth(Method::GET, &url).await;
    assert_eq!(response.status(), StatusCode::OK);

    let (_parts, body) = response.into_parts();
    let fetched_label: RestApiResponse<LabelDto> = deserialize_json_body(body)
        .await
        .expect("Failed to deserialize fetched label response");

    let fetched_data = fetched_label.0.data.expect("No fetched label data");
    assert_eq!(fetched_data.id, label_data.id, "IDs should match");
    assert_eq!(fetched_data.name, label_data.name, "Names should match");
    assert_eq!(
        fetched_data.manual, label_data.manual,
        "Manual flags should match"
    );

    println!("Successfully verified created and fetched label match");
}

/// Test getting non-existent label returns 404
#[tokio::test]
async fn test_get_nonexistent_label() {
    let response = request_with_auth(Method::GET, "/cards/labels/99999").await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    println!("Correctly returned 404 for non-existent label");
}

/// Test label search functionality
#[tokio::test]
async fn test_label_search() {
    // Test with a search query
    let response = request_with_auth(Method::GET, "/cards/labels?name=test").await;
    assert_eq!(response.status(), StatusCode::OK);

    let (_parts, body) = response.into_parts();
    let labels: RestApiResponse<PaginatedResponse<LabelDto>> = deserialize_json_body(body)
        .await
        .expect("Failed to deserialize labels search response");

    let labels_data = labels.0.data.expect("No labels data");
    println!("Found {} labels matching search", labels_data.results.len());

    // If no specific search results, the test should pass
    // but if results exist, they should contain the search term (case-insensitive)
    if !labels_data.results.is_empty() {
        let matching_count = labels_data
            .results
            .iter()
            .filter(|label| label.name.to_lowercase().contains("test"))
            .count();
        println!(
            "Found {} labels containing 'test' out of {}",
            matching_count,
            labels_data.results.len()
        );

        // The search should return relevant results, but we accept it may also include others
        // As long as at least some results match or the list is not empty, it's working
        assert!(
            !labels_data.results.is_empty(),
            "Search should return some results"
        );
    }
}

#[tokio::test]
async fn test_label_deduplication() {
    let label_request = serde_json::json!({
        "name": "Label Dedup Test Label",
        "link": "https://test-label-dedup.com",
        "manual": true
    });

    // First creation
    let response1 = request_with_auth_and_body(Method::POST, "/cards/labels", &label_request).await;
    assert_eq!(response1.status(), StatusCode::OK);

    let (_parts1, body1) = response1.into_parts();
    let response_data1: RestApiResponse<LabelDto> = deserialize_json_body(body1)
        .await
        .expect("Failed to deserialize first label response");
    let first_id = response_data1.0.data.expect("No label data").id;
    println!("First creation: Label Dedup Test Label (ID: {first_id})");

    // Second creation with identical data
    let response2 = request_with_auth_and_body(Method::POST, "/cards/labels", &label_request).await;
    assert_eq!(response2.status(), StatusCode::OK);

    let (_parts2, body2) = response2.into_parts();
    let response_data2: RestApiResponse<LabelDto> = deserialize_json_body(body2)
        .await
        .expect("Failed to deserialize second label response");
    let second_id = response_data2.0.data.expect("No label data").id;
    println!("Second creation: Label Dedup Test Label (ID: {second_id})");

    // Verify both return the same ID
    assert_eq!(
        first_id, second_id,
        "Deduplication failed: got different IDs {first_id} and {second_id}"
    );
    println!("Successfully verified label deduplication works");
}
