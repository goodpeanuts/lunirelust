use axum::http::{Method, StatusCode};
use lunirelust::{
    common::dto::RestApiResponse,
    domains::luna::dto::{GenreDto, PaginatedResponse},
};

use super::test_helpers::{deserialize_json_body, request_with_auth, request_with_auth_and_body};

/// Test getting all genres
#[tokio::test]
async fn test_get_all_genres() {
    let response = request_with_auth(Method::GET, "/cards/genres").await;
    assert_eq!(response.status(), StatusCode::OK);

    let (_parts, body) = response.into_parts();
    let genres: RestApiResponse<PaginatedResponse<GenreDto>> = deserialize_json_body(body)
        .await
        .expect("Failed to deserialize genres response");

    let genres_data = genres.0.data.expect("No genres data");
    assert!(
        !genres_data.results.is_empty(),
        "Should have at least default genres"
    );
    println!("Found {} genres", genres_data.results.len());

    // Verify the structure of the first genre
    let first_genre = &genres_data.results[0];
    assert!(first_genre.id >= 0, "Genre ID should be non-negative");
    assert!(
        !first_genre.name.is_empty(),
        "Genre name should not be empty"
    );
    println!("First genre: {} (ID: {})", first_genre.name, first_genre.id);
}

/// Test getting a specific genre by ID
#[tokio::test]
async fn test_get_genre_by_id() {
    // First get all genres to find a valid ID
    let response = request_with_auth(Method::GET, "/cards/genres").await;
    assert_eq!(response.status(), StatusCode::OK);

    let (_parts, body) = response.into_parts();
    let genres: RestApiResponse<PaginatedResponse<GenreDto>> = deserialize_json_body(body)
        .await
        .expect("Failed to deserialize genres response");

    let genres_data = genres.0.data.expect("No genres data");
    if !genres_data.results.is_empty() {
        let genre_id = genres_data.results[0].id;

        // Now get this specific genre
        let url = format!("/cards/genres/{genre_id}");
        let response = request_with_auth(Method::GET, &url).await;
        assert_eq!(response.status(), StatusCode::OK);

        let (_parts, body) = response.into_parts();
        let genre: RestApiResponse<GenreDto> = deserialize_json_body(body)
            .await
            .expect("Failed to deserialize genre response");

        let genre_data = genre.0.data.expect("No genre data");
        assert_eq!(
            genre_data.id, genre_id,
            "Retrieved genre ID should match requested ID"
        );
        println!(
            "Successfully fetched genre: {} (ID: {})",
            genre_data.name, genre_data.id
        );
    }
}

/// Test getting a non-existent genre
#[tokio::test]
async fn test_get_nonexistent_genre() {
    let response = request_with_auth(Method::GET, "/cards/genres/999999").await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    println!("Correctly returned 404 for non-existent genre");
}

#[tokio::test]
async fn test_genre_deduplication() {
    let genre_request = serde_json::json!({
        "name": "Genre Dedup Test Genre",
        "link": "https://test-genre-dedup.com",
        "manual": true
    });

    // First creation
    let response1 = request_with_auth_and_body(Method::POST, "/cards/genres", &genre_request).await;
    assert_eq!(response1.status(), StatusCode::OK);

    let (_parts1, body1) = response1.into_parts();
    let response_data1: RestApiResponse<GenreDto> = deserialize_json_body(body1)
        .await
        .expect("Failed to deserialize first genre response");
    let first_id = response_data1.0.data.expect("No genre data").id;
    println!("First creation: Genre Dedup Test Genre (ID: {first_id})");

    // Second creation with identical data
    let response2 = request_with_auth_and_body(Method::POST, "/cards/genres", &genre_request).await;
    assert_eq!(response2.status(), StatusCode::OK);

    let (_parts2, body2) = response2.into_parts();
    let response_data2: RestApiResponse<GenreDto> = deserialize_json_body(body2)
        .await
        .expect("Failed to deserialize second genre response");
    let second_id = response_data2.0.data.expect("No genre data").id;
    println!("Second creation: Genre Dedup Test Genre (ID: {second_id})");

    // Verify both return the same ID
    assert_eq!(
        first_id, second_id,
        "Deduplication failed: got different IDs {first_id} and {second_id}"
    );
    println!("Successfully verified genre deduplication works");
}
