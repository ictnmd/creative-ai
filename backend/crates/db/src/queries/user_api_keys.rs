//! User API key query operations
//!
//! Database operations for user-brought API keys (BYOK).

use chrono::{DateTime, Utc};
use common::AppResult;
use sqlx::PgPool;
use uuid::Uuid;

/// User API key row from the user_api_keys table.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct UserApiKeyRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub provider: String,
    pub encrypted_key: String,
    pub label: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

/// Find an active API key for a user and provider.
pub async fn find_active(
    pool: &PgPool,
    user_id: Uuid,
    provider: &str,
) -> AppResult<Option<UserApiKeyRow>> {
    sqlx::query_as::<_, UserApiKeyRow>(
        r#"
        SELECT id, user_id, provider, encrypted_key, label, is_active, created_at
        FROM user_api_keys
        WHERE user_id = $1 AND provider = $2 AND is_active = TRUE
        ORDER BY created_at DESC
        LIMIT 1
        "#,
    )
    .bind(user_id)
    .bind(provider)
    .fetch_optional(pool)
    .await
    .map_err(common::AppError::from)
}

/// Store a new user API key (key should already be encrypted by the caller).
pub async fn insert(
    pool: &PgPool,
    user_id: Uuid,
    provider: &str,
    encrypted_key: &str,
    label: Option<&str>,
) -> AppResult<Uuid> {
    let id: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO user_api_keys (id, user_id, provider, encrypted_key, label, is_active)
        VALUES (uuid_generate_v4(), $1, $2, $3, $4, TRUE)
        RETURNING id
        "#,
    )
    .bind(user_id)
    .bind(provider)
    .bind(encrypted_key)
    .bind(label)
    .fetch_one(pool)
    .await
    .map_err(common::AppError::from)?;

    Ok(id)
}

/// Deactivate a user API key.
pub async fn deactivate(pool: &PgPool, id: Uuid, user_id: Uuid) -> AppResult<bool> {
    let result = sqlx::query(
        r#"
        UPDATE user_api_keys SET is_active = FALSE
        WHERE id = $1 AND user_id = $2
        "#,
    )
    .bind(id)
    .bind(user_id)
    .execute(pool)
    .await
    .map_err(common::AppError::from)?;

    Ok(result.rows_affected() > 0)
}
