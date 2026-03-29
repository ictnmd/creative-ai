//! # Billing API
//!
//! HTTP endpoints for subscription management, credit purchases, and Stripe webhooks.

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use billing::stripe_client::{available_tiers, WebhookEvent};
use billing::{StripeClient, Plan};
use chrono::{Duration, Utc};
use common::AppError;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::extractors::AuthUser;
use super::ApiState;

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

/// Application state for billing handlers.
#[derive(Clone)]
pub struct BillingState {
    pub inner: Arc<BillingStateInner>,
}

pub struct BillingStateInner {
    pub api_state: ApiState,
}

impl BillingState {
    pub fn new(api_state: ApiState) -> Self {
        Self {
            inner: Arc::new(BillingStateInner { api_state }),
        }
    }

    fn stripe_client(&self) -> Result<StripeClient, AppError> {
        let key = self
            .inner
            .api_state
            .inner
            .config
            .stripe_secret_key
            .as_deref()
            .ok_or_else(|| AppError::Config("stripe_secret_key not configured".to_string()))?;
        Ok(StripeClient::new(key))
    }

    fn pool(&self) -> &sqlx::PgPool {
        &self.inner.api_state.inner.pool
    }

    fn config(&self) -> &common::AppConfig {
        &self.inner.api_state.inner.config
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

/// Create the billing router.
pub fn create_billing_router(state: ApiState) -> Router {
    let billing_state = BillingState::new(state);
    Router::new()
        .route("/api/v1/subscriptions", get(list_subscriptions))
        .route("/api/v1/subscriptions/subscribe", post(subscribe))
        .route("/api/v1/subscriptions/:id", delete(cancel_subscription))
        .route("/api/v1/credits/balance", get(get_credit_balance))
        .route("/api/v1/credits/purchase", post(purchase_credits))
        .route("/api/v1/credits/history", get(get_credit_history))
        .route("/api/v1/webhooks/stripe", post(stripe_webhook))
        .with_state(billing_state)
}

// ---------------------------------------------------------------------------
// Request/Response Types
// ---------------------------------------------------------------------------

/// Subscription tier details returned to clients.
#[derive(Debug, Serialize)]
pub struct SubscriptionTierResponse {
    pub tier: String,
    pub monthly_price_cents: i64,
    pub yearly_price_cents: i64,
    pub generation_limit: i64,
    pub credits_included: i64,
}

/// User's current billing status.
#[derive(Debug, Serialize)]
pub struct CreditBalanceResponse {
    pub credit_balance: i64,
    pub subscription_tier: String,
    pub quota_remaining: i64,
}

/// Request to subscribe to a tier.
#[derive(Debug, Deserialize)]
pub struct SubscribeRequest {
    pub tier: String,
    pub interval: String,
}

/// Response with checkout URL.
#[derive(Debug, Serialize)]
pub struct CheckoutUrlResponse {
    pub checkout_url: String,
}

/// Request to purchase credits.
#[derive(Debug, Deserialize)]
pub struct PurchaseCreditsRequest {
    pub amount: i64,
}

/// Credit transaction item in history.
#[derive(Debug, Serialize)]
pub struct CreditTransactionResponse {
    pub id: String,
    pub amount: i32,
    pub transaction_type: String,
    pub description: Option<String>,
    pub created_at: String,
}

/// Paginated credit history response.
#[derive(Debug, Serialize)]
pub struct CreditHistoryResponse {
    pub transactions: Vec<CreditTransactionResponse>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

/// Pagination query parameters.
#[derive(Debug, Deserialize)]
pub struct PaginationParams {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// GET /api/v1/subscriptions
///
/// List available subscription tiers with pricing.
async fn list_subscriptions() -> Json<Vec<SubscriptionTierResponse>> {
    let tiers = available_tiers();
    let response: Vec<SubscriptionTierResponse> = tiers
        .into_iter()
        .map(|t| SubscriptionTierResponse {
            tier: t.tier,
            monthly_price_cents: t.monthly_price_cents,
            yearly_price_cents: t.yearly_price_cents,
            generation_limit: t.generation_limit,
            credits_included: t.credits_included,
        })
        .collect();

    Json(response)
}

/// POST /api/v1/subscriptions/subscribe
///
/// Create a Stripe checkout session for a subscription upgrade.
async fn subscribe(
    State(state): State<BillingState>,
    auth_user: AuthUser,
    Json(req): Json<SubscribeRequest>,
) -> Result<Json<CheckoutUrlResponse>, AppError> {
    let user_id: Uuid = auth_user
        .user_id
        .parse()
        .map_err(|_| AppError::Validation("invalid user id".to_string()))?;

    let user = db::queries::users::get_user_by_id(state.pool(), user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("user not found".to_string()))?;

    // Map tier name to plan
    let _plan = match req.tier.to_lowercase().as_str() {
        "starter" | "basic" => Plan::Starter,
        "pro" => Plan::Pro,
        "team" => Plan::Team,
        _ => return Err(AppError::Validation("invalid tier".to_string())),
    };

    let interval = match req.interval.to_lowercase().as_str() {
        "monthly" => billing::stripe_client::SubscriptionInterval::Monthly,
        "yearly" => billing::stripe_client::SubscriptionInterval::Yearly,
        _ => return Err(AppError::Validation("invalid interval".to_string())),
    };

    let prices = _plan.price_ids().ok_or_else(|| {
        AppError::Config(format!("price IDs not configured for plan"))
    })?;

    let price_id = match interval {
        billing::stripe_client::SubscriptionInterval::Monthly => &prices.monthly,
        billing::stripe_client::SubscriptionInterval::Yearly => &prices.yearly,
    };

    let stripe = state.stripe_client()?;

    // Get or create Stripe customer
    let subscription = db::queries::subscriptions::get_user_subscription(state.pool(), user_id)
        .await?;
    let existing_stripe_customer = subscription
        .as_ref()
        .and_then(|s| s.stripe_customer_id.as_deref());

    let customer_id = stripe
        .get_or_create_customer(&user.email, user.name.as_deref(), existing_stripe_customer)
        .await?;

    // Build success/cancel URLs
    let frontend = &state.config().frontend_url;
    let success_url = format!("{}/dashboard?subscription=success", frontend);
    let cancel_url = format!("{}/dashboard?subscription=cancelled", frontend);

    let checkout_url = stripe
        .create_subscription_checkout(&customer_id, price_id, &success_url, &cancel_url)
        .await?;

    Ok(Json(CheckoutUrlResponse { checkout_url }))
}

/// DELETE /api/v1/subscriptions/:id
///
/// Cancel a subscription.
async fn cancel_subscription(
    State(state): State<BillingState>,
    auth_user: AuthUser,
    Path(subscription_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let user_id: Uuid = auth_user
        .user_id
        .parse()
        .map_err(|_| AppError::Validation("invalid user id".to_string()))?;

    // Verify the subscription belongs to the user
    let subscription = db::queries::subscriptions::get_subscription(state.pool(), subscription_id)
        .await?
        .ok_or_else(|| AppError::NotFound("subscription not found".to_string()))?;

    if subscription.user_id != user_id {
        return Err(AppError::Unauthorized("not your subscription".to_string()));
    }

    let stripe_sub_id = subscription
        .stripe_subscription_id
        .as_deref()
        .ok_or_else(|| AppError::Validation("no Stripe subscription ID".to_string()))?;

    let stripe = state.stripe_client()?;
    stripe.cancel_subscription(stripe_sub_id).await?;

    // Update subscription status in DB
    db::queries::subscriptions::cancel_subscription(state.pool(), subscription_id).await?;

    // Downgrade user tier to free
    db::queries::users::update_subscription_tier(state.pool(), user_id, "free").await?;

    Ok((StatusCode::OK, Json(serde_json::json!({ "message": "subscription cancelled" }))))
}

/// GET /api/v1/credits/balance
///
/// Get user's credit balance and quota remaining.
async fn get_credit_balance(
    State(state): State<BillingState>,
    auth_user: AuthUser,
) -> Result<Json<CreditBalanceResponse>, AppError> {
    let user_id: Uuid = auth_user
        .user_id
        .parse()
        .map_err(|_| AppError::Validation("invalid user id".to_string()))?;

    let balance = db::queries::credit_transactions::get_credit_balance(state.pool(), user_id).await?;
    let tier = billing::get_effective_tier(state.pool(), user_id).await?;
    let quota = billing::get_quota_remaining(&tier);

    Ok(Json(CreditBalanceResponse {
        credit_balance: balance,
        subscription_tier: tier,
        quota_remaining: quota,
    }))
}

/// POST /api/v1/credits/purchase
///
/// Create a Stripe checkout session for credit purchase.
async fn purchase_credits(
    State(state): State<BillingState>,
    auth_user: AuthUser,
    Json(req): Json<PurchaseCreditsRequest>,
) -> Result<Json<CheckoutUrlResponse>, AppError> {
    let user_id: Uuid = auth_user
        .user_id
        .parse()
        .map_err(|_| AppError::Validation("invalid user id".to_string()))?;

    if req.amount <= 0 {
        return Err(AppError::Validation("amount must be positive".to_string()));
    }

    // Limit max credits per purchase
    if req.amount > 10000 {
        return Err(AppError::Validation("amount exceeds maximum (10000)".to_string()));
    }

    let user = db::queries::users::get_user_by_id(state.pool(), user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("user not found".to_string()))?;

    let stripe = state.stripe_client()?;

    // Get existing Stripe customer
    let subscription = db::queries::subscriptions::get_user_subscription(state.pool(), user_id)
        .await?;
    let existing_stripe_customer = subscription
        .as_ref()
        .and_then(|s| s.stripe_customer_id.as_deref());

    let customer_id = stripe
        .get_or_create_customer(&user.email, user.name.as_deref(), existing_stripe_customer)
        .await?;

    // Price: $0.01 per credit (1 cent per credit)
    let price_in_cents = req.amount;

    let frontend = &state.config().frontend_url;
    let success_url = format!("{}/dashboard?credits=success", frontend);
    let cancel_url = format!("{}/dashboard?credits=cancelled", frontend);

    let checkout_url = stripe
        .create_credit_checkout(&customer_id, req.amount, price_in_cents, &success_url, &cancel_url)
        .await?;

    Ok(Json(CheckoutUrlResponse { checkout_url }))
}

/// GET /api/v1/credits/history
///
/// Get paginated credit transaction history.
async fn get_credit_history(
    State(state): State<BillingState>,
    auth_user: AuthUser,
    Query(params): Query<PaginationParams>,
) -> Result<Json<CreditHistoryResponse>, AppError> {
    let user_id: Uuid = auth_user
        .user_id
        .parse()
        .map_err(|_| AppError::Validation("invalid user id".to_string()))?;

    let limit = params.limit.unwrap_or(20).min(100);
    let offset = params.offset.unwrap_or(0);

    let transactions = db::queries::credit_transactions::get_credit_transactions(
        state.pool(),
        user_id,
        limit,
        offset,
    )
    .await?;

    let total = db::queries::credit_transactions::count_transactions(state.pool(), user_id).await?;

    let items: Vec<CreditTransactionResponse> = transactions
        .into_iter()
        .map(|t| CreditTransactionResponse {
            id: t.id.to_string(),
            amount: t.amount,
            transaction_type: t.transaction_type,
            description: t.description,
            created_at: t.created_at.to_rfc3339(),
        })
        .collect();

    Ok(Json(CreditHistoryResponse {
        transactions: items,
        total,
        limit,
        offset,
    }))
}

/// POST /api/v1/webhooks/stripe
///
/// Handle Stripe webhook events.
async fn stripe_webhook(
    State(state): State<BillingState>,
    headers: axum::http::HeaderMap,
    body: String,
) -> Result<impl IntoResponse, AppError> {
    let sig_header = headers
        .get("stripe-signature")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| AppError::Validation("missing Stripe signature header".to_string()))?;

    let webhook_secret = state
        .config()
        .stripe_webhook_secret
        .as_deref()
        .ok_or_else(|| AppError::Config("stripe_webhook_secret not configured".to_string()))?;

    let stripe = state.stripe_client()?;

    let event = stripe.construct_webhook_event(body.as_bytes(), sig_header, webhook_secret)?;

    tracing::info!(event_type = %event.type_, "received Stripe webhook");

    match event.type_.as_str() {
        "checkout.session.completed" => {
            handle_checkout_completed(state.pool(), &event).await?;
        }
        "customer.subscription.updated" => {
            handle_subscription_updated(state.pool(), &event).await?;
        }
        "customer.subscription.deleted" => {
            handle_subscription_deleted(state.pool(), &event).await?;
        }
        _ => {
            tracing::debug!(event_type = %event.type_, "unhandled Stripe webhook event");
        }
    }

    Ok(StatusCode::OK)
}

// ---------------------------------------------------------------------------
// Webhook Handlers
// ---------------------------------------------------------------------------

/// Parse a checkout session from webhook data.
fn parse_checkout_session(data: &serde_json::Value) -> Option<(Option<String>, Option<String>, Option<String>, Option<i64>)> {
    let customer_id = data.get("customer").and_then(|v| v.as_str()).map(String::from);
    let subscription_id = data.get("subscription").and_then(|v| v.as_str()).map(String::from);
    let payment_intent = data.get("payment_intent").and_then(|v| v.as_str()).map(String::from);
    let amount_total = data.get("amount_total").and_then(|v| v.as_i64());
    Some((customer_id, subscription_id, payment_intent, amount_total))
}

/// Parse a subscription from webhook data.
fn parse_subscription(data: &serde_json::Value) -> Option<(String, String)> {
    let id = data.get("id").and_then(|v| v.as_str())?.to_string();
    let status = data.get("status").and_then(|v| v.as_str())?.to_string();
    Some((id, status))
}

async fn handle_checkout_completed(pool: &sqlx::PgPool, event: &WebhookEvent) -> Result<(), AppError> {
    let (customer_id, subscription_id, payment_intent, amount_total) =
        parse_checkout_session(&event.data)
        .ok_or_else(|| AppError::Internal("failed to parse checkout session from webhook".to_string()))?;

    if let Some(sub_id) = subscription_id {
        // Subscription checkout completed
        if let Some(row) = db::queries::subscriptions::get_by_stripe_id(pool, &sub_id).await? {
            tracing::debug!(subscription_id = %sub_id, "subscription already recorded");
        } else {
            let tier = determine_tier_from_subscription(pool, &sub_id).await?;
            let user_id = find_user_by_stripe_customer(pool, customer_id.as_deref().unwrap_or_default()).await?;

            if let Some(uid) = user_id {
                let expires = Utc::now() + Duration::days(30);
                let quota = match tier.as_str() {
                    "basic" => 200,
                    "pro" => 1000,
                    "unlimited" => 5000,
                    _ => 200,
                };

                db::queries::subscriptions::create_subscription(
                    pool,
                    uid,
                    &tier,
                    Some(&sub_id),
                    customer_id.as_deref(),
                    quota,
                    expires,
                )
                .await?;

                db::queries::users::update_subscription_tier(pool, uid, &tier).await?;

                tracing::info!(
                    user_id = %uid,
                    tier = %tier,
                    subscription_id = %sub_id,
                    "subscription activated from webhook"
                );
            }
        }
    } else {
        // Credit purchase checkout completed
        if let Some(credits) = amount_total {
            let user_id = find_user_by_stripe_customer(pool, customer_id.as_deref().unwrap_or_default()).await?;

            if let Some(uid) = user_id {
                let description = format!("Credit purchase - {} credits", credits);
                let payment_id = payment_intent.unwrap_or_default();

                db::queries::credit_transactions::insert_credit_transaction(
                    pool,
                    uid,
                    credits as i32,
                    "purchase",
                    Some(&description),
                    Some(&payment_id),
                )
                .await?;

                tracing::info!(
                    user_id = %uid,
                    credits = credits,
                    payment_id = %payment_id,
                    "credits purchased via webhook"
                );
            }
        }
    }

    Ok(())
}

async fn handle_subscription_updated(pool: &sqlx::PgPool, event: &WebhookEvent) -> Result<(), AppError> {
    let (sub_id, status) = parse_subscription(&event.data)
        .ok_or_else(|| AppError::Internal("failed to parse subscription from webhook".to_string()))?;

    if let Some(row) = db::queries::subscriptions::get_by_stripe_id(pool, &sub_id).await? {
        let db_status = match status.as_str() {
            "active" => "active",
            "past_due" => "past_due",
            "canceled" => "cancelled",
            _ => "active",
        };

        db::queries::subscriptions::update_status(pool, row.id, db_status).await?;

        tracing::info!(
            subscription_id = %sub_id,
            status = %status,
            "subscription status updated from webhook"
        );
    }

    Ok(())
}

async fn handle_subscription_deleted(pool: &sqlx::PgPool, event: &WebhookEvent) -> Result<(), AppError> {
    let (sub_id, _) = parse_subscription(&event.data)
        .ok_or_else(|| AppError::Internal("failed to parse subscription from webhook".to_string()))?;

    if let Some(row) = db::queries::subscriptions::get_by_stripe_id(pool, &sub_id).await? {
        db::queries::subscriptions::cancel_subscription(pool, row.id).await?;
        db::queries::users::update_subscription_tier(pool, row.user_id, "free").await?;

        tracing::info!(
            user_id = %row.user_id,
            subscription_id = %sub_id,
            "subscription cancelled from webhook"
        );
    }

    Ok(())
}

/// Determine the tier name from a Stripe subscription's price by fetching it.
async fn determine_tier_from_subscription(pool: &sqlx::PgPool, subscription_id: &str) -> Result<String, AppError> {
    // Get the subscription from Stripe to find the price ID
    let stripe_key = std::env::var("STRIPE_SECRET_KEY")
        .map_err(|_| AppError::Config("STRIPE_SECRET_KEY not configured".to_string()))?;

    let client = reqwest::Client::new();
    let resp = client
        .get(format!("https://api.stripe.com/v1/subscriptions/{}", subscription_id))
        .header("Authorization", format!("Bearer {}", stripe_key))
        .send()
        .await
        .map_err(|e| AppError::Http(e))?;

    if !resp.status().is_success() {
        return Err(AppError::Billing("failed to fetch subscription from Stripe".to_string()));
    }

    #[derive(serde::Deserialize)]
    struct SubResponse {
        items: Option<SubItems>,
    }

    #[derive(serde::Deserialize)]
    struct SubItems {
        data: Vec<SubItemData>,
    }

    #[derive(serde::Deserialize)]
    struct SubItemData {
        price: Option<SubPrice>,
    }

    #[derive(serde::Deserialize)]
    struct SubPrice {
        id: String,
    }

    let sub: SubResponse = resp.json().await
        .map_err(|e| AppError::Internal(format!("failed to parse subscription: {}", e)))?;

    if let Some(items) = sub.items {
        for item in items.data {
            if let Some(price) = item.price {
                let price_id = &price.id;

                if std::env::var("STRIPE_STARTER_MONTHLY_PRICE_ID").ok().as_ref() == Some(&price_id)
                    || std::env::var("STRIPE_STARTER_YEARLY_PRICE_ID").ok().as_ref() == Some(&price_id)
                {
                    return Ok("basic".to_string());
                }
                if std::env::var("STRIPE_PRO_MONTHLY_PRICE_ID").ok().as_ref() == Some(&price_id)
                    || std::env::var("STRIPE_PRO_YEARLY_PRICE_ID").ok().as_ref() == Some(&price_id)
                {
                    return Ok("pro".to_string());
                }
                if std::env::var("STRIPE_TEAM_MONTHLY_PRICE_ID").ok().as_ref() == Some(&price_id)
                    || std::env::var("STRIPE_TEAM_YEARLY_PRICE_ID").ok().as_ref() == Some(&price_id)
                {
                    return Ok("unlimited".to_string());
                }
            }
        }
    }

    Err(AppError::Billing("could not determine tier from subscription".to_string()))
}

/// Find a user by their Stripe customer ID.
async fn find_user_by_stripe_customer(
    pool: &sqlx::PgPool,
    customer_id: &str,
) -> Result<Option<Uuid>, AppError> {
    let user_id: Option<(Uuid,)> = sqlx::query_as(
        r#"
        SELECT user_id FROM subscriptions WHERE stripe_customer_id = $1 LIMIT 1
        "#,
    )
    .bind(customer_id)
    .fetch_optional(pool)
    .await
    .map_err(AppError::from)?;

    Ok(user_id.map(|r| r.0))
}
