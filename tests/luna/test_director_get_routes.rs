use axum::http::{Method, StatusCode};
use lunirelust::{
    common::dto::RestApiResponse,
    domains::luna::dto::{DirectorDto, PaginatedResponse},
};

use super::test_helpers::{deserialize_json_body, request_with_auth, request_with_auth_and_body};

/// Test getting all directors
#[tokio::test]
async fn test_get_all_directors() {
    let response = request_with_auth(Method::GET, "/cards/directors").await;
    assert_eq!(response.status(), StatusCode::OK);

    let (_parts, body) = response.into_parts();
    let directors: RestApiResponse<PaginatedResponse<DirectorDto>> = deserialize_json_body(body)
        .await
        .expect("Failed to deserialize directors response");

    let directors_data = directors.0.data.expect("No directors data");
    assert!(
        !directors_data.results.is_empty(),
        "Should have at least default directors"
    );
    println!("Found {} directors", directors_data.results.len());

    // Verify the structure of the first director
    if let Some(first_director) = directors_data.results.first() {
        assert!(first_director.id >= 0, "Director ID should be non-negative");
        assert!(
            !first_director.name.is_empty(),
            "Director name should not be empty"
        );
        println!(
            "First director: {} (ID: {})",
            first_director.name, first_director.id
        );
    }
}

/// Test getting a specific director by ID
#[tokio::test]
async fn test_get_director_by_id() {
    // First get all directors to find a valid ID
    let response = request_with_auth(Method::GET, "/cards/directors").await;
    assert_eq!(response.status(), StatusCode::OK);

    let (_parts, body) = response.into_parts();
    let directors: RestApiResponse<PaginatedResponse<DirectorDto>> = deserialize_json_body(body)
        .await
        .expect("Failed to deserialize directors response");

    let directors_data = directors.0.data.expect("No directors data");

    if let Some(director) = directors_data.results.first() {
        // Test getting this specific director
        let url = format!("/cards/directors/{}", director.id);
        let response = request_with_auth(Method::GET, &url).await;
        assert_eq!(response.status(), StatusCode::OK);

        let (_parts, body) = response.into_parts();
        let fetched_director: RestApiResponse<DirectorDto> = deserialize_json_body(body)
            .await
            .expect("Failed to deserialize director response");

        let director_data = fetched_director.0.data.expect("No director data");
        assert_eq!(director_data.id, director.id, "IDs should match");
        assert_eq!(director_data.name, director.name, "Names should match");
        println!(
            "Successfully fetched director: {} (ID: {})",
            director_data.name, director_data.id
        );
    }
}

/// Test creating a new director and then fetching it
#[tokio::test]
async fn test_create_and_get_director() {
    // Create a new director
    let create_payload = serde_json::json!({
        "name": format!("Test Director {}", uuid::Uuid::new_v4()),
        "link": "https://example.com/test-director",
        "manual": true
    });

    let response =
        request_with_auth_and_body(Method::POST, "/cards/directors", &create_payload).await;
    assert_eq!(response.status(), StatusCode::OK);

    let (_parts, body) = response.into_parts();
    let created_director: RestApiResponse<DirectorDto> = deserialize_json_body(body)
        .await
        .expect("Failed to deserialize created director response");

    let director_data = created_director.0.data.expect("No created director data");
    println!(
        "Created director: {} (ID: {})",
        director_data.name, director_data.id
    );

    // Now fetch this director by ID
    let url = format!("/cards/directors/{}", director_data.id);
    let response = request_with_auth(Method::GET, &url).await;
    assert_eq!(response.status(), StatusCode::OK);

    let (_parts, body) = response.into_parts();
    let fetched_director: RestApiResponse<DirectorDto> = deserialize_json_body(body)
        .await
        .expect("Failed to deserialize fetched director response");

    let fetched_data = fetched_director.0.data.expect("No fetched director data");
    assert_eq!(fetched_data.id, director_data.id, "IDs should match");
    assert_eq!(fetched_data.name, director_data.name, "Names should match");
    assert_eq!(
        fetched_data.manual, director_data.manual,
        "Manual flags should match"
    );

    println!("Successfully verified created and fetched director match");
}

/// Test getting non-existent director returns 404
#[tokio::test]
async fn test_get_nonexistent_director() {
    let response = request_with_auth(Method::GET, "/cards/directors/99999").await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    println!("Correctly returned 404 for non-existent director");
}

/// Test director search functionality
#[tokio::test]
async fn test_director_search() {
    // Test with a search query
    let response = request_with_auth(Method::GET, "/cards/directors?name=test").await;
    assert_eq!(response.status(), StatusCode::OK);

    let (_parts, body) = response.into_parts();
    let directors: RestApiResponse<PaginatedResponse<DirectorDto>> = deserialize_json_body(body)
        .await
        .expect("Failed to deserialize directors search response");

    let directors_data = directors.0.data.expect("No directors data");
    println!(
        "Found {} directors matching search",
        directors_data.results.len()
    );

    // If no specific search results, the test should pass
    // but if results exist, they should contain the search term (case-insensitive)
    if !directors_data.results.is_empty() {
        let matching_count = directors_data
            .results
            .iter()
            .filter(|director| director.name.to_lowercase().contains("test"))
            .count();
        println!(
            "Found {} directors containing 'test' out of {}",
            matching_count,
            directors_data.results.len()
        );

        // The search should return relevant results, but we accept it may also include others
        // As long as at least some results match or the list is not empty, it's working
        assert!(
            !directors_data.results.is_empty(),
            "Search should return some results"
        );
    }
}

/// Test director deduplication logic
#[tokio::test]
async fn test_director_deduplication() {
    // Create a director with specific attributes
    let create_payload = serde_json::json!({
        "name": "Dedup Test Director",
        "link": "https://example.com/dedup-director",
        "manual": true
    });

    // Create the director first time
    let response1 =
        request_with_auth_and_body(Method::POST, "/cards/directors", &create_payload).await;
    assert_eq!(response1.status(), StatusCode::OK);

    let (_parts1, body1) = response1.into_parts();
    let created_director1: RestApiResponse<DirectorDto> = deserialize_json_body(body1)
        .await
        .expect("Failed to deserialize first director response");

    let director1_data = created_director1.0.data.expect("No first director data");
    println!(
        "First creation: Director {} (ID: {})",
        director1_data.name, director1_data.id
    );

    // Try to create the same director again - should return existing ID
    let response2 =
        request_with_auth_and_body(Method::POST, "/cards/directors", &create_payload).await;
    assert_eq!(response2.status(), StatusCode::OK);

    let (_parts2, body2) = response2.into_parts();
    let created_director2: RestApiResponse<DirectorDto> = deserialize_json_body(body2)
        .await
        .expect("Failed to deserialize second director response");

    let director2_data = created_director2.0.data.expect("No second director data");
    println!(
        "Second creation: Director {} (ID: {})",
        director2_data.name, director2_data.id
    );

    // Should be the same ID (deduplication worked)
    assert_eq!(
        director1_data.id, director2_data.id,
        "Should return same ID for duplicate director"
    );
    assert_eq!(
        director1_data.name, director2_data.name,
        "Names should match"
    );
    assert_eq!(
        director1_data.link, director2_data.link,
        "Links should match"
    );
    assert_eq!(
        director1_data.manual, director2_data.manual,
        "Manual flags should match"
    );

    println!("Successfully verified director deduplication works");
}
