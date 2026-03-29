//! Subscription query operations

use chrono::{DateTime, Utc};
use common::AppResult;
use sqlx::PgPool;
use uuid::Uuid;

/// Subscription row returned from the subscriptions table.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct SubscriptionRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub tier: String,
    pub stripe_subscription_id: Option<String>,
    pub stripe_customer_id: Option<String>,
    pub quota_monthly: i32,
    pub starts_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

/// Create a new subscription record.
pub async fn create_subscription(
    pool: &PgPool,
    user_id: Uuid,
    tier: &str,
    stripe_subscription_id: Option<&str>,
    stripe_customer_id: Option<&str>,
    quota_monthly: i32,
    expires_at: DateTime<Utc>,
) -> AppResult<Uuid> {
    let id: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO subscriptions (id, user_id, tier, stripe_subscription_id, stripe_customer_id, quota_monthly, expires_at)
        VALUES (uuid_generate_v4(), $1, $2, $3, $4, $5, $6)
        RETURNING id
        "#,
    )
    .bind(user_id)
    .bind(tier)
    .bind(stripe_subscription_id)
    .bind(stripe_customer_id)
    .bind(quota_monthly)
    .bind(expires_at)
    .fetch_one(pool)
    .await
    .map_err(common::AppError::from)?;

    Ok(id)
}

/// Cancel a subscription by setting its status to 'cancelled'.
pub async fn cancel_subscription(pool: &PgPool, subscription_id: Uuid) -> AppResult<()> {
    sqlx::query(
        r#"
        UPDATE subscriptions SET status = 'cancelled' WHERE id = $1
        "#,
    )
    .bind(subscription_id)
    .execute(pool)
    .await
    .map_err(common::AppError::from)?;

    Ok(())
}

/// Cancel a subscription by Stripe subscription ID.
pub async fn cancel_by_stripe_id(pool: &PgPool, stripe_subscription_id: &str) -> AppResult<()> {
    sqlx::query(
        r#"
        UPDATE subscriptions SET status = 'cancelled' WHERE stripe_subscription_id = $1
        "#,
    )
    .bind(stripe_subscription_id)
    .execute(pool)
    .await
    .map_err(common::AppError::from)?;

    Ok(())
}

/// Get a subscription by its UUID.
pub async fn get_subscription(pool: &PgPool, subscription_id: Uuid) -> AppResult<Option<SubscriptionRow>> {
    sqlx::query_as::<_, SubscriptionRow>(
        r#"
        SELECT id, user_id, tier, stripe_subscription_id, stripe_customer_id,
               quota_monthly, starts_at, expires_at, status, created_at
        FROM subscriptions
        WHERE id = $1
        "#,
    )
    .bind(subscription_id)
    .fetch_optional(pool)
    .await
    .map_err(common::AppError::from)
}

/// Get a subscription by Stripe subscription ID.
pub async fn get_by_stripe_id(
    pool: &PgPool,
    stripe_subscription_id: &str,
) -> AppResult<Option<SubscriptionRow>> {
    sqlx::query_as::<_, SubscriptionRow>(
        r#"
        SELECT id, user_id, tier, stripe_subscription_id, stripe_customer_id,
               quota_monthly, starts_at, expires_at, status, created_at
        FROM subscriptions
        WHERE stripe_subscription_id = $1
        "#,
    )
    .bind(stripe_subscription_id)
    .fetch_optional(pool)
    .await
    .map_err(common::AppError::from)
}

/// Get the active subscription for a user.
pub async fn get_user_subscription(pool: &PgPool, user_id: Uuid) -> AppResult<Option<SubscriptionRow>> {
    sqlx::query_as::<_, SubscriptionRow>(
        r#"
        SELECT id, user_id, tier, stripe_subscription_id, stripe_customer_id,
               quota_monthly, starts_at, expires_at, status, created_at
        FROM subscriptions
        WHERE user_id = $1 AND status = 'active' AND expires_at > NOW()
        ORDER BY created_at DESC
        LIMIT 1
        "#,
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(common::AppError::from)
}

/// Update subscription status.
pub async fn update_status(
    pool: &PgPool,
    subscription_id: Uuid,
    status: &str,
) -> AppResult<()> {
    sqlx::query(
        r#"
        UPDATE subscriptions SET status = $2 WHERE id = $1
        "#,
    )
    .bind(subscription_id)
    .bind(status)
    .execute(pool)
    .await
    .map_err(common::AppError::from)?;

    Ok(())
}

/// Update subscription tier for a user.
pub async fn update_tier(
    pool: &PgPool,
    user_id: Uuid,
    tier: &str,
) -> AppResult<()> {
    sqlx::query(
        r#"
        UPDATE subscriptions SET tier = $2 WHERE user_id = $1 AND status = 'active'
        "#,
    )
    .bind(user_id)
    .bind(tier)
    .execute(pool)
    .await
    .map_err(common::AppError::from)?;

    Ok(())
}
