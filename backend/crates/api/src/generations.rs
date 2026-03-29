//! # Generation API Routes
//!
//! REST endpoints for creating, listing, retrieving, and deleting
//! image generation requests.

use crate::extractors::AuthUser;
use crate::ApiState;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use common::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Request / Response types
// ---------------------------------------------------------------------------

/// Request body for creating a new generation.
#[derive(Debug, Deserialize)]
pub struct CreateGenerationRequest {
    pub prompt: String,
    #[serde(default)]
    pub enhanced_prompt: Option<String>,
    #[serde(default = "default_provider")]
    pub provider: String,
    #[serde(default = "default_model")]
    pub model: String,
    #[serde(default)]
    pub style_preset_id: Option<Uuid>,
    #[serde(default)]
    pub reference_images: Vec<String>,
    #[serde(default)]
    pub sketch_data: Option<String>,
    #[serde(default = "default_aspect_ratio")]
    pub aspect_ratio: String,
    #[serde(default = "default_num_images")]
    pub num_images: u32,
    #[serde(default)]
    pub idempotency_key: Option<Uuid>,
}

fn default_provider() -> String {
    "openai".to_string()
}

fn default_model() -> String {
    "dall-e-3".to_string()
}

fn default_aspect_ratio() -> String {
    "1:1".to_string()
}

fn default_num_images() -> u32 {
    1
}

/// Response for a single generation.
#[derive(Debug, Serialize)]
pub struct GenerationResponse {
    pub id: Uuid,
    pub user_id: Uuid,
    pub prompt: String,
    pub enhanced_prompt: Option<String>,
    pub provider: String,
    pub model: String,
    pub style_preset_id: Option<Uuid>,
    pub reference_images: Vec<String>,
    pub output_urls: Vec<String>,
    pub status: String,
    pub error_message: Option<String>,
    pub credits_used: i32,
    pub visibility: String,
    pub view_count: i32,
    pub created_at: DateTime<Utc>,
}

impl From<db::queries::generations::GenerationRow> for GenerationResponse {
    fn from(row: db::queries::generations::GenerationRow) -> Self {
        Self {
            id: row.id,
            user_id: row.user_id,
            prompt: row.prompt,
            enhanced_prompt: row.enhanced_prompt,
            provider: row.provider,
            model: row.model,
            style_preset_id: row.style_preset_id,
            reference_images: row.reference_images,
            output_urls: row.output_urls,
            status: row.status,
            error_message: row.error_message,
            credits_used: row.credits_used,
            visibility: row.visibility,
            view_count: row.view_count,
            created_at: row.created_at,
        }
    }
}

/// Response for creating a generation (returns just the ID immediately).
#[derive(Debug, Serialize)]
pub struct CreateGenerationResponse {
    pub generation_id: Uuid,
    pub status: String,
    pub message: String,
}

/// Status-only response for polling.
#[derive(Debug, Serialize)]
pub struct GenerationStatusResponse {
    pub id: Uuid,
    pub status: String,
    pub output_urls: Vec<String>,
    pub error_message: Option<String>,
}

/// Pagination query params for listing generations.
#[derive(Debug, Deserialize)]
pub struct ListGenerationsQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub provider: Option<String>,
}

fn default_limit() -> i64 {
    20
}

/// Paginated list response.
#[derive(Debug, Serialize)]
pub struct PaginatedGenerationsResponse {
    pub items: Vec<GenerationResponse>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

/// Create the generations router.
pub fn create_generations_router(state: ApiState) -> Router {
    Router::new()
        .route("/api/v1/generations", post(create_generation))
        .route("/api/v1/generations", get(list_generations))
        .route("/api/v1/generations/:id", get(get_generation))
        .route("/api/v1/generations/:id", delete(delete_generation))
        .route("/api/v1/generations/:id/status", get(get_generation_status))
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// POST /api/v1/generations
///
/// Create a new generation request. Creates a DB record with status
/// "pending" and enqueues a job in DragonflyDB for async processing.
async fn create_generation(
    State(state): State<ApiState>,
    auth_user: AuthUser,
    Json(req): Json<CreateGenerationRequest>,
) -> AppResult<impl IntoResponse> {
    let user_id: Uuid = auth_user
        .user_id
        .parse()
        .map_err(|_| AppError::Validation("invalid user id".to_string()))?;

    // Validate prompt
    if req.prompt.trim().is_empty() {
        return Err(AppError::Validation("prompt cannot be empty".to_string()));
    }

    if req.prompt.len() > 10000 {
        return Err(AppError::Validation("prompt too long (max 10000 chars)".to_string()));
    }

    // Validate num_images range
    if req.num_images == 0 || req.num_images > 4 {
        return Err(AppError::Validation(
            "num_images must be between 1 and 4".to_string(),
        ));
    }

    let idempotency_key = req.idempotency_key.unwrap_or_else(Uuid::new_v4);

    // Create generation record in DB
    let generation_id = db::queries::generations::create(
        &state.inner.pool,
        &db::queries::generations::CreateGenerationParams {
            user_id,
            prompt: req.prompt.clone(),
            enhanced_prompt: req.enhanced_prompt.clone(),
            provider: req.provider.clone(),
            model: req.model.clone(),
            style_preset_id: req.style_preset_id,
            reference_images: req.reference_images.clone(),
            sketch_data: req.sketch_data.clone(),
            aspect_ratio: Some(req.aspect_ratio.clone()),
            num_images: req.num_images as i32,
            idempotency_key,
        },
    )
    .await?;

    // Enqueue job in DragonflyDB queue
    let queue = build_queue(&state)?;
    let job = generation::queue::GenerationJob {
        generation_id,
        user_id,
        prompt: req.prompt,
        enhanced_prompt: req.enhanced_prompt,
        provider: req.provider,
        model: req.model,
        style_preset_id: req.style_preset_id,
        reference_images: req.reference_images,
        sketch_data: req.sketch_data,
        aspect_ratio: req.aspect_ratio,
        num_images: req.num_images,
        idempotency_key,
        created_at: Utc::now(),
    };

    if let Err(e) = queue.enqueue(&job).await {
        tracing::error!(generation_id = %generation_id, error = %e, "failed to enqueue generation job");
        // Don't fail the request — the job is in DB with pending status
        // A retry mechanism can pick it up later
    }

    Ok((
        StatusCode::CREATED,
        Json(CreateGenerationResponse {
            generation_id,
            status: "pending".to_string(),
            message: "Generation queued".to_string(),
        }),
    ))
}

/// GET /api/v1/generations
///
/// List the authenticated user's generations with optional filtering.
async fn list_generations(
    State(state): State<ApiState>,
    auth_user: AuthUser,
    Query(query): Query<ListGenerationsQuery>,
) -> AppResult<impl IntoResponse> {
    let user_id: Uuid = auth_user
        .user_id
        .parse()
        .map_err(|_| AppError::Validation("invalid user id".to_string()))?;

    let limit = query.limit.clamp(1, 100);
    let offset = query.offset.max(0);

    let (rows, total) = tokio::try_join!(
        db::queries::generations::list(
            &state.inner.pool,
            user_id,
            query.status.as_deref(),
            query.provider.as_deref(),
            limit,
            offset,
        ),
        db::queries::generations::count(
            &state.inner.pool,
            user_id,
            query.status.as_deref(),
            query.provider.as_deref(),
        ),
    )?;

    Ok(Json(PaginatedGenerationsResponse {
        items: rows.into_iter().map(GenerationResponse::from).collect(),
        total,
        limit,
        offset,
    }))
}

/// GET /api/v1/generations/:id
///
/// Get a single generation by ID (owner only).
async fn get_generation(
    State(state): State<ApiState>,
    auth_user: AuthUser,
    Path(id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    let user_id: Uuid = auth_user
        .user_id
        .parse()
        .map_err(|_| AppError::Validation("invalid user id".to_string()))?;

    let row = db::queries::generations::get(&state.inner.pool, id, user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("generation not found".to_string()))?;

    // Increment view count
    sqlx::query("UPDATE generations SET view_count = view_count + 1 WHERE id = $1")
        .bind(id)
        .execute(&state.inner.pool)
        .await
        .ok();

    Ok(Json(GenerationResponse::from(row)))
}

/// DELETE /api/v1/generations/:id
///
/// Delete a generation (owner only). Removes from DB and S3.
async fn delete_generation(
    State(state): State<ApiState>,
    auth_user: AuthUser,
    Path(id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    let user_id: Uuid = auth_user
        .user_id
        .parse()
        .map_err(|_| AppError::Validation("invalid user id".to_string()))?;

    // Fetch the generation to know the S3 keys
    let row = db::queries::generations::get(&state.inner.pool, id, user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("generation not found".to_string()))?;

    // Delete from DB (returns true if deleted)
    let deleted = db::queries::generations::delete(&state.inner.pool, id, user_id).await?;
    if !deleted {
        return Err(AppError::NotFound("generation not found".to_string()));
    }

    // Delete images from S3
    for (i, _) in row.output_urls.iter().enumerate() {
        let s3_key = format!("generations/{}/{}.png", id, i);
        if let Err(e) = delete_s3_object(&state, &s3_key).await {
            tracing::warn!(generation_id = %id, key = %s3_key, error = %e, "failed to delete S3 object");
        }
    }

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "message": "generation deleted" })),
    ))
}

/// GET /api/v1/generations/:id/status
///
/// Poll the current status of a generation.
async fn get_generation_status(
    State(state): State<ApiState>,
    auth_user: AuthUser,
    Path(id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    let user_id: Uuid = auth_user
        .user_id
        .parse()
        .map_err(|_| AppError::Validation("invalid user id".to_string()))?;

    let row = db::queries::generations::get(&state.inner.pool, id, user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("generation not found".to_string()))?;

    Ok(Json(GenerationStatusResponse {
        id: row.id,
        status: row.status,
        output_urls: row.output_urls,
        error_message: row.error_message,
    }))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build a Queue instance from ApiState.
fn build_queue(state: &ApiState) -> AppResult<generation::queue::Queue> {
    let redis_guard = state.inner.redis.read();
    Ok(generation::queue::Queue::new(redis_guard.clone()))
}

/// Delete a single object from S3.
async fn delete_s3_object(state: &ApiState, key: &str) -> AppResult<()> {
    let storage = storage::S3Storage::new(storage::StorageConfig {
        endpoint: state.inner.config.s3_endpoint.clone(),
        access_key: state.inner.config.s3_access_key.clone(),
        secret_key: state.inner.config.s3_secret_key.clone(),
        bucket: state.inner.config.s3_bucket.clone(),
        region: "auto".to_string(),
    })
    .await?;
    storage.delete(key).await?;
    Ok(())
}
