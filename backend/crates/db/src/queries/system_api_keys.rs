//! System API key query operations
//!
//! Database operations for system-level API keys used when no user BYOK is set.

use chrono::{DateTime, Utc};
use common::AppResult;
use sqlx::PgPool;
use uuid::Uuid;

/// System API key row from the system_api_keys table.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct SystemApiKeyRow {
    pub id: Uuid,
    pub provider: String,
    pub encrypted_key: String,
    pub is_active: bool,
    pub rate_limit: i32,
    pub created_at: DateTime<Utc>,
}

/// Find an active system API key for a provider.
pub async fn find_active(pool: &PgPool, provider: &str) -> AppResult<Option<SystemApiKeyRow>> {
    sqlx::query_as::<_, SystemApiKeyRow>(
        r#"
        SELECT id, provider, encrypted_key, is_active, rate_limit, created_at
        FROM system_api_keys
        WHERE provider = $1 AND is_active = TRUE
        LIMIT 1
        "#,
    )
    .bind(provider)
    .fetch_optional(pool)
    .await
    .map_err(common::AppError::from)
}
