//! Refresh token query operations

use chrono::{DateTime, Utc};
use common::AppResult;
use sqlx::PgPool;
use uuid::Uuid;

/// A row from the `refresh_tokens` table.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct RefreshTokenRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub token_hash: String,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

/// Insert a new refresh token.
pub async fn insert(pool: &PgPool, user_id: Uuid, token_hash: &str, expires_at: DateTime<Utc>) -> AppResult<Uuid> {
    let id = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO refresh_tokens (user_id, token_hash, expires_at)
            VALUES ($1, $2, $3)
            RETURNING id"#,
    )
    .bind(user_id)
    .bind(token_hash)
    .bind(expires_at)
    .fetch_one(pool)
    .await
    .map_err(common::AppError::from)?;

    Ok(id)
}

/// Find a refresh token by its hash.
pub async fn find_by_hash(pool: &PgPool, token_hash: &str) -> AppResult<Option<RefreshTokenRow>> {
    sqlx::query_as::<_, RefreshTokenRow>(
        r#"SELECT id, user_id, token_hash, expires_at, created_at
           FROM refresh_tokens
           WHERE token_hash = $1"#,
    )
    .bind(token_hash)
    .fetch_optional(pool)
    .await
    .map_err(common::AppError::from)
}

/// Delete a specific refresh token by its hash.
pub async fn delete_by_hash(pool: &PgPool, token_hash: &str) -> AppResult<()> {
    sqlx::query("DELETE FROM refresh_tokens WHERE token_hash = $1")
        .bind(token_hash)
        .execute(pool)
        .await
        .map_err(common::AppError::from)?;
    Ok(())
}

/// Delete all refresh tokens for a user.
pub async fn delete_all_for_user(pool: &PgPool, user_id: Uuid) -> AppResult<()> {
    sqlx::query("DELETE FROM refresh_tokens WHERE user_id = $1")
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(common::AppError::from)?;
    Ok(())
}

/// Delete expired refresh tokens (cleanup job).
pub async fn delete_expired(pool: &PgPool) -> AppResult<u64> {
    let result = sqlx::query("DELETE FROM refresh_tokens WHERE expires_at < NOW()")
        .execute(pool)
        .await
        .map_err(common::AppError::from)?;
    Ok(result.rows_affected())
}
