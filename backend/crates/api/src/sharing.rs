//! # Sharing API
//!
//! HTTP endpoints for share links and public profiles.

use crate::ApiState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use common::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

/// Create the sharing router.
pub fn create_sharing_router(state: ApiState) -> Router {
    Router::new()
        // Authenticated endpoints
        .route("/api/v1/generations/:id/share", post(create_share_link))
        .route("/api/v1/generations/:id/share", delete(delete_share_link))
        // Public endpoints
        .route("/shared/:token", get(view_shared_generation))
        .route("/public/:username", get(view_public_profile))
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Request / Response types
// ---------------------------------------------------------------------------

/// Request body for creating/updating a share link.
#[derive(Debug, Deserialize)]
pub struct CreateShareLinkRequest {
    pub is_public: bool,
}

/// Response when a share link is created.
#[derive(Debug, Serialize)]
pub struct ShareLinkResponse {
    pub share_token: String,
    pub share_url: String,
    pub visibility: String,
}

/// Response when a share link is deleted.
#[derive(Debug, Serialize)]
pub struct ShareLinkDeletedResponse {
    pub message: String,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// POST /api/v1/generations/:id/share
///
/// Create or update a share link for a generation.
/// Updates the generation visibility based on `is_public`.
/// Auth required.
async fn create_share_link(
    State(state): State<ApiState>,
    auth_user: crate::extractors::AuthUser,
    Path(generation_id): Path<Uuid>,
    Json(req): Json<CreateShareLinkRequest>,
) -> AppResult<impl IntoResponse> {
    let user_id: Uuid = auth_user
        .user_id
        .parse()
        .map_err(|_| AppError::Validation("invalid user id".to_string()))?;

    // Verify the generation exists and belongs to the authenticated user.
    let gen = db::queries::generations::get(&state.inner.pool, generation_id, user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("generation not found".to_string()))?;

    // Only allow sharing completed generations.
    if gen.status != "completed" {
        return Err(AppError::Validation(
            "can only share completed generations".to_string(),
        ));
    }

    let visibility = if req.is_public { "public" } else { "shared" };

    let share_token = sharing::create_share_link(&state.inner.pool, generation_id, req.is_public)
        .await?;

    let share_url = format!("/shared/{}", share_token);

    tracing::info!(
        generation_id = %generation_id,
        user_id = %user_id,
        share_token = %share_token,
        visibility = %visibility,
        "share link created"
    );

    Ok((
        StatusCode::OK,
        Json(ShareLinkResponse {
            share_token,
            share_url,
            visibility: visibility.to_string(),
        }),
    ))
}

/// DELETE /api/v1/generations/:id/share
///
/// Remove the share link for a generation and set it back to private.
/// Auth required.
async fn delete_share_link(
    State(state): State<ApiState>,
    auth_user: crate::extractors::AuthUser,
    Path(generation_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    let user_id: Uuid = auth_user
        .user_id
        .parse()
        .map_err(|_| AppError::Validation("invalid user id".to_string()))?;

    // Verify the generation exists and belongs to the authenticated user.
    let _ = db::queries::generations::get(&state.inner.pool, generation_id, user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("generation not found".to_string()))?;

    sharing::delete_share_link(&state.inner.pool, generation_id).await?;

    tracing::info!(
        generation_id = %generation_id,
        user_id = %user_id,
        "share link deleted"
    );

    Ok((
        StatusCode::OK,
        Json(ShareLinkDeletedResponse {
            message: "share link removed".to_string(),
        }),
    ))
}

/// GET /shared/:token
///
/// View a shared generation by its share token.
/// Public endpoint — no authentication required.
/// Increments the view count.
async fn view_shared_generation(
    State(state): State<ApiState>,
    Path(token): Path<String>,
) -> AppResult<impl IntoResponse> {
    let view = sharing::get_shared_generation(&state.inner.pool, &token)
        .await?
        .ok_or_else(|| AppError::NotFound("shared generation not found".to_string()))?;

    // Increment view count asynchronously (fire-and-forget with tracing on error).
    let pool = state.inner.pool.clone();
    let generation_id = view.generation_id;
    tokio::spawn(async move {
        if let Err(e) = sharing::increment_view_count(&pool, generation_id).await {
            tracing::warn!(
                generation_id = %generation_id,
                error = %e,
                "failed to increment view count"
            );
        }
    });

    Ok(Json(view))
}

/// GET /public/:username
///
/// View a user's public profile with their public generations.
/// Public endpoint — no authentication required.
async fn view_public_profile(
    State(state): State<ApiState>,
    Path(username): Path<String>,
) -> AppResult<impl IntoResponse> {
    let profile = sharing::get_public_profile(&state.inner.pool, &username)
        .await?
        .ok_or_else(|| AppError::NotFound("public profile not found".to_string()))?;

    Ok(Json(profile))
}
