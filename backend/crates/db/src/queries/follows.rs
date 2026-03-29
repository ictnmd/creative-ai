//! Follow relationships query operations

use chrono::{DateTime, Utc};
use common::{AppError, AppResult};
use sqlx::PgPool;
use uuid::Uuid;

/// User summary for follow list responses.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct UserSummary {
    pub id: Uuid,
    pub username: String,
    pub name: Option<String>,
    pub avatar_url: Option<String>,
}

/// Follow row from the follows table.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct FollowRow {
    pub follower_id: Uuid,
    pub following_id: Uuid,
    pub created_at: DateTime<Utc>,
}

/// Follow a user.
pub async fn follow_user(pool: &PgPool, follower_id: Uuid, following_id: Uuid) -> AppResult<()> {
    sqlx::query(
        r#"
        INSERT INTO follows (follower_id, following_id, created_at)
        VALUES ($1, $2, NOW())
        ON CONFLICT (follower_id, following_id) DO NOTHING
        "#,
    )
    .bind(follower_id)
    .bind(following_id)
    .execute(pool)
    .await
    .map_err(AppError::from)?;

    Ok(())
}

/// Unfollow a user.
pub async fn unfollow_user(pool: &PgPool, follower_id: Uuid, following_id: Uuid) -> AppResult<()> {
    sqlx::query(
        r#"
        DELETE FROM follows WHERE follower_id = $1 AND following_id = $2
        "#,
    )
    .bind(follower_id)
    .bind(following_id)
    .execute(pool)
    .await
    .map_err(AppError::from)?;

    Ok(())
}

/// Get the list of followers for a user.
pub async fn get_followers(
    pool: &PgPool,
    user_id: Uuid,
    limit: i64,
    offset: i64,
) -> AppResult<Vec<UserSummary>> {
    sqlx::query_as::<_, UserSummary>(
        r#"
        SELECT u.id, u.username, u.name, u.avatar_url
        FROM users u
        JOIN follows f ON f.follower_id = u.id
        WHERE f.following_id = $1
        ORDER BY f.created_at DESC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(user_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
    .map_err(AppError::from)
}

/// Get the list of users that a user is following.
pub async fn get_following(
    pool: &PgPool,
    user_id: Uuid,
    limit: i64,
    offset: i64,
) -> AppResult<Vec<UserSummary>> {
    sqlx::query_as::<_, UserSummary>(
        r#"
        SELECT u.id, u.username, u.name, u.avatar_url
        FROM users u
        JOIN follows f ON f.following_id = u.id
        WHERE f.follower_id = $1
        ORDER BY f.created_at DESC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(user_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
    .map_err(AppError::from)
}

/// Get the follower count for a user.
pub async fn get_follower_count(pool: &PgPool, user_id: Uuid) -> AppResult<i64> {
    sqlx::query_scalar(
        r#"
        SELECT COUNT(*) FROM follows WHERE following_id = $1
        "#,
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .map_err(AppError::from)
}

/// Get the following count for a user.
pub async fn get_following_count(pool: &PgPool, user_id: Uuid) -> AppResult<i64> {
    sqlx::query_scalar(
        r#"
        SELECT COUNT(*) FROM follows WHERE follower_id = $1
        "#,
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .map_err(AppError::from)
}

/// Check if a user is following another user.
pub async fn is_following(pool: &PgPool, follower_id: Uuid, following_id: Uuid) -> AppResult<bool> {
    sqlx::query_scalar(
        r#"
        SELECT EXISTS(SELECT 1 FROM follows WHERE follower_id = $1 AND following_id = $2)
        "#,
    )
    .bind(follower_id)
    .bind(following_id)
    .fetch_one(pool)
    .await
    .map_err(AppError::from)
}
