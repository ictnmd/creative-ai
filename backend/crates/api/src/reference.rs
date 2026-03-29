//! # Reference Image API
//!
//! HTTP endpoints for uploading reference images used in generation requests.
//! Provides both presigned S3 upload URLs (client-side upload) and
//! server-side URL fetching.

use axum::{
    extract::State,
    routing::post,
    Json, Router,
};
use common::AppError;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::extractors::AuthUser;
use super::ApiState;

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

/// Create the reference image upload router.
pub fn create_reference_router(state: ApiState) -> Router {
    Router::new()
        .route("/api/v1/reference/upload", post(get_upload_url))
        .route("/api/v1/reference/from-url", post(upload_from_url))
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Request/Response Types
// ---------------------------------------------------------------------------

/// Request to get a presigned upload URL.
#[derive(Debug, Deserialize)]
pub struct UploadUrlRequest {
    pub filename: String,
    pub content_type: String,
}

/// Response with presigned upload URL and object key.
#[derive(Debug, Serialize)]
pub struct UploadUrlResponse {
    pub upload_url: String,
    pub object_key: String,
}

/// Request to fetch an image from a URL and upload to S3.
#[derive(Debug, Deserialize)]
pub struct FromUrlRequest {
    pub url: String,
}

/// Response after uploading an image from URL.
#[derive(Debug, Serialize)]
pub struct FromUrlResponse {
    pub object_key: String,
    pub url: String,
    pub content_type: Option<String>,
    pub size_bytes: usize,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// POST /api/v1/reference/upload
///
/// Get a presigned S3 URL for client-side reference image upload.
///
/// The client uploads directly to S3 using the presigned PUT URL.
/// Object key: `references/{user_id}/{uuid}/{filename}`
async fn get_upload_url(
    State(state): State<ApiState>,
    auth_user: AuthUser,
    Json(req): Json<UploadUrlRequest>,
) -> Result<Json<UploadUrlResponse>, AppError> {
    let user_id: Uuid = auth_user
        .user_id
        .parse()
        .map_err(|_| AppError::Validation("invalid user id".to_string()))?;

    // Validate content type
    let allowed = ["image/jpeg", "image/png", "image/webp", "image/gif"];
    if !allowed.contains(&req.content_type.as_str()) {
        return Err(AppError::Validation(
            "content_type must be image/jpeg, image/png, image/webp, or image/gif".to_string(),
        ));
    }

    // Validate filename (sanitize)
    if req.filename.is_empty() || req.filename.len() > 255 {
        return Err(AppError::Validation("invalid filename".to_string()));
    }

    let object_id = Uuid::new_v4();
    let object_key = format!("references/{}/{}/{}", user_id, object_id, req.filename);

    // Generate presigned PUT URL
    let upload_url = generate_presigned_put_url(
        &state.inner.config,
        &object_key,
        &req.content_type,
        3600, // 1 hour expiry
    )
    .await?;

    Ok(Json(UploadUrlResponse {
        upload_url,
        object_key,
    }))
}

/// POST /api/v1/reference/from-url
///
/// Fetch an image from a URL and upload it to S3 server-side.
///
/// Limits: 10MB max file size.
async fn upload_from_url(
    State(state): State<ApiState>,
    auth_user: AuthUser,
    Json(req): Json<FromUrlRequest>,
) -> Result<Json<FromUrlResponse>, AppError> {
    let user_id: Uuid = auth_user
        .user_id
        .parse()
        .map_err(|_| AppError::Validation("invalid user id".to_string()))?;

    // Validate URL
    let parsed_url = reqwest::Url::parse(&req.url)
        .map_err(|_| AppError::Validation("invalid URL".to_string()))?;

    // Only allow http/https
    if !["http", "https"].contains(&parsed_url.scheme()) {
        return Err(AppError::Validation("URL must use http or https".to_string()));
    }

    // Fetch the image (with size limit)
    let response = reqwest::Client::new()
        .get(req.url.clone())
        .header("Accept", "image/*")
        .send()
        .await
        .map_err(|e| AppError::Http(e))?;

    if !response.status().is_success() {
        return Err(AppError::Validation("failed to fetch image from URL".to_string()));
    }

    let content_length = response.content_length().unwrap_or(0);
    const MAX_SIZE: u64 = 10 * 1024 * 1024; // 10MB

    if content_length > MAX_SIZE {
        return Err(AppError::Validation(
            "image exceeds maximum size of 10MB".to_string(),
        ));
    }

    let content_type: Option<String> = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let bytes: bytes::Bytes = response
        .bytes()
        .await
        .map_err(|e| AppError::Internal(format!("failed to read image bytes: {}", e)))?;

    if bytes.len() as u64 > MAX_SIZE {
        return Err(AppError::Validation(
            "image exceeds maximum size of 10MB".to_string(),
        ));
    }

    // Determine extension from content type
    let extension = match content_type.as_deref() {
        Some("image/jpeg") | Some("image/jpg") => "jpg",
        Some("image/png") => "png",
        Some("image/webp") => "webp",
        Some("image/gif") => "gif",
        _ => "bin",
    };

    let object_id = Uuid::new_v4();
    let object_key = format!("references/{}/{}/image.{}", user_id, object_id, extension);

    // Upload to S3
    upload_to_s3(
        &state.inner.config,
        &object_key,
        bytes.to_vec(),
        content_type.as_deref().unwrap_or("application/octet-stream"),
    )
    .await?;

    let public_url = format!(
        "{}/{}/{}",
        state.inner.config.s3_endpoint,
        state.inner.config.s3_bucket,
        object_key
    );

    Ok(Json(FromUrlResponse {
        object_key,
        url: public_url,
        content_type,
        size_bytes: bytes.len(),
    }))
}

// ---------------------------------------------------------------------------
// S3 Helpers
// ---------------------------------------------------------------------------

use aws_sdk_s3::config::Credentials;
use aws_sdk_s3::primitives::ByteStream;

async fn generate_presigned_put_url(
    config: &common::AppConfig,
    object_key: &str,
    content_type: &str,
    expires_in_secs: i64,
) -> Result<String, AppError> {
    let access_key = &config.s3_access_key;
    let secret_key = &config.s3_secret_key;
    let bucket = &config.s3_bucket;
    let region = &config.s3_region;

    let creds = Credentials::new(access_key, secret_key, None, None, "static");
    let region_obj = aws_types::region::Region::new(region.to_string());

    let cfg = aws_config::defaults(aws_config::BehaviorVersion::latest())
        .credentials_provider(creds)
        .region(region_obj)
        .load()
        .await;

    let client = aws_sdk_s3::Client::new(&cfg);

    let presigning_config = aws_sdk_s3::presigning::PresigningConfig::builder()
        .expires_in(std::time::Duration::from_secs(expires_in_secs as u64))
        .build()
        .map_err(|e| AppError::Aws(format!("failed to build presigning config: {}", e)))?;

    let presigned = client
        .put_object()
        .bucket(bucket)
        .key(object_key)
        .content_type(content_type)
        .presigned(presigning_config)
        .await
        .map_err(|e| AppError::Aws(format!("failed to generate presigned URL: {}", e)))?;

    Ok(presigned.uri().to_string())
}

async fn upload_to_s3(
    config: &common::AppConfig,
    object_key: &str,
    data: Vec<u8>,
    content_type: &str,
) -> Result<(), AppError> {
    let access_key = &config.s3_access_key;
    let secret_key = &config.s3_secret_key;
    let bucket = &config.s3_bucket;
    let region = &config.s3_region;

    let creds = Credentials::new(access_key, secret_key, None, None, "static");
    let region_obj = aws_types::region::Region::new(region.to_string());

    let cfg = aws_config::defaults(aws_config::BehaviorVersion::latest())
        .credentials_provider(creds)
        .region(region_obj)
        .load()
        .await;

    let client = aws_sdk_s3::Client::new(&cfg);
    let body = ByteStream::from(data);

    client
        .put_object()
        .bucket(bucket)
        .key(object_key)
        .body(body)
        .content_type(content_type)
        .send()
        .await
        .map_err(|e| AppError::Aws(format!("failed to upload to S3: {}", e)))?;

    Ok(())
}
