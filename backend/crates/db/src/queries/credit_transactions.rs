//! Credit transaction query operations

use chrono::{DateTime, Utc};
use common::AppResult;
use sqlx::PgPool;
use uuid::Uuid;

/// Credit transaction row.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct CreditTransactionRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub amount: i32,
    pub transaction_type: String,
    pub description: Option<String>,
    pub stripe_payment_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Insert a new credit transaction.
pub async fn insert_credit_transaction(
    pool: &PgPool,
    user_id: Uuid,
    amount: i32,
    transaction_type: &str,
    description: Option<&str>,
    stripe_payment_id: Option<&str>,
) -> AppResult<Uuid> {
    let id: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO credit_transactions (id, user_id, amount, transaction_type, description, stripe_payment_id)
        VALUES (uuid_generate_v4(), $1, $2, $3, $4, $5)
        RETURNING id
        "#,
    )
    .bind(user_id)
    .bind(amount)
    .bind(transaction_type)
    .bind(description)
    .bind(stripe_payment_id)
    .fetch_one(pool)
    .await
    .map_err(common::AppError::from)?;

    Ok(id)
}

/// Get paginated credit transactions for a user.
pub async fn get_credit_transactions(
    pool: &PgPool,
    user_id: Uuid,
    limit: i64,
    offset: i64,
) -> AppResult<Vec<CreditTransactionRow>> {
    let rows = sqlx::query_as::<_, CreditTransactionRow>(
        r#"
        SELECT id, user_id, amount, transaction_type, description, stripe_payment_id, created_at
        FROM credit_transactions
        WHERE user_id = $1
        ORDER BY created_at DESC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(user_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
    .map_err(common::AppError::from)?;

    Ok(rows)
}

/// Get the total credit balance for a user (sum of all transactions).
pub async fn get_credit_balance(pool: &PgPool, user_id: Uuid) -> AppResult<i64> {
    let balance: Option<(i64,)> = sqlx::query_as(
        r#"
        SELECT COALESCE(SUM(amount), 0)::bigint
        FROM credit_transactions
        WHERE user_id = $1
        "#,
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(common::AppError::from)?;

    Ok(balance.map(|r| r.0).unwrap_or(0))
}

/// Count total credit transactions for a user.
pub async fn count_transactions(pool: &PgPool, user_id: Uuid) -> AppResult<i64> {
    let count: Option<(i64,)> = sqlx::query_as(
        r#"
        SELECT COUNT(*)::bigint FROM credit_transactions WHERE user_id = $1
        "#,
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(common::AppError::from)?;

    Ok(count.map(|r| r.0).unwrap_or(0))
}
