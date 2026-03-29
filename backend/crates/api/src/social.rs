//! # Social API Routes
//!
//! REST endpoints for follow relationships: follow, unfollow, list followers, list following.

use crate::extractors::AuthUser;
use crate::ApiState;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use common::AppError;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

/// Create the social/follow router.
pub fn create_social_router(state: ApiState) -> Router {
    Router::new()
        .route("/api/v1/users/:username/follow", post(follow_user))
        .route("/api/v1/users/:username/follow", delete(unfollow_user))
        .route("/api/v1/users/:username/followers", get(list_followers))
        .route("/api/v1/users/:username/following", get(list_following))
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Request / Response types
// ---------------------------------------------------------------------------

/// Query parameters for paginated lists.
#[derive(Debug, Deserialize)]
pub struct PaginationParams {
    #[serde(default = "default_page")]
    pub page: i64,
    #[serde(default = "default_limit")]
    pub limit: i64,
}

fn default_page() -> i64 {
    1
}

fn default_limit() -> i64 {
    20
}

/// A user summary returned in follow lists.
#[derive(Debug, Serialize)]
pub struct UserSummaryResponse {
    pub id: String,
    pub username: String,
    pub name: Option<String>,
    pub avatar_url: Option<String>,
}

/// Paginated list of user summaries.
#[derive(Debug, Serialize)]
pub struct FollowListResponse {
    pub users: Vec<UserSummaryResponse>,
    pub page: i64,
    pub limit: i64,
    pub total: i64,
}

/// Resolve a username to a user ID, returning a 404 if not found.
async fn resolve_username_to_id(
    pool: &sqlx::PgPool,
    username: &str,
) -> Result<Uuid, AppError> {
    let user = db::queries::users::find_by_username(pool, username)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("user '{}' not found", username)))?;
    Ok(user.id)
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// POST /api/v1/users/:username/follow
///
/// Follow a user by their username. Auth required.
async fn follow_user(
    State(state): State<ApiState>,
    auth_user: AuthUser,
    Path(username): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let follower_id: Uuid = auth_user
        .user_id
        .parse()
        .map_err(|_| AppError::Validation("invalid user id".to_string()))?;

    // Resolve target username to user ID
    let target_user_id = resolve_username_to_id(&state.inner.pool, &username).await?;

    // Prevent self-follow
    if follower_id == target_user_id {
        return Err(AppError::Validation("cannot follow yourself".to_string()));
    }

    db::queries::follows::follow_user(&state.inner.pool, follower_id, target_user_id).await?;

    Ok((StatusCode::OK, Json(serde_json::json!({ "message": "followed successfully" }))))
}

/// DELETE /api/v1/users/:username/follow
///
/// Unfollow a user by their username. Auth required.
async fn unfollow_user(
    State(state): State<ApiState>,
    auth_user: AuthUser,
    Path(username): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let follower_id: Uuid = auth_user
        .user_id
        .parse()
        .map_err(|_| AppError::Validation("invalid user id".to_string()))?;

    // Resolve target username to user ID
    let target_user_id = resolve_username_to_id(&state.inner.pool, &username).await?;

    db::queries::follows::unfollow_user(&state.inner.pool, follower_id, target_user_id).await?;

    Ok((StatusCode::OK, Json(serde_json::json!({ "message": "unfollowed successfully" }))))
}

/// GET /api/v1/users/:username/followers
///
/// List followers of a user with pagination.
async fn list_followers(
    State(state): State<ApiState>,
    Path(username): Path<String>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<FollowListResponse>, AppError> {
    let target_user_id = resolve_username_to_id(&state.inner.pool, &username).await?;

    let page = params.page.max(1);
    let limit = params.limit.clamp(1, 100);
    let offset = (page - 1) * limit;

    let users = db::queries::follows::get_followers(&state.inner.pool, target_user_id, limit, offset).await?;
    let total = db::queries::follows::get_follower_count(&state.inner.pool, target_user_id).await?;

    let users_response: Vec<UserSummaryResponse> = users
        .into_iter()
        .map(|u| UserSummaryResponse {
            id: u.id.to_string(),
            username: u.username,
            name: u.name,
            avatar_url: u.avatar_url,
        })
        .collect();

    Ok(Json(FollowListResponse {
        users: users_response,
        page,
        limit,
        total,
    }))
}

/// GET /api/v1/users/:username/following
///
/// List users that a user is following with pagination.
async fn list_following(
    State(state): State<ApiState>,
    Path(username): Path<String>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<FollowListResponse>, AppError> {
    let target_user_id = resolve_username_to_id(&state.inner.pool, &username).await?;

    let page = params.page.max(1);
    let limit = params.limit.clamp(1, 100);
    let offset = (page - 1) * limit;

    let users = db::queries::follows::get_following(&state.inner.pool, target_user_id, limit, offset).await?;
    let total = db::queries::follows::get_following_count(&state.inner.pool, target_user_id).await?;

    let users_response: Vec<UserSummaryResponse> = users
        .into_iter()
        .map(|u| UserSummaryResponse {
            id: u.id.to_string(),
            username: u.username,
            name: u.name,
            avatar_url: u.avatar_url,
        })
        .collect();

    Ok(Json(FollowListResponse {
        users: users_response,
        page,
        limit,
        total,
    }))
}
