use axum::http::{Method, StatusCode};
use lunirelust::{common::dto::RestApiResponse, domains::luna::dto::EntityCountDto};

use super::test_helpers::{deserialize_json_body, request_with_auth};

/// Test getting director records count statistics
#[tokio::test]
async fn test_get_director_records_count() {
    // Act - Get director records count
    let response = request_with_auth(Method::GET, "/cards/director-records-count").await;

    // Assert - Verify the endpoint is accessible and returns proper structure
    let (parts, body) = response.into_parts();

    assert_eq!(
        parts.status,
        StatusCode::OK,
        "Expected director records count endpoint to return OK, got: {}",
        parts.status
    );

    let response_body: RestApiResponse<Vec<EntityCountDto>> = deserialize_json_body(body)
        .await
        .expect("Failed to deserialize director records count response");

    assert_eq!(
        response_body.0.status,
        StatusCode::OK,
        "Expected response status to be OK"
    );

    let data = response_body.0.data.expect("Should have data in response");

    // Verify response structure - should be a list of EntityCountDto
    for count_item in &data {
        assert!(count_item.id >= 0, "ID should be non-negative");
        assert!(!count_item.name.is_empty(), "Name should not be empty");
        assert!(count_item.count >= 0, "Count should be non-negative");
    }

    println!(
        "Successfully retrieved director records count with {} entries",
        data.len()
    );
}

/// Test getting genre records count statistics
#[tokio::test]
async fn test_get_genre_records_count() {
    // Act - Get genre records count
    let response = request_with_auth(Method::GET, "/cards/genre-records-count").await;

    // Assert - Verify the endpoint is accessible and returns proper structure
    let (parts, body) = response.into_parts();

    assert_eq!(
        parts.status,
        StatusCode::OK,
        "Expected genre records count endpoint to return OK, got: {}",
        parts.status
    );

    let response_body: RestApiResponse<Vec<EntityCountDto>> = deserialize_json_body(body)
        .await
        .expect("Failed to deserialize genre records count response");

    assert_eq!(
        response_body.0.status,
        StatusCode::OK,
        "Expected response status to be OK"
    );

    let data = response_body.0.data.expect("Should have data in response");

    // Verify response structure
    for count_item in &data {
        assert!(count_item.id >= 0, "ID should be non-negative");
        assert!(!count_item.name.is_empty(), "Name should not be empty");
        assert!(count_item.count >= 0, "Count should be non-negative");
    }

    println!(
        "Successfully retrieved genre records count with {} entries",
        data.len()
    );
}

/// Test getting label records count statistics
#[tokio::test]
async fn test_get_label_records_count() {
    // Act - Get label records count
    let response = request_with_auth(Method::GET, "/cards/label-records-count").await;

    // Assert - Verify the endpoint is accessible and returns proper structure
    let (parts, body) = response.into_parts();

    assert_eq!(
        parts.status,
        StatusCode::OK,
        "Expected label records count endpoint to return OK, got: {}",
        parts.status
    );

    let response_body: RestApiResponse<Vec<EntityCountDto>> = deserialize_json_body(body)
        .await
        .expect("Failed to deserialize label records count response");

    assert_eq!(
        response_body.0.status,
        StatusCode::OK,
        "Expected response status to be OK"
    );

    let data = response_body.0.data.expect("Should have data in response");

    // Verify response structure
    for count_item in &data {
        assert!(count_item.id >= 0, "ID should be non-negative");
        assert!(!count_item.name.is_empty(), "Name should not be empty");
        assert!(count_item.count >= 0, "Count should be non-negative");
    }

    println!(
        "Successfully retrieved label records count with {} entries",
        data.len()
    );
}

/// Test getting studio records count statistics
#[tokio::test]
async fn test_get_studio_records_count() {
    // Act - Get studio records count
    let response = request_with_auth(Method::GET, "/cards/studio-records-count").await;

    // Assert - Verify the endpoint is accessible and returns proper structure
    let (parts, body) = response.into_parts();

    assert_eq!(
        parts.status,
        StatusCode::OK,
        "Expected studio records count endpoint to return OK, got: {}",
        parts.status
    );

    let response_body: RestApiResponse<Vec<EntityCountDto>> = deserialize_json_body(body)
        .await
        .expect("Failed to deserialize studio records count response");

    assert_eq!(
        response_body.0.status,
        StatusCode::OK,
        "Expected response status to be OK"
    );

    let data = response_body.0.data.expect("Should have data in response");

    // Verify response structure
    for count_item in &data {
        assert!(count_item.id >= 0, "ID should be non-negative");
        assert!(!count_item.name.is_empty(), "Name should not be empty");
        assert!(count_item.count >= 0, "Count should be non-negative");
    }

    println!(
        "Successfully retrieved studio records count with {} entries",
        data.len()
    );
}

/// Test getting series records count statistics
#[tokio::test]
async fn test_get_series_records_count() {
    // Act - Get series records count
    let response = request_with_auth(Method::GET, "/cards/series-records-count").await;

    // Assert - Verify the endpoint is accessible and returns proper structure
    let (parts, body) = response.into_parts();

    assert_eq!(
        parts.status,
        StatusCode::OK,
        "Expected series records count endpoint to return OK, got: {}",
        parts.status
    );

    let response_body: RestApiResponse<Vec<EntityCountDto>> = deserialize_json_body(body)
        .await
        .expect("Failed to deserialize series records count response");

    assert_eq!(
        response_body.0.status,
        StatusCode::OK,
        "Expected response status to be OK"
    );

    let data = response_body.0.data.expect("Should have data in response");

    // Verify response structure
    for count_item in &data {
        assert!(count_item.id >= 0, "ID should be non-negative");
        assert!(!count_item.name.is_empty(), "Name should not be empty");
        assert!(count_item.count >= 0, "Count should be non-negative");
    }

    println!(
        "Successfully retrieved series records count with {} entries",
        data.len()
    );
}

/// Test getting idol records count statistics
#[tokio::test]
async fn test_get_idol_records_count() {
    // Act - Get idol records count
    let response = request_with_auth(Method::GET, "/cards/idol-records-count").await;

    // Assert - Verify the endpoint is accessible and returns proper structure
    let (parts, body) = response.into_parts();

    assert_eq!(
        parts.status,
        StatusCode::OK,
        "Expected idol records count endpoint to return OK, got: {}",
        parts.status
    );

    let response_body: RestApiResponse<Vec<EntityCountDto>> = deserialize_json_body(body)
        .await
        .expect("Failed to deserialize idol records count response");

    assert_eq!(
        response_body.0.status,
        StatusCode::OK,
        "Expected response status to be OK"
    );

    let data = response_body.0.data.expect("Should have data in response");

    // Verify response structure
    for count_item in &data {
        assert!(count_item.id >= 0, "ID should be non-negative");
        assert!(!count_item.name.is_empty(), "Name should not be empty");
        assert!(count_item.count >= 0, "Count should be non-negative");
    }

    println!(
        "Successfully retrieved idol records count with {} entries",
        data.len()
    );
}

/// Test all statistics endpoints in one comprehensive test
#[tokio::test]
async fn test_all_statistics_endpoints() {
    let endpoints = vec![
        "/cards/director-records-count",
        "/cards/genre-records-count",
        "/cards/label-records-count",
        "/cards/studio-records-count",
        "/cards/series-records-count",
        "/cards/idol-records-count",
    ];

    for endpoint in endpoints {
        println!("Testing endpoint: {endpoint}");

        let response = request_with_auth(Method::GET, endpoint).await;
        let (parts, body) = response.into_parts();

        assert_eq!(
            parts.status,
            StatusCode::OK,
            "Expected {} to return OK, got: {}",
            endpoint,
            parts.status
        );

        let response_body: Result<RestApiResponse<Vec<EntityCountDto>>, _> =
            deserialize_json_body(body).await;

        assert!(
            response_body.is_ok(),
            "Failed to deserialize response from {}: {:?}",
            endpoint,
            response_body.err()
        );

        let response_body = response_body.expect("Failed to deserialize response");
        assert_eq!(
            response_body.0.status,
            StatusCode::OK,
            "Expected response status to be OK for {endpoint}"
        );

        let data = response_body.0.data.expect("Should have data in response");
        println!(
            "Endpoint {} returned {} count entries",
            endpoint,
            data.len()
        );
    }

    println!("All statistics endpoints tested successfully!");
}
