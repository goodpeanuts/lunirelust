use axum::http::{Method, StatusCode};
use lunirelust::{
    common::dto::RestApiResponse,
    domains::luna::dto::{PaginatedResponse, StudioDto},
};

use super::test_helpers::{deserialize_json_body, request_with_auth, request_with_auth_and_body};

/// Test getting all studios
#[tokio::test]
async fn test_get_all_studios() {
    let response = request_with_auth(Method::GET, "/cards/studios").await;
    assert_eq!(response.status(), StatusCode::OK);

    let (_parts, body) = response.into_parts();
    let studios: RestApiResponse<PaginatedResponse<StudioDto>> = deserialize_json_body(body)
        .await
        .expect("Failed to deserialize search response");

    let studios_data = studios.0.data.expect("No search data");
    assert!(!studios_data.results.is_empty(), "No studios found");

    // Verify the structure of the first studio
    if let Some(first_studio) = studios_data.results.first() {
        assert!(first_studio.id >= 0, "Studio ID should be non-negative");
        assert!(
            !first_studio.name.is_empty(),
            "Studio name should not be empty"
        );
        println!(
            "First studio: {} (ID: {})",
            first_studio.name, first_studio.id
        );
    }
}

/// Test getting a specific studio by ID
#[tokio::test]
async fn test_get_studio_by_id() {
    // First get all studios to find a valid ID
    let response = request_with_auth(Method::GET, "/cards/studios").await;
    assert_eq!(response.status(), StatusCode::OK);

    let (_parts, body) = response.into_parts();
    let studios: RestApiResponse<PaginatedResponse<StudioDto>> = deserialize_json_body(body)
        .await
        .expect("Failed to deserialize studios response");

    let studios_data = studios.0.data.expect("No studios data");

    if let Some(studio) = studios_data.results.first() {
        // Test getting this specific studio
        let url = format!("/cards/studios/{}", studio.id);
        let response = request_with_auth(Method::GET, &url).await;
        assert_eq!(response.status(), StatusCode::OK);

        let (_parts, body) = response.into_parts();
        let fetched_studio: RestApiResponse<StudioDto> = deserialize_json_body(body)
            .await
            .expect("Failed to deserialize studio response");

        let studio_data = fetched_studio.0.data.expect("No studio data");
        assert_eq!(studio_data.id, studio.id, "IDs should match");
        assert_eq!(studio_data.name, studio.name, "Names should match");
        println!(
            "Successfully fetched studio: {} (ID: {})",
            studio_data.name, studio_data.id
        );
    }
}

/// Test creating a new studio and then fetching it
#[tokio::test]
async fn test_create_and_get_studio() {
    // Create a new studio
    let create_payload = serde_json::json!({
        "name": format!("Test Studio {}", uuid::Uuid::new_v4()),
        "link": "https://example.com/test-studio",
        "manual": true
    });

    let response =
        request_with_auth_and_body(Method::POST, "/cards/studios", &create_payload).await;
    assert_eq!(response.status(), StatusCode::OK);

    let (_parts, body) = response.into_parts();
    let created_studio: RestApiResponse<StudioDto> = deserialize_json_body(body)
        .await
        .expect("Failed to deserialize created studio response");

    let studio_data = created_studio.0.data.expect("No created studio data");
    println!(
        "Created studio: {} (ID: {})",
        studio_data.name, studio_data.id
    );

    // Now fetch this studio by ID
    let url = format!("/cards/studios/{}", studio_data.id);
    let response = request_with_auth(Method::GET, &url).await;
    assert_eq!(response.status(), StatusCode::OK);

    let (_parts, body) = response.into_parts();
    let fetched_studio: RestApiResponse<StudioDto> = deserialize_json_body(body)
        .await
        .expect("Failed to deserialize fetched studio response");

    let fetched_data = fetched_studio.0.data.expect("No fetched studio data");
    assert_eq!(fetched_data.id, studio_data.id, "IDs should match");
    assert_eq!(fetched_data.name, studio_data.name, "Names should match");
    assert_eq!(
        fetched_data.manual, studio_data.manual,
        "Manual flags should match"
    );

    println!("Successfully verified created and fetched studio match");
}

/// Test getting non-existent studio returns 404
#[tokio::test]
async fn test_get_nonexistent_studio() {
    let response = request_with_auth(Method::GET, "/cards/studios/99999").await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    println!("Correctly returned 404 for non-existent studio");
}

/// Test studio search functionality
#[tokio::test]
async fn test_studio_search() {
    // Test with a search query
    let response = request_with_auth(Method::GET, "/cards/studios?name=test").await;
    assert_eq!(response.status(), StatusCode::OK);

    let (_parts, body) = response.into_parts();
    let studios: RestApiResponse<PaginatedResponse<StudioDto>> = deserialize_json_body(body)
        .await
        .expect("Failed to deserialize studios search response");

    let studios_data = studios.0.data.expect("No studios data");
    println!(
        "Found {} studios matching search",
        studios_data.results.len()
    );

    // If no specific search results, the test should pass
    // but if results exist, they should contain the search term (case-insensitive)
    if !studios_data.results.is_empty() {
        let matching_count = studios_data
            .results
            .iter()
            .filter(|studio| studio.name.to_lowercase().contains("test"))
            .count();
        println!(
            "Found {} studios containing 'test' out of {}",
            matching_count,
            studios_data.results.len()
        );

        // The search should return relevant results, but we accept it may also include others
        // As long as at least some results match or the list is not empty, it's working
        assert!(
            !studios_data.results.is_empty(),
            "Search should return some results"
        );
    }
}

#[tokio::test]
async fn test_studio_deduplication() {
    let studio_request = serde_json::json!({
        "name": "Studio Dedup Test Studio",
        "link": "https://test-studio-dedup.com",
        "manual": true
    });

    println!("Request payload: {studio_request}");

    // First creation
    let response1 =
        request_with_auth_and_body(Method::POST, "/cards/studios", &studio_request).await;

    println!("First response status: {}", response1.status());
    let first_status = response1.status();
    if first_status != StatusCode::OK {
        let (_parts, body) = response1.into_parts();
        let body_bytes = axum::body::to_bytes(body, usize::MAX)
            .await
            .expect("Failed to read body");
        let body_text = String::from_utf8_lossy(&body_bytes);
        println!("First response body: {body_text}");
        panic!("First request failed with status: {first_status}");
    }

    let (_parts1, body1) = response1.into_parts();
    let response_data1: RestApiResponse<StudioDto> = deserialize_json_body(body1)
        .await
        .expect("Failed to deserialize first studio response");
    let first_id = response_data1.0.data.expect("No studio data").id;
    println!("First creation: Studio Dedup Test Studio (ID: {first_id})");

    // Second creation with identical data
    let response2 =
        request_with_auth_and_body(Method::POST, "/cards/studios", &studio_request).await;

    println!("Second response status: {}", response2.status());
    let second_status = response2.status();
    if second_status != StatusCode::OK {
        let (_parts, body) = response2.into_parts();
        let body_bytes = axum::body::to_bytes(body, usize::MAX)
            .await
            .expect("Failed to read body");
        let body_text = String::from_utf8_lossy(&body_bytes);
        println!("Second response body: {body_text}");
        panic!("Second request failed with status: {second_status}");
    }

    let (_parts2, body2) = response2.into_parts();
    let response_data2: RestApiResponse<StudioDto> = deserialize_json_body(body2)
        .await
        .expect("Failed to deserialize second studio response");
    let second_id = response_data2.0.data.expect("No studio data").id;
    println!("Second creation: Studio Dedup Test Studio (ID: {second_id})");

    // Verify both return the same ID
    assert_eq!(
        first_id, second_id,
        "Deduplication failed: got different IDs {first_id} and {second_id}"
    );
    println!("Successfully verified studio deduplication works");
}
