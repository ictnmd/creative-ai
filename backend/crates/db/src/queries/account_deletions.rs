//! Account deletion query operations

use chrono::{DateTime, Utc};
use common::AppResult;
use sqlx::PgPool;
use uuid::Uuid;

/// Insert a new account deletion record.
pub async fn insert(pool: &PgPool, user_id: Uuid, purge_after: DateTime<Utc>) -> AppResult<Uuid> {
    let id = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO account_deletions (id, user_id, requested_at, purge_after)
            VALUES ($1, $2, $3, $4)
            RETURNING id"#,
    )
    .bind(Uuid::new_v4())
    .bind(user_id)
    .bind(Utc::now())
    .bind(purge_after)
    .fetch_one(pool)
    .await
    .map_err(common::AppError::from)?;

    Ok(id)
}

/// Find the pending deletion for a user (if any).
pub async fn find_pending(pool: &PgPool, user_id: Uuid) -> AppResult<Option<AccountDeletionRow>> {
    sqlx::query_as::<_, AccountDeletionRow>(
        r#"SELECT id, user_id, requested_at, purge_after
           FROM account_deletions
           WHERE user_id = $1 AND cancelled_at IS NULL
           ORDER BY requested_at DESC
           LIMIT 1"#,
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(common::AppError::from)
}

/// Cancel a pending deletion (user changed their mind).
pub async fn cancel(pool: &PgPool, user_id: Uuid) -> AppResult<bool> {
    let result = sqlx::query(
        r#"UPDATE account_deletions
           SET cancelled_at = NOW()
           WHERE user_id = $1 AND cancelled_at IS NULL"#,
    )
    .bind(user_id)
    .execute(pool)
    .await
    .map_err(common::AppError::from)?;

    Ok(result.rows_affected() > 0)
}

/// Delete all expired account deletions and return affected rows.
pub async fn delete_expired(pool: &PgPool) -> AppResult<u64> {
    let result = sqlx::query(
        r#"DELETE FROM account_deletions WHERE purge_after < NOW() AND cancelled_at IS NULL"#,
    )
    .execute(pool)
    .await
    .map_err(common::AppError::from)?;

    Ok(result.rows_affected())
}

/// Account deletion row from the database.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AccountDeletionRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub requested_at: DateTime<Utc>,
    pub purge_after: DateTime<Utc>,
}
