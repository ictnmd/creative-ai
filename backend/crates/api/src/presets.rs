//! # Presets API
//!
//! HTTP endpoints for managing style presets (builtin + custom user presets).

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post, put},
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

/// Create the presets router.
pub fn create_presets_router(state: ApiState) -> Router {
    Router::new()
        .route("/api/v1/presets", get(list_presets).post(create_preset))
        .route("/api/v1/presets/:id", get(get_preset).put(update_preset).delete(delete_preset))
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Request/Response Types
// ---------------------------------------------------------------------------

/// Preset response returned to clients.
#[derive(Debug, Serialize)]
pub struct PresetResponse {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub prompt_suffix: String,
    pub thumbnail_url: Option<String>,
    pub is_public: bool,
    pub is_builtin: bool,
    pub creator_id: Option<String>,
    pub tags: Vec<String>,
    pub usage_count: i32,
    pub created_at: String,
}

/// Request to create a custom preset.
#[derive(Debug, Deserialize)]
pub struct CreatePresetRequest {
    pub name: String,
    pub prompt_suffix: String,
    pub description: Option<String>,
    pub thumbnail_url: Option<String>,
    pub tags: Option<Vec<String>>,
    pub aspect_ratio: Option<String>,
    pub style_type: Option<String>,
}

/// Request to update a preset.
#[derive(Debug, Deserialize)]
pub struct UpdatePresetRequest {
    pub name: Option<String>,
    pub description: Option<Option<String>>,
    pub prompt_suffix: Option<String>,
    pub thumbnail_url: Option<Option<String>>,
    pub tags: Option<Vec<String>>,
    pub aspect_ratio: Option<String>,
    pub style_type: Option<String>,
}

/// Map a preset row to its response type.
fn preset_to_response(row: db::queries::presets::PresetRow) -> PresetResponse {
    PresetResponse {
        id: row.id.to_string(),
        name: row.name,
        description: row.description,
        prompt_suffix: row.prompt_suffix,
        thumbnail_url: row.thumbnail_url,
        is_public: row.is_public,
        is_builtin: row.is_builtin,
        creator_id: row.creator_id.map(|id| id.to_string()),
        tags: row.tags,
        usage_count: row.usage_count,
        created_at: row.created_at.to_rfc3339(),
    }
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// GET /api/v1/presets
///
/// List presets accessible to the authenticated user.
/// Includes builtin presets and the user's own custom presets.
async fn list_presets(
    State(state): State<ApiState>,
    auth_user: AuthUser,
) -> Result<Json<Vec<PresetResponse>>, AppError> {
    let user_id: Uuid = auth_user
        .user_id
        .parse()
        .map_err(|_| AppError::Validation("invalid user id".to_string()))?;

    let presets = db::queries::presets::list_presets(&state.inner.pool, user_id).await?;

    let response: Vec<PresetResponse> = presets.into_iter().map(preset_to_response).collect();

    Ok(Json(response))
}

/// GET /api/v1/presets/:id
///
/// Get details of a specific preset.
async fn get_preset(
    State(state): State<ApiState>,
    Path(preset_id): Path<Uuid>,
) -> Result<Json<PresetResponse>, AppError> {
    let preset = db::queries::presets::get_preset(&state.inner.pool, preset_id)
        .await?
        .ok_or_else(|| AppError::NotFound("preset not found".to_string()))?;

    Ok(Json(preset_to_response(preset)))
}

/// POST /api/v1/presets
///
/// Create a custom preset. Requires authentication.
async fn create_preset(
    State(state): State<ApiState>,
    auth_user: AuthUser,
    Json(req): Json<CreatePresetRequest>,
) -> Result<(StatusCode, Json<PresetResponse>), AppError> {
    let user_id: Uuid = auth_user
        .user_id
        .parse()
        .map_err(|_| AppError::Validation("invalid user id".to_string()))?;

    // Validate name
    if req.name.is_empty() || req.name.len() > 100 {
        return Err(AppError::Validation("name must be between 1 and 100 characters".to_string()));
    }

    // Validate prompt_suffix
    if req.prompt_suffix.is_empty() {
        return Err(AppError::Validation("prompt_suffix cannot be empty".to_string()));
    }

    let tags = req.tags.unwrap_or_default();

    let preset_id = db::queries::presets::create_preset(
        &state.inner.pool,
        user_id,
        &req.name,
        req.description.as_deref(),
        &req.prompt_suffix,
        req.thumbnail_url.as_deref(),
        &tags,
    )
    .await?;

    let preset = db::queries::presets::get_preset(&state.inner.pool, preset_id)
        .await?
        .ok_or_else(|| AppError::Internal("preset creation failed".to_string()))?;

    tracing::info!(preset_id = %preset_id, user_id = %user_id, "custom preset created");

    Ok((StatusCode::CREATED, Json(preset_to_response(preset))))
}

/// PUT /api/v1/presets/:id
///
/// Update a preset. Only the owner can update it.
async fn update_preset(
    State(state): State<ApiState>,
    auth_user: AuthUser,
    Path(preset_id): Path<Uuid>,
    Json(req): Json<UpdatePresetRequest>,
) -> Result<impl IntoResponse, AppError> {
    let user_id: Uuid = auth_user
        .user_id
        .parse()
        .map_err(|_| AppError::Validation("invalid user id".to_string()))?;

    // Check if preset exists and is not builtin
    let existing = db::queries::presets::get_preset(&state.inner.pool, preset_id)
        .await?
        .ok_or_else(|| AppError::NotFound("preset not found".to_string()))?;

    if existing.is_builtin {
        return Err(AppError::Validation("cannot modify builtin presets".to_string()));
    }

    let updated = db::queries::presets::update_preset(
        &state.inner.pool,
        preset_id,
        user_id,
        req.name.as_deref(),
        req.description.as_ref().and_then(|o| o.as_ref()).map(|s| s.as_str()),
        req.prompt_suffix.as_deref(),
        req.thumbnail_url.as_ref().and_then(|o| o.as_ref()).map(|s| s.as_str()),
        req.tags.as_deref(),
    )
    .await?;

    if !updated {
        return Err(AppError::NotFound("preset not found or not owned by you".to_string()));
    }

    tracing::info!(preset_id = %preset_id, user_id = %user_id, "preset updated");

    Ok((StatusCode::OK, Json(serde_json::json!({ "message": "preset updated" }))))
}

/// DELETE /api/v1/presets/:id
///
/// Delete a preset. Only the owner can delete it, and builtin presets cannot be deleted.
async fn delete_preset(
    State(state): State<ApiState>,
    auth_user: AuthUser,
    Path(preset_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let user_id: Uuid = auth_user
        .user_id
        .parse()
        .map_err(|_| AppError::Validation("invalid user id".to_string()))?;

    // Check if preset is builtin
    let existing = db::queries::presets::get_preset(&state.inner.pool, preset_id)
        .await?
        .ok_or_else(|| AppError::NotFound("preset not found".to_string()))?;

    if existing.is_builtin {
        return Err(AppError::Validation("cannot delete builtin presets".to_string()));
    }

    let deleted = db::queries::presets::delete_preset(&state.inner.pool, preset_id, user_id).await?;

    if !deleted {
        return Err(AppError::NotFound("preset not found or not owned by you".to_string()));
    }

    tracing::info!(preset_id = %preset_id, user_id = %user_id, "preset deleted");

    Ok((StatusCode::OK, Json(serde_json::json!({ "message": "preset deleted" }))))
}
