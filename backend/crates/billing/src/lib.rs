//! # Billing Crate
//!
//! Stripe-based billing and subscription management.
//! Handles plan upgrades, downgrades, payment processing, and webhooks.

use common::AppError;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

// Re-export stripe client
pub mod stripe_client;

pub use stripe_client::{StripeClient, TierPricing, available_tiers};

/// Plan pricing IDs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanPrices {
    pub monthly: String,
    pub yearly: String,
}

/// Available subscription plans.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Plan {
    Free,
    Starter,
    Pro,
    Team,
}

impl Plan {
    /// Get the Stripe price IDs for a plan from environment variables.
    pub fn price_ids(&self) -> Option<PlanPrices> {
        match self {
            Plan::Free => None,
            Plan::Starter => Some(PlanPrices {
                monthly: std::env::var("STRIPE_STARTER_MONTHLY_PRICE_ID").ok()?,
                yearly: std::env::var("STRIPE_STARTER_YEARLY_PRICE_ID").ok()?,
            }),
            Plan::Pro => Some(PlanPrices {
                monthly: std::env::var("STRIPE_PRO_MONTHLY_PRICE_ID").ok()?,
                yearly: std::env::var("STRIPE_PRO_YEARLY_PRICE_ID").ok()?,
            }),
            Plan::Team => Some(PlanPrices {
                monthly: std::env::var("STRIPE_TEAM_MONTHLY_PRICE_ID").ok()?,
                yearly: std::env::var("STRIPE_TEAM_YEARLY_PRICE_ID").ok()?,
            }),
        }
    }

    /// Get the generation limit for a plan.
    pub fn generation_limit(&self) -> i64 {
        match self {
            Plan::Free => 10,
            Plan::Starter => 200,
            Plan::Pro => 1000,
            Plan::Team => 5000,
        }
    }

    /// Convert a string tier name to a Plan enum.
    pub fn from_str(s: &str) -> Option<Plan> {
        match s.to_lowercase().as_str() {
            "free" => Some(Plan::Free),
            "starter" => Some(Plan::Starter),
            "basic" => Some(Plan::Starter),
            "pro" => Some(Plan::Pro),
            "team" => Some(Plan::Team),
            _ => None,
        }
    }

    /// Convert Plan enum to a tier string for DB storage.
    pub fn as_tier_str(&self) -> &'static str {
        match self {
            Plan::Free => "free",
            Plan::Starter => "basic",
            Plan::Pro => "pro",
            Plan::Team => "unlimited",
        }
    }
}

/// Billing service for Stripe operations.
/// Initialize with a Stripe API key when needed.
#[derive(Clone)]
pub struct BillingService {
    api_key: String,
}

impl BillingService {
    /// Create a new billing service from a Stripe API key.
    pub fn new(api_key: String) -> Self {
        Self { api_key }
    }

    /// Get the configured API key.
    pub fn api_key(&self) -> &str {
        &self.api_key
    }
}

// ---------------------------------------------------------------------------
// Billing Query Functions
// ---------------------------------------------------------------------------

/// Get effective subscription tier for a user from the users table.
pub async fn get_effective_tier(pool: &PgPool, user_id: Uuid) -> Result<String, AppError> {
    let tier: Option<(String,)> = sqlx::query_as("SELECT subscription_tier FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .map_err(AppError::from)?;

    Ok(tier.map(|r| r.0).unwrap_or_else(|| "free".to_string()))
}

/// Get remaining quota for a subscription tier.
///
/// Returns the number of generations remaining in the current billing period.
/// Returns -1 for unlimited tiers.
pub fn get_quota_remaining(tier: &str) -> i64 {
    let plan = Plan::from_str(tier).unwrap_or(Plan::Free);
    plan.generation_limit()
}

/// Get credit balance for a user by summing credit_transactions.
pub async fn get_credit_balance(pool: &PgPool, user_id: Uuid) -> Result<i64, AppError> {
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
    .map_err(AppError::from)?;

    Ok(balance.map(|r| r.0).unwrap_or(0))
}

/// Deduct credits from a user by inserting a negative credit_transactions row.
pub async fn deduct_credits(
    pool: &PgPool,
    user_id: Uuid,
    amount: i64,
    description: &str,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT INTO credit_transactions (user_id, amount, transaction_type, description)
        VALUES ($1, $2, 'usage', $3)
        "#,
    )
    .bind(user_id)
    .bind(-amount.abs())
    .bind(description)
    .execute(pool)
    .await
    .map_err(AppError::from)?;

    Ok(())
}

/// Get the provider cost in credits per generation.
///
/// | Provider  | Model       | Credits |
/// |-----------|-------------|---------|
/// | dall-e-3  | dall-e-3    | 4       |
/// | dall-e-2  | dall-e-2    | 1       |
/// | imagen-3  | imagen-3    | 3       |
/// | claude    | claude-...  | 0       |
pub fn get_provider_cost(provider: &str, model: &str) -> u32 {
    match provider.to_lowercase().as_str() {
        "dall-e-3" | "dalle3" | "dalle" => 4,
        "dall-e-2" | "dalle2" => 1,
        "imagen-3" | "imagen3" | "imagen" => 3,
        "claude" => 0,
        _ => {
            // Fallback: use model name matching
            let m = model.to_lowercase();
            if m.contains("dall-e-3") || m.contains("dalle3") {
                4
            } else if m.contains("dall-e-2") || m.contains("dalle2") {
                1
            } else if m.contains("imagen") {
                3
            } else if m.contains("claude") {
                0
            } else {
                4 // default to highest cost
            }
        }
    }
}

/// Determine if a user has enough quota/credits for a generation.
///
/// Returns true if:
/// - The user's quota remaining (based on subscription tier) > 0, OR
/// - The user's credit balance >= the generation cost
pub async fn can_generate(pool: &PgPool, user_id: Uuid, cost: u32) -> Result<bool, AppError> {
    // Check quota based on tier
    let tier = get_effective_tier(pool, user_id).await?;
    let quota = get_quota_remaining(&tier);

    // Unlimited tiers (unlimited returns -1)
    if quota == -1 || quota > 0 {
        return Ok(true);
    }

    // Check credit balance
    let balance = get_credit_balance(pool, user_id).await?;
    Ok(balance >= cost as i64)
}

/// Record a credit purchase transaction.
pub async fn record_credit_purchase(
    pool: &PgPool,
    user_id: Uuid,
    amount: i64,
    stripe_payment_id: &str,
    description: &str,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT INTO credit_transactions (user_id, amount, transaction_type, description, stripe_payment_id)
        VALUES ($1, $2, 'purchase', $3, $4)
        "#,
    )
    .bind(user_id)
    .bind(amount)
    .bind(description)
    .bind(stripe_payment_id)
    .execute(pool)
    .await
    .map_err(AppError::from)?;

    Ok(())
}

/// Record a credit refund transaction.
pub async fn record_credit_refund(
    pool: &PgPool,
    user_id: Uuid,
    amount: i64,
    description: &str,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT INTO credit_transactions (user_id, amount, transaction_type, description)
        VALUES ($1, $2, 'refund', $3)
        "#,
    )
    .bind(user_id)
    .bind(amount.abs())
    .bind(description)
    .execute(pool)
    .await
    .map_err(AppError::from)?;

    Ok(())
}

/// Record a bonus credit grant.
pub async fn record_credit_bonus(
    pool: &PgPool,
    user_id: Uuid,
    amount: i64,
    description: &str,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT INTO credit_transactions (user_id, amount, transaction_type, description)
        VALUES ($1, $2, 'bonus', $3)
        "#,
    )
    .bind(user_id)
    .bind(amount)
    .bind(description)
    .execute(pool)
    .await
    .map_err(AppError::from)?;

    Ok(())
}
