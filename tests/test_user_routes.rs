use axum::http::{Method, StatusCode};

use lunirelust::{
    common::{dto::RestApiResponse, error::AppError},
    domains::user::dto::user_dto::{CreateUserMultipartDto, SearchUserDto, UpdateUserDto, UserDto},
};

mod test_helpers;

use test_helpers::{
    deserialize_json_body, request_with_auth, request_with_auth_and_body,
    request_with_auth_and_multipart, TEST_USER_ID,
};

async fn create_user() -> Result<(CreateUserMultipartDto, UserDto), AppError> {
    let username = format!("testuser-{}", uuid::Uuid::new_v4());
    let email = format!("{username}@test.com");

    let payload = CreateUserMultipartDto {
        username,
        email,
        modified_by: TEST_USER_ID.to_owned(),
        profile_picture: None,
    };

    let multipart_body = format!(
        "------XYZ\r\nContent-Disposition: form-data; name=\"username\"\r\n\r\n{}\r\n------XYZ\r\nContent-Disposition: form-data; name=\"email\"\r\n\r\n{}\r\n------XYZ\r\nContent-Disposition: form-data; name=\"modified_by\"\r\n\r\n{}\r\n------XYZ--\r\n",
        payload.username, payload.email, payload.modified_by
    ).as_bytes().to_vec();

    let response = request_with_auth_and_multipart(Method::POST, "/user", multipart_body);

    let (parts, body) = response.await.into_parts();

    assert_eq!(parts.status, StatusCode::OK, "Expected status to be OK");

    let response_body: RestApiResponse<UserDto> = deserialize_json_body(body)
        .await
        .expect("Failed to deserialize user response body");

    assert_eq!(
        response_body.0.status,
        StatusCode::OK,
        "Expected response status to be OK"
    );
    let user_dto = response_body.0.data.expect("Failed to get user data");

    Ok((payload, user_dto))
}

#[tokio::test]
async fn test_create_user() {
    let created = create_user().await.expect("Failed to create user");

    let payload = created.0;
    let user_dto = created.1;

    assert!(!user_dto.id.is_empty());
    assert_eq!(user_dto.username, payload.username.clone());
    assert_eq!(user_dto.email, Some(payload.email.clone()));
    assert_ne!(user_dto.modified_by, Some(payload.modified_by.clone()));
    assert_eq!(user_dto.origin_file_name, None);
    assert!(user_dto.file_id.is_none());
}

async fn create_user_with_file() -> Result<(CreateUserMultipartDto, UserDto, String), AppError> {
    let username = format!("testuser-{}", uuid::Uuid::new_v4());
    let email = format!("{username}@test.com");

    let image_file = "cat.png";

    let payload = CreateUserMultipartDto {
        username,
        email,
        modified_by: TEST_USER_ID.to_owned(),
        // Indicate the file name being uploaded
        profile_picture: Some(image_file.to_owned()),
    };

    // Read the image file from the test/asset/ directory
    let file_path = format!("tests/asset/{image_file}");
    let file_bytes = std::fs::read(file_path)
        .unwrap_or_else(|_| panic!("Failed to read {image_file} from tests/asset/"));

    // Build the multipart body as a byte vector (Vec<u8>)
    let mut multipart_body = Vec::new();
    use std::io::Write as _;
    // Add the username part
    write!(
        &mut multipart_body,
        "------XYZ\r\nContent-Disposition: form-data; name=\"username\"\r\n\r\n{}\r\n",
        payload.username
    )
    .unwrap();
    // Add the email part
    write!(
        &mut multipart_body,
        "------XYZ\r\nContent-Disposition: form-data; name=\"email\"\r\n\r\n{}\r\n",
        payload.email
    )
    .unwrap();
    // Add the modified_by part
    write!(
        &mut multipart_body,
        "------XYZ\r\nContent-Disposition: form-data; name=\"modified_by\"\r\n\r\n{}\r\n",
        payload.modified_by
    )
    .unwrap();
    // Add the file part for profile_picture
    write!(
        &mut multipart_body,
        "------XYZ\r\nContent-Disposition: form-data; name=\"profile_picture\"; filename=\"{image_file}\"\r\nContent-Type: image/png\r\n\r\n"
    ).unwrap();
    multipart_body.extend_from_slice(&file_bytes);
    write!(&mut multipart_body, "\r\n").unwrap();
    // Add the final boundary
    write!(&mut multipart_body, "------XYZ--\r\n").unwrap();

    let response = request_with_auth_and_multipart(Method::POST, "/user", multipart_body);

    let (parts, body) = response.await.into_parts();

    assert_eq!(parts.status, StatusCode::OK, "Expected status to be OK");

    let response_body: RestApiResponse<UserDto> = deserialize_json_body(body)
        .await
        .expect("Failed to deserialize user response body");

    assert_eq!(
        response_body.0.status,
        StatusCode::OK,
        "Expected response status to be OK"
    );
    let user_dto = response_body.0.data.expect("Failed to get user data");

    Ok((payload, user_dto, image_file.to_owned()))
}

#[tokio::test]
async fn test_create_user_with_file() {
    let created = create_user_with_file()
        .await
        .expect("Failed to create user with file");

    let payload = created.0;
    let user_dto = created.1;
    let image_file = created.2;

    assert!(!user_dto.id.is_empty());
    assert_eq!(user_dto.username, payload.username.clone());
    assert_eq!(user_dto.email, Some(payload.email.clone()));
    assert_ne!(user_dto.modified_by, Some(payload.modified_by.clone()));
    assert_eq!(user_dto.origin_file_name, Some(image_file.clone()));
    assert!(!user_dto.file_id.clone().unwrap_or_default().is_empty());
}

#[tokio::test]
async fn test_get_users() {
    let response = request_with_auth(Method::GET, "/user");

    let (parts, body) = response.await.into_parts();

    assert_eq!(parts.status, StatusCode::OK);

    let response_body: RestApiResponse<Vec<UserDto>> = deserialize_json_body(body)
        .await
        .expect("Failed to deserialize users response body");

    assert_eq!(response_body.0.status, StatusCode::OK);

    let user_dtos = response_body.0.data.expect("Failed to get users data");

    // println!("user_dtos: {:?}", user_dtos);
    assert!(!user_dtos.is_empty());
}

#[tokio::test]
async fn test_get_user_list() {
    let username = "user0".to_owned();

    let payload = SearchUserDto {
        username: Some(username),
        id: None,
        email: None,
    };

    let response = request_with_auth_and_body(Method::POST, "/user/list", &payload);

    let (parts, body) = response.await.into_parts();

    assert_eq!(parts.status, StatusCode::OK);

    let response_body: RestApiResponse<Vec<UserDto>> = deserialize_json_body(body)
        .await
        .expect("Failed to deserialize users response body");

    assert_eq!(response_body.0.status, StatusCode::OK);

    let user_dtos = response_body.0.data.expect("Failed to get users data");

    // println!("user_dtos: {:?}", user_dtos);
    assert!(!user_dtos.is_empty());
}

#[tokio::test]
async fn test_get_user_by_id() {
    let created = create_user().await.expect("Failed to create user");

    let existent_user = created.1;
    let existent_id = existent_user.id;

    let url = format!("/user/{existent_id}");
    let response = request_with_auth(Method::GET, url.as_str());

    let (parts, body) = response.await.into_parts();

    assert_eq!(parts.status, StatusCode::OK);

    let response_body: RestApiResponse<UserDto> = deserialize_json_body(body)
        .await
        .expect("Failed to deserialize user response body");

    assert_eq!(response_body.0.status, StatusCode::OK);
    let user_dto = response_body.0.data.expect("Failed to get user data");

    assert_eq!(user_dto.id, *existent_id);
    assert_eq!(user_dto.username, existent_user.username);
    assert_eq!(user_dto.email, existent_user.email);
    assert_eq!(user_dto.created_by, existent_user.created_by);
    assert_eq!(user_dto.created_at, existent_user.created_at);
    assert_eq!(user_dto.modified_by, existent_user.modified_by);
    assert_eq!(user_dto.modified_at, existent_user.modified_at);
    assert_eq!(user_dto.file_id, existent_user.file_id);
    assert_eq!(user_dto.origin_file_name, existent_user.origin_file_name);
}

#[tokio::test]
async fn test_update_user() {
    let created = create_user().await.expect("Failed to create user");

    let existent_user = created.1;
    let existent_id = existent_user.id;

    let username = format!("update-testuser-{}", uuid::Uuid::new_v4());
    let email = format!("{username}@test.com");

    let payload = UpdateUserDto {
        username,
        email,
        modified_by: TEST_USER_ID.to_owned(),
    };

    let url = format!("/user/{existent_id}");

    let response = request_with_auth_and_body(Method::PUT, url.as_str(), &payload);

    let (parts, body) = response.await.into_parts();

    assert_eq!(parts.status, StatusCode::OK);

    let response_body: RestApiResponse<UserDto> = deserialize_json_body(body)
        .await
        .expect("Failed to deserialize user response body");

    assert_eq!(response_body.0.status, StatusCode::OK);
    let user_dto = response_body.0.data.expect("Failed to get user data");

    assert_eq!(user_dto.id, *existent_id);
    assert_eq!(user_dto.username, payload.username);
    assert_eq!(user_dto.email, Some(payload.email));
}

#[tokio::test]
async fn test_delete_user_not_found() {
    let non_existent_id = uuid::Uuid::new_v4();

    let url = format!("/user/{non_existent_id}");
    let response = request_with_auth(Method::DELETE, url.as_str());

    let (parts, body) = response.await.into_parts();

    assert_eq!(parts.status, StatusCode::NOT_FOUND);

    let response_body: RestApiResponse<()> = deserialize_json_body(body)
        .await
        .expect("Failed to deserialize response body");

    assert_eq!(response_body.0.status, StatusCode::NOT_FOUND);
    // println!("response_body.0.status: {:?}", response_body.0.status);
    // println!("response_body.0.message: {:?}", response_body.0.message);
}

#[tokio::test]
async fn test_delete_user() {
    let created = create_user()
        .await
        .expect("Failed to create user for deletion");

    let user = created.1;

    let url = format!("/user/{}", user.id);

    let response = request_with_auth(Method::DELETE, url.as_str());

    let (parts, body) = response.await.into_parts();

    assert_eq!(parts.status, StatusCode::OK);

    let response_body: RestApiResponse<()> = deserialize_json_body(body)
        .await
        .expect("Failed to deserialize response body");

    assert_eq!(response_body.0.status, StatusCode::OK);
    // println!("response_body.0.status: {:?}", response_body.0.status);
    // println!("response_body.0.message: {:?}", response_body.0.message);
}

#[tokio::test]
async fn test_delete_user_file() {
    let created = create_user_with_file()
        .await
        .expect("Failed to create user with file for deletion");
    let user_dto = created.1;
    let file_id = user_dto.file_id.clone().unwrap_or_default();

    let url = format!("/file/{file_id}");

    let response = request_with_auth(Method::DELETE, url.as_str());

    let (parts, body) = response.await.into_parts();

    assert_eq!(parts.status, StatusCode::OK);

    let response_body: RestApiResponse<()> = deserialize_json_body(body)
        .await
        .expect("Failed to deserialize response body");

    assert_eq!(response_body.0.status, StatusCode::OK);
    // println!("response_body.0.status: {:?}", response_body.0.status);
    // println!("response_body.0.message: {:?}", response_body.0.message);
}
