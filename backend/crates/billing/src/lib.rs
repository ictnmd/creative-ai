//! # Billing Crate
//!
//! Stripe-based billing and subscription management.
//! Handles plan upgrades, downgrades, payment processing, and webhooks.

use serde::{Deserialize, Serialize};

/// Plan pricing IDs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanPrices {
    pub monthly: String,
    pub yearly: String,
}

/// Available subscription plans.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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
