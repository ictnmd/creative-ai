//! User profile API handlers.

use super::ApiState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use common::AppError;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::extractors::AuthUser;

/// Response body for a user profile.
#[derive(Debug, Serialize)]
pub struct UserProfileResponse {
    pub id: String,
    pub username: String,
    pub email: String,
    pub name: Option<String>,
    pub bio: Option<String>,
    pub avatar_url: Option<String>,
    pub is_public_profile: bool,
    pub is_verified: bool,
    pub role: String,
    pub created_at: String,
}

/// Request body for updating a user profile.
#[derive(Debug, Deserialize)]
pub struct UpdateProfileRequest {
    pub name: Option<String>,
    pub bio: Option<Option<String>>,
    pub avatar_url: Option<Option<String>>,
    pub is_public_profile: Option<bool>,
}

/// Create the users router.
pub fn create_users_router(state: ApiState) -> Router {
    Router::new()
        .route("/api/v1/user/profile", get(get_profile).put(update_profile))
        .route("/api/v1/users/:username", get(get_public_profile))
        .with_state(state)
}

/// GET /api/v1/user/profile - Get the current authenticated user's profile.
async fn get_profile(
    State(state): State<ApiState>,
    auth_user: AuthUser,
) -> Result<Json<UserProfileResponse>, AppError> {
    let user_id: Uuid = auth_user.user_id
        .parse()
        .map_err(|_| AppError::Validation("invalid user id in token".to_string()))?;

    let user = db::queries::users::get_user_by_id(&state.inner.pool, user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("user not found".to_string()))?;

    let profile = db::queries::users::get_profile(&state.inner.pool, user_id).await?;

    Ok(Json(UserProfileResponse {
        id: user.id.to_string(),
        username: user.username,
        email: user.email,
        name: user.name,
        bio: profile.as_ref().and_then(|p| p.bio.clone()),
        avatar_url: user.avatar_url,
        is_public_profile: profile.as_ref().map(|p| p.is_public_profile).unwrap_or(false),
        is_verified: false,
        role: user.role,
        created_at: user.created_at.to_rfc3339(),
    }))
}

/// PUT /api/v1/user/profile - Update the current user's profile.
async fn update_profile(
    State(state): State<ApiState>,
    auth_user: AuthUser,
    Json(req): Json<UpdateProfileRequest>,
) -> Result<impl IntoResponse, AppError> {
    let user_id: Uuid = auth_user.user_id
        .parse()
        .map_err(|_| AppError::Validation("invalid user id in token".to_string()))?;

    // Verify user exists.
    let _ = db::queries::users::get_user_by_id(&state.inner.pool, user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("user not found".to_string()))?;

    // Update user profile fields (name, avatar_url)
    db::queries::users::update_user_profile(
        &state.inner.pool,
        user_id,
        req.name.as_ref().map(|s| s.as_str()),
        req.avatar_url.as_ref().and_then(|o| o.as_ref()).map(|s| s.as_str()),
    )
    .await?;

    // Update profile settings (bio, is_public_profile)
    db::queries::users::update_profile_settings(
        &state.inner.pool,
        user_id,
        req.bio.as_ref().and_then(|o| o.as_ref()).map(|s| s.as_str()),
        req.is_public_profile.unwrap_or(true),
    )
    .await?;

    Ok((StatusCode::OK, Json(serde_json::json!({ "message": "profile updated" }))))
}

/// GET /api/v1/users/:username - Get a public user profile by username.
async fn get_public_profile(
    State(state): State<ApiState>,
    Path(username): Path<String>,
) -> Result<Json<UserProfileResponse>, AppError> {
    let user = db::queries::users::find_by_username(&state.inner.pool, &username)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("user '{}' not found", username)))?;

    let profile = db::queries::users::get_profile(&state.inner.pool, user.id).await?;

    let is_public_profile = profile.as_ref().map(|p| p.is_public_profile).unwrap_or(false);

    // Hide non-public profiles: return minimal info.
    if !is_public_profile {
        return Ok(Json(UserProfileResponse {
            id: user.id.to_string(),
            username: user.username,
            email: String::new(), // hidden for private profiles
            name: None,
            bio: None,
            avatar_url: None,
            is_public_profile: false,
            is_verified: false,
            role: String::new(),
            created_at: String::new(),
        }));
    }

    Ok(Json(UserProfileResponse {
        id: user.id.to_string(),
        username: user.username,
        email: user.email,
        name: user.name,
        bio: profile.as_ref().and_then(|p| p.bio.clone()),
        avatar_url: user.avatar_url,
        is_public_profile,
        is_verified: false,
        role: user.role,
        created_at: user.created_at.to_rfc3339(),
    }))
}
