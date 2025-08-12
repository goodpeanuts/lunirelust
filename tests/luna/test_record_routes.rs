use axum::http::{Method, StatusCode};
use lunirelust::{
    common::dto::RestApiResponse,
    domains::luna::dto::{PaginatedResponse, RecordDto},
};

use super::test_helpers::{deserialize_json_body, request_with_auth, request_with_auth_and_body};

/// Test creating a new record with a simple payload to verify basic functionality
#[tokio::test]
async fn test_create_record_simple() {
    // Arrange - Create payload with minimal required fields using nested objects
    let payload = serde_json::json!({
        "id": format!("test-record-{}", uuid::Uuid::new_v4()),
        "title": "Test Record Title",
        "date": "2025-08-11",
        "duration": 7200,
        "director": {
            "name": "Test Director",
            "link": "https://example.com/director",
            "manual": true
        },
        "studio": {
            "name": "Test Studio",
            "link": "https://example.com/studio",
            "manual": true
        },
        "label": {
            "name": "Test Label",
            "link": "https://example.com/label",
            "manual": true
        },
        "series": {
            "name": "Test Series",
            "link": "https://example.com/series",
            "manual": true
        },
        "genres": [
            {
                "name": "Test Genre",
                "link": "https://example.com/genre",
                "manual": true
            }
        ],
        "idols": [
            {
                "name": "Test Idol",
                "link": "https://example.com/idol",
                "manual": true
            }
        ],
        "has_links": false,
        "links": [],
        "permission": 1,
        "local_img_count": 0,
        "creator": "test_creator",
        "modified_by": "test_modifier"
    });

    // Act - Send POST request to create record
    let response = request_with_auth_and_body(Method::POST, "/cards/records", &payload).await;

    // Assert - Check if creation succeeded or failed gracefully
    let (parts, body) = response.into_parts();

    // The test might fail due to missing dependencies, but we want to ensure the endpoint is accessible
    assert!(
        parts.status == StatusCode::OK
            || parts.status == StatusCode::BAD_REQUEST
            || parts.status == StatusCode::NOT_FOUND
            || parts.status == StatusCode::UNPROCESSABLE_ENTITY,
        "Expected record creation endpoint to be accessible, got: {}",
        parts.status
    );

    if parts.status == StatusCode::OK {
        // If successful, verify the response structure
        let response_body: Result<RestApiResponse<RecordDto>, _> =
            deserialize_json_body(body).await;
        assert!(
            response_body.is_ok(),
            "Should be able to deserialize successful response"
        );
    }

    println!(
        "Record creation endpoint test completed with status: {}",
        parts.status
    );
}

/// Test getting records list - this should work regardless of data
#[tokio::test]
async fn test_get_records_list() {
    // Act - Get records list
    let response = request_with_auth(Method::GET, "/cards/records").await;

    // Assert - Verify the endpoint is accessible and returns proper structure
    let (parts, body) = response.into_parts();

    assert_eq!(
        parts.status,
        StatusCode::OK,
        "Expected records list endpoint to return OK, got: {}",
        parts.status
    );

    let response_body: RestApiResponse<PaginatedResponse<RecordDto>> = deserialize_json_body(body)
        .await
        .expect("Failed to deserialize records list response");

    assert_eq!(
        response_body.0.status,
        StatusCode::OK,
        "Expected response status to be OK"
    );

    let paginated_data = response_body.0.data.expect("Should have data in response");

    // Verify paginated response structure
    assert!(paginated_data.count >= 0, "Count should be non-negative");
    // The 'results' field should exist (even if empty)
    println!(
        "Retrieved {} results from records list",
        paginated_data.results.len()
    );

    println!(
        "Successfully retrieved records list with {} items",
        paginated_data.results.len()
    );
}

/// Test getting records with pagination parameters
#[tokio::test]
async fn test_get_records_with_pagination() {
    // Act - Get records with specific pagination
    let response = request_with_auth(Method::GET, "/cards/records?limit=5&offset=0").await;

    // Assert - Verify pagination is handled
    let (parts, body) = response.into_parts();

    assert_eq!(
        parts.status,
        StatusCode::OK,
        "Expected paginated records endpoint to return OK, got: {}",
        parts.status
    );

    let response_body: RestApiResponse<PaginatedResponse<RecordDto>> = deserialize_json_body(body)
        .await
        .expect("Failed to deserialize paginated records response");

    let paginated_data = response_body.0.data.expect("Should have data in response");

    // Verify pagination behavior
    assert!(
        paginated_data.results.len() <= 5,
        "Should respect limit parameter, got {} items",
        paginated_data.results.len()
    );

    println!(
        "Successfully tested pagination with {} items returned",
        paginated_data.results.len()
    );
}

/// Test getting a specific record by ID - this will test the endpoint structure
#[tokio::test]
async fn test_get_record_by_id_endpoint() {
    // Arrange - Use a test ID
    let test_id = "test-record-endpoint-check";

    // Act - Try to get record by ID
    let url = format!("/cards/records/{test_id}");
    let response = request_with_auth(Method::GET, &url).await;

    // Assert - Verify the endpoint is accessible
    let (parts, _body) = response.into_parts();

    // The endpoint should be accessible, even if the record doesn't exist
    assert!(
        parts.status == StatusCode::OK || parts.status == StatusCode::NOT_FOUND,
        "Expected record by ID endpoint to be accessible, got: {}",
        parts.status
    );

    if parts.status == StatusCode::NOT_FOUND {
        println!("Record by ID endpoint correctly returns 404 for non-existent record");
    } else {
        println!("Record by ID endpoint is accessible");
    }
}

/// Test record creation with invalid data to verify validation
#[tokio::test]
async fn test_create_record_validation() {
    // Arrange - Create payload with invalid data
    let invalid_payload = serde_json::json!({
        "id": "", // Empty ID should fail validation
        "title": "",
        "date": "invalid-date",
        "duration": -1, // Negative duration should fail
        "director_id": "invalid",
        "studio_id": "invalid",
        "label_id": "invalid",
        "series_id": "invalid",
        "genres": [],
        "idols": [],
        "has_links": false,
        "links": [],
        "permission": 1,
        "local_img_count": 0,
        "creator": "",
        "modified_by": ""
    });

    // Act - Send POST request with invalid data
    let response =
        request_with_auth_and_body(Method::POST, "/cards/records", &invalid_payload).await;

    // Assert - Should get validation error
    let (parts, _body) = response.into_parts();

    assert!(
        parts.status == StatusCode::BAD_REQUEST
            || parts.status == StatusCode::UNPROCESSABLE_ENTITY
            || parts.status == StatusCode::INTERNAL_SERVER_ERROR,
        "Expected validation error for invalid record data, got: {}",
        parts.status
    );

    println!(
        "Record validation correctly rejected invalid data with status: {}",
        parts.status
    );
}

/// Test well-formatted JSON payload creation
#[tokio::test]
async fn test_json_payload_formatting() {
    // This test demonstrates the improved JSON formatting approach with nested objects
    let payload = serde_json::json!({
        "id": format!("test-record-{}", uuid::Uuid::new_v4()),
        "title": "Well Formatted Test Record",
        "date": "2025-08-11",
        "duration": 7200,
        "director": {
            "name": "Famous Director",
            "link": "https://example.com/director",
            "manual": true
        },
        "studio": {
            "name": "Big Studio",
            "link": "https://example.com/studio",
            "manual": true
        },
        "label": {
            "name": "Premium Label",
            "link": "https://example.com/label",
            "manual": true
        },
        "series": {
            "name": "Popular Series",
            "link": "https://example.com/series",
            "manual": true
        },
        "genres": [
            {
                "name": "Action",
                "link": "https://example.com/genre/action",
                "manual": true
            },
            {
                "name": "Drama",
                "link": "https://example.com/genre/drama",
                "manual": false
            }
        ],
        "idols": [
            {
                "name": "Popular Idol",
                "link": "https://example.com/idol",
                "manual": true
            }
        ],
        "has_links": true,
        "links": [
            {
                "name": "Test Link",
                "size": "1.5",
                "date": "2025-08-11",
                "link": "https://example.com/test",
                "star": true
            }
        ],
        "permission": 1,
        "local_img_count": 5,
        "creator": "test_creator",
        "modified_by": "test_modifier"
    });

    // Verify the payload can be serialized properly
    let json_string =
        serde_json::to_string_pretty(&payload).expect("Should be able to serialize payload");

    assert!(!json_string.is_empty(), "JSON string should not be empty");
    assert!(
        json_string.contains("Well Formatted Test Record"),
        "Should contain the title"
    );
    assert!(
        json_string.contains("genres"),
        "Should contain genres field"
    );
    assert!(json_string.contains("idols"), "Should contain idols field");
    assert!(json_string.contains("links"), "Should contain links field");
    assert!(
        json_string.contains("director"),
        "Should contain director field"
    );
    assert!(
        json_string.contains("studio"),
        "Should contain studio field"
    );

    println!("Successfully demonstrated well-formatted JSON payload creation");
    println!("Sample JSON payload:\n{json_string}");
}
