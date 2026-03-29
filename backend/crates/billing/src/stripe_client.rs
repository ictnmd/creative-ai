//! # Stripe Client
//!
//! HTTP-based Stripe API client using reqwest.
//! Provides subscription and payment operations without external stripe-rust dependency.

use common::AppError;
use serde::{Deserialize, Serialize};

/// Subscription interval (monthly or yearly billing).
#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SubscriptionInterval {
    Monthly,
    Yearly,
}

/// Subscription tier pricing info.
#[derive(Debug, Clone, Serialize)]
pub struct TierPricing {
    pub tier: String,
    pub monthly_price_cents: i64,
    pub yearly_price_cents: i64,
    pub monthly_price_id: Option<String>,
    pub yearly_price_id: Option<String>,
    pub generation_limit: i64,
    pub credits_included: i64,
}

impl TierPricing {
    /// Returns the Stripe price ID for the given interval.
    pub fn price_id(&self, interval: SubscriptionInterval) -> Option<&str> {
        match interval {
            SubscriptionInterval::Monthly => self.monthly_price_id.as_deref(),
            SubscriptionInterval::Yearly => self.yearly_price_id.as_deref(),
        }
    }
}

/// Available subscription tiers with their pricing.
pub fn available_tiers() -> Vec<TierPricing> {
    vec![
        TierPricing {
            tier: "starter".to_string(),
            monthly_price_cents: 9_99,
            yearly_price_cents: 99_99,
            monthly_price_id: std::env::var("STRIPE_STARTER_MONTHLY_PRICE_ID").ok(),
            yearly_price_id: std::env::var("STRIPE_STARTER_YEARLY_PRICE_ID").ok(),
            generation_limit: 200,
            credits_included: 0,
        },
        TierPricing {
            tier: "pro".to_string(),
            monthly_price_cents: 29_99,
            yearly_price_cents: 299_99,
            monthly_price_id: std::env::var("STRIPE_PRO_MONTHLY_PRICE_ID").ok(),
            yearly_price_id: std::env::var("STRIPE_PRO_YEARLY_PRICE_ID").ok(),
            generation_limit: 1000,
            credits_included: 0,
        },
        TierPricing {
            tier: "team".to_string(),
            monthly_price_cents: 99_99,
            yearly_price_cents: 999_99,
            monthly_price_id: std::env::var("STRIPE_TEAM_MONTHLY_PRICE_ID").ok(),
            yearly_price_id: std::env::var("STRIPE_TEAM_YEARLY_PRICE_ID").ok(),
            generation_limit: 5000,
            credits_included: 0,
        },
    ]
}

// ---------------------------------------------------------------------------
// Stripe API types (raw JSON responses)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub(crate) struct StripeCustomer {
    pub id: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct StripeCheckoutSession {
    pub id: String,
    pub url: Option<String>,
    #[serde(rename = "type")]
    pub type_: Option<String>,
    pub customer: Option<String>,
    pub subscription: Option<String>,
    pub payment_intent: Option<String>,
    pub amount_total: Option<i64>,
    pub status: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct StripeSubscription {
    pub id: String,
    pub status: String,
    pub items: Option<StripeSubscriptionItems>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct StripeSubscriptionItems {
    pub data: Vec<StripeSubscriptionItem>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct StripeSubscriptionItem {
    pub price: Option<StripePrice>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct StripePrice {
    pub id: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct StripeEvent {
    #[serde(rename = "type")]
    pub type_: String,
    pub data: StripeEventData,
}

#[derive(Debug, Deserialize)]
pub(crate) struct StripeEventData {
    pub object: serde_json::Value,
}

/// Subscription info extracted from Stripe API response.
#[derive(Debug, Clone)]
pub struct SubscriptionInfo {
    pub id: String,
    pub status: String,
}

/// Webhook event parsed from Stripe payload.
#[derive(Debug, Clone)]
pub struct WebhookEvent {
    pub type_: String,
    pub data: serde_json::Value,
}

// ---------------------------------------------------------------------------
// Stripe Client
// ---------------------------------------------------------------------------

/// Stripe client wrapper for billing operations.
/// Uses raw Stripe REST API via HTTPS requests.
#[derive(Clone)]
pub struct StripeClient {
    api_key: String,
    client: reqwest::Client,
}

impl StripeClient {
    /// Create a new Stripe client from an API key.
    pub fn new(api_key: &str) -> Self {
        Self {
            api_key: api_key.to_string(),
            client: reqwest::Client::new(),
        }
    }

    fn base_url(&self) -> &str {
        "https://api.stripe.com/v1"
    }

    fn auth_header(&self) -> String {
        format!("Bearer {}", self.api_key)
    }

    fn check_error(resp: &reqwest::Response) -> Result<(), AppError> {
        if !resp.status().is_success() {
            return Err(AppError::Billing(format!(
                "Stripe API error: status {}",
                resp.status()
            )));
        }
        Ok(())
    }

    /// Get or create a Stripe customer for a user.
    pub async fn get_or_create_customer(
        &self,
        email: &str,
        name: Option<&str>,
        existing_id: Option<&str>,
    ) -> Result<String, AppError> {
        if let Some(id) = existing_id {
            return Ok(id.to_string());
        }

        let mut form = vec![("email", email.to_string())];
        if let Some(n) = name {
            form.push(("name", n.to_string()));
        }

        let resp = self
            .client
            .post(format!("{}/customers", self.base_url()))
            .header("Authorization", &self.auth_header())
            .header("Content-Type", "application/x-www-form-urlencoded")
            .form(&form)
            .send()
            .await
            .map_err(AppError::Http)?;

        Self::check_error(&resp)?;

        let customer: StripeCustomer = resp
            .json()
            .await
            .map_err(|e| AppError::Internal(format!("failed to parse Stripe customer: {}", e)))?;

        Ok(customer.id)
    }

    /// Create a Stripe checkout session for a subscription upgrade.
    ///
    /// Returns the checkout URL to redirect the user to.
    pub async fn create_subscription_checkout(
        &self,
        customer_id: &str,
        price_id: &str,
        success_url: &str,
        cancel_url: &str,
    ) -> Result<String, AppError> {
        let form = [
            ("customer", customer_id.to_string()),
            ("mode", "subscription".to_string()),
            ("success_url", success_url.to_string()),
            ("cancel_url", cancel_url.to_string()),
            ("line_items[0][price]", price_id.to_string()),
            ("line_items[0][quantity]", "1".to_string()),
        ];

        let resp = self
            .client
            .post(format!("{}/checkout/sessions", self.base_url()))
            .header("Authorization", &self.auth_header())
            .header("Content-Type", "application/x-www-form-urlencoded")
            .form(&form)
            .send()
            .await
            .map_err(AppError::Http)?;

        Self::check_error(&resp)?;

        let session: StripeCheckoutSession = resp
            .json()
            .await
            .map_err(|e| AppError::Internal(format!("failed to parse checkout session: {}", e)))?;

        session.url.ok_or_else(|| AppError::Billing("checkout session has no URL".to_string()))
    }

    /// Create a Stripe checkout session for purchasing credits (one-time payment).
    ///
    /// Returns the checkout URL.
    pub async fn create_credit_checkout(
        &self,
        customer_id: &str,
        credits_amount: i64,
        _price_in_cents: i64,
        success_url: &str,
        cancel_url: &str,
    ) -> Result<String, AppError> {
        // For credit purchases, create an inline price and use it in checkout.
        // First create a price for the credit pack.
        let price_id = self.create_credit_price(credits_amount).await?;

        let form = [
            ("customer", customer_id.to_string()),
            ("mode", "payment".to_string()),
            ("success_url", success_url.to_string()),
            ("cancel_url", cancel_url.to_string()),
            ("line_items[0][price]", price_id),
            ("line_items[0][quantity]", "1".to_string()),
        ];

        let resp = self
            .client
            .post(format!("{}/checkout/sessions", self.base_url()))
            .header("Authorization", &self.auth_header())
            .header("Content-Type", "application/x-www-form-urlencoded")
            .form(&form)
            .send()
            .await
            .map_err(AppError::Http)?;

        Self::check_error(&resp)?;

        let session: StripeCheckoutSession = resp
            .json()
            .await
            .map_err(|e| AppError::Internal(format!("failed to parse credit checkout session: {}", e)))?;

        session.url.ok_or_else(|| AppError::Billing("checkout session has no URL".to_string()))
    }

    /// Create a Stripe price for credit purchase.
    async fn create_credit_price(&self, credits: i64) -> Result<String, AppError> {
        let form = [
            ("currency", "usd".to_string()),
            ("unit_amount", (credits).to_string()),
            ("product_data[name]", format!("{} Credits", credits)),
        ];

        #[derive(Deserialize)]
        struct PriceResponse {
            id: String,
        }

        let resp = self
            .client
            .post(format!("{}/prices", self.base_url()))
            .header("Authorization", &self.auth_header())
            .header("Content-Type", "application/x-www-form-urlencoded")
            .form(&form)
            .send()
            .await
            .map_err(AppError::Http)?;

        Self::check_error(&resp)?;

        let price: PriceResponse = resp
            .json()
            .await
            .map_err(|e| AppError::Internal(format!("failed to parse price: {}", e)))?;

        Ok(price.id)
    }

    /// Cancel a subscription by marking it to cancel at period end.
    pub async fn cancel_subscription(&self, subscription_id: &str) -> Result<(), AppError> {
        let resp = self
            .client
            .post(format!("{}/subscriptions/{}", self.base_url(), subscription_id))
            .header("Authorization", &self.auth_header())
            .header("Content-Type", "application/x-www-form-urlencoded")
            .form(&[("cancel_at_period_end", "true")])
            .send()
            .await
            .map_err(AppError::Http)?;

        Self::check_error(&resp)?;
        Ok(())
    }

    /// Get subscription details by Stripe subscription ID.
    pub async fn get_subscription(&self, subscription_id: &str) -> Result<SubscriptionInfo, AppError> {
        let resp = self
            .client
            .get(format!("{}/subscriptions/{}", self.base_url(), subscription_id))
            .header("Authorization", &self.auth_header())
            .send()
            .await
            .map_err(AppError::Http)?;

        Self::check_error(&resp)?;

        let sub: StripeSubscription = resp
            .json()
            .await
            .map_err(|e| AppError::Internal(format!("failed to parse subscription: {}", e)))?;

        Ok(SubscriptionInfo {
            id: sub.id,
            status: sub.status,
        })
    }

    /// Construct and verify a webhook event from raw payload and Stripe signature header.
    ///
    /// Uses HMAC-SHA256 to verify the Stripe webhook signature.
    pub fn construct_webhook_event(
        &self,
        payload: &[u8],
        sig_header: &str,
        webhook_secret: &str,
    ) -> Result<WebhookEvent, AppError> {
        // Parse signature header: "t=timestamp,v1=signature"
        let mut timestamp: Option<i64> = None;
        let mut signature: Option<String> = None;

        for part in sig_header.split(',') {
            let part = part.trim();
            if let Some(v) = part.strip_prefix("t=") {
                timestamp = v.parse().ok();
            } else if let Some(v) = part.strip_prefix("v1=") {
                signature = Some(v.to_string());
            }
        }

        let timestamp = timestamp.ok_or_else(|| AppError::Billing("missing timestamp in webhook signature".to_string()))?;
        let sig = signature.ok_or_else(|| AppError::Billing("missing v1 signature".to_string()))?;

        // Compute expected signature: HMAC-SHA256(timestamp.payload, webhook_secret)
        let signed_payload = format!("{}.{}", timestamp, String::from_utf8_lossy(payload));
        use hmac::{Hmac, Mac};
        use sha2::Sha256;
        type HmacSha256 = Hmac<Sha256>;

        let mut mac = HmacSha256::new_from_slice(webhook_secret.as_bytes())
            .map_err(|e| AppError::Internal(format!("HMAC error: {}", e)))?;
        mac.update(signed_payload.as_bytes());

        let expected_sig = hex::encode(mac.finalize().into_bytes());

        if !constant_time_compare(&expected_sig, &sig) {
            return Err(AppError::Billing("webhook signature verification failed".to_string()));
        }

        let event: StripeEvent = serde_json::from_slice(payload)
            .map_err(|e| AppError::Json(e))?;

        Ok(WebhookEvent {
            type_: event.type_,
            data: event.data.object,
        })
    }
}

/// Constant-time string comparison to prevent timing attacks.
fn constant_time_compare(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.bytes().zip(b.bytes()) {
        diff |= x ^ y;
    }
    diff == 0
}
