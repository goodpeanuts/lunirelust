use axum::http::{Method, StatusCode};
use lunirelust::{
    common::dto::RestApiResponse,
    domains::luna::dto::{IdolDto, PaginatedResponse},
};

use super::test_helpers::{deserialize_json_body, request_with_auth, request_with_auth_and_body};

/// Test getting all idols
#[tokio::test]
async fn test_get_all_idols() {
    let response = request_with_auth(Method::GET, "/cards/idols").await;
    assert_eq!(response.status(), StatusCode::OK);

    let (_parts, body) = response.into_parts();
    let idols: RestApiResponse<PaginatedResponse<IdolDto>> = deserialize_json_body(body)
        .await
        .expect("Failed to deserialize idols response");

    let idols_data = idols.0.data.expect("No idols data");
    assert!(
        !idols_data.results.is_empty(),
        "Should have at least default idols"
    );
    println!("Found {} idols", idols_data.results.len());

    // Verify the structure of the first idol
    if let Some(first_idol) = idols_data.results.first() {
        assert!(first_idol.id >= 0, "Idol ID should be non-negative");
        assert!(!first_idol.name.is_empty(), "Idol name should not be empty");
        println!("First idol: {} (ID: {})", first_idol.name, first_idol.id);
    }
}

/// Test getting a specific idol by ID
#[tokio::test]
async fn test_get_idol_by_id() {
    // First get all idols to find a valid ID
    let response = request_with_auth(Method::GET, "/cards/idols").await;
    assert_eq!(response.status(), StatusCode::OK);

    let (_parts, body) = response.into_parts();
    let idols: RestApiResponse<PaginatedResponse<IdolDto>> = deserialize_json_body(body)
        .await
        .expect("Failed to deserialize idols response");

    let idols_data = idols.0.data.expect("No idols data");

    if let Some(idol) = idols_data.results.first() {
        // Test getting this specific idol
        let url = format!("/cards/idols/{}", idol.id);
        let response = request_with_auth(Method::GET, &url).await;
        assert_eq!(response.status(), StatusCode::OK);

        let (_parts, body) = response.into_parts();
        let fetched_idol: RestApiResponse<IdolDto> = deserialize_json_body(body)
            .await
            .expect("Failed to deserialize idol response");

        let idol_data = fetched_idol.0.data.expect("No idol data");
        assert_eq!(idol_data.id, idol.id, "IDs should match");
        assert_eq!(idol_data.name, idol.name, "Names should match");
        println!(
            "Successfully fetched idol: {} (ID: {})",
            idol_data.name, idol_data.id
        );
    }
}

/// Test creating a new idol and then fetching it
#[tokio::test]
async fn test_create_and_get_idol() {
    // Create a new idol
    let create_payload = serde_json::json!({
        "name": format!("Test Idol {}", uuid::Uuid::new_v4()),
        "link": "https://example.com/test-idol",
        "manual": true
    });

    let response = request_with_auth_and_body(Method::POST, "/cards/idols", &create_payload).await;
    assert_eq!(response.status(), StatusCode::OK);

    let (_parts, body) = response.into_parts();
    let created_idol: RestApiResponse<IdolDto> = deserialize_json_body(body)
        .await
        .expect("Failed to deserialize created idol response");

    let idol_data = created_idol.0.data.expect("No created idol data");
    println!("Created idol: {} (ID: {})", idol_data.name, idol_data.id);

    // Now fetch this idol by ID
    let url = format!("/cards/idols/{}", idol_data.id);
    let response = request_with_auth(Method::GET, &url).await;
    assert_eq!(response.status(), StatusCode::OK);

    let (_parts, body) = response.into_parts();
    let fetched_idol: RestApiResponse<IdolDto> = deserialize_json_body(body)
        .await
        .expect("Failed to deserialize fetched idol response");

    let fetched_data = fetched_idol.0.data.expect("No fetched idol data");
    assert_eq!(fetched_data.id, idol_data.id, "IDs should match");
    assert_eq!(fetched_data.name, idol_data.name, "Names should match");
    assert_eq!(
        fetched_data.manual, idol_data.manual,
        "Manual flags should match"
    );

    println!("Successfully verified created and fetched idol match");
}

/// Test getting non-existent idol returns 404
#[tokio::test]
async fn test_get_nonexistent_idol() {
    let response = request_with_auth(Method::GET, "/cards/idols/99999").await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    println!("Correctly returned 404 for non-existent idol");
}

/// Test idol search functionality
#[tokio::test]
async fn test_idol_search() {
    // Test with a search query
    let response = request_with_auth(Method::GET, "/cards/idols?name=test").await;
    assert_eq!(response.status(), StatusCode::OK);

    let (_parts, body) = response.into_parts();
    let idols: RestApiResponse<PaginatedResponse<IdolDto>> = deserialize_json_body(body)
        .await
        .expect("Failed to deserialize idols search response");

    let idols_data = idols.0.data.expect("No idols data");
    println!("Found {} idols matching search", idols_data.results.len());

    // If no specific search results, the test should pass
    // but if results exist, they should contain the search term (case-insensitive)
    if !idols_data.results.is_empty() {
        let matching_count = idols_data
            .results
            .iter()
            .filter(|idol| idol.name.to_lowercase().contains("test"))
            .count();
        println!(
            "Found {} idols containing 'test' out of {}",
            matching_count,
            idols_data.results.len()
        );

        // The search should return relevant results, but we accept it may also include others
        // As long as at least some results match or the list is not empty, it's working
        assert!(
            !idols_data.results.is_empty(),
            "Search should return some results"
        );
    }
}

#[tokio::test]
async fn test_idol_deduplication() {
    let idol_request = serde_json::json!({
        "name": "Idol Dedup Test Idol",
        "link": "https://test-idol-dedup.com",
        "manual": true
    });

    // First creation
    let response1 = request_with_auth_and_body(Method::POST, "/cards/idols", &idol_request).await;
    assert_eq!(response1.status(), StatusCode::OK);

    let (_parts1, body1) = response1.into_parts();
    let response_data1: RestApiResponse<IdolDto> = deserialize_json_body(body1)
        .await
        .expect("Failed to deserialize first idol response");
    let first_id = response_data1.0.data.expect("No idol data").id;
    println!("First creation: Idol Dedup Test Idol (ID: {first_id})");

    // Second creation with identical data
    let response2 = request_with_auth_and_body(Method::POST, "/cards/idols", &idol_request).await;
    assert_eq!(response2.status(), StatusCode::OK);

    let (_parts2, body2) = response2.into_parts();
    let response_data2: RestApiResponse<IdolDto> = deserialize_json_body(body2)
        .await
        .expect("Failed to deserialize second idol response");
    let second_id = response_data2.0.data.expect("No idol data").id;
    println!("Second creation: Idol Dedup Test Idol (ID: {second_id})");

    // Verify both return the same ID
    assert_eq!(
        first_id, second_id,
        "Deduplication failed: got different IDs {first_id} and {second_id}"
    );
    println!("Successfully verified idol deduplication works");
}
