//! # Models Library
//!
//! Shared domain models for the Creative AI Studio backend.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// User model representing an authenticated user.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub username: String,
    pub hashed_password: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub is_active: bool,
    pub is_verified: bool,
    pub role: UserRole,
}

/// User role enum.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum UserRole {
    Free,
    Pro,
    Team,
    Admin,
}

/// Project model representing a creative project.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub status: ProjectStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub settings: ProjectSettings,
}

/// Project status enum.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ProjectStatus {
    Draft,
    Processing,
    Complete,
    Failed,
    Archived,
}

/// Project settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSettings {
    pub width: u32,
    pub height: u32,
    pub style: Option<String>,
    pub model: Option<String>,
    pub seed: Option<i64>,
}

impl Default for ProjectSettings {
    fn default() -> Self {
        Self {
            width: 1024,
            height: 1024,
            style: None,
            model: None,
            seed: None,
        }
    }
}

/// Generation request model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationRequest {
    pub id: Uuid,
    pub project_id: Uuid,
    pub user_id: Uuid,
    pub prompt: String,
    pub negative_prompt: Option<String>,
    pub status: GenerationStatus,
    pub model: String,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

/// Generation status enum.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum GenerationStatus {
    Queued,
    Processing,
    Completed,
    Failed,
    Cancelled,
}

/// Generation result with output URLs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationResult {
    pub id: Uuid,
    pub request_id: Uuid,
    pub output_url: String,
    pub thumbnail_url: Option<String>,
    pub width: u32,
    pub height: u32,
    pub seed: i64,
    pub metadata: serde_json::Value,
}

/// Subscription plan types.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PlanType {
    Free,
    Starter,
    Pro,
    Team,
    Enterprise,
}

/// User subscription.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subscription {
    pub id: Uuid,
    pub user_id: Uuid,
    pub plan: PlanType,
    pub stripe_subscription_id: Option<String>,
    pub stripe_customer_id: Option<String>,
    pub status: SubscriptionStatus,
    pub current_period_start: DateTime<Utc>,
    pub current_period_end: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

/// Subscription status enum.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SubscriptionStatus {
    Active,
    PastDue,
    Cancelled,
    Trialing,
}

/// Usage record for tracking generation counts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageRecord {
    pub id: Uuid,
    pub user_id: Uuid,
    pub generation_count: i64,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
}

/// Share link model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareLink {
    pub id: Uuid,
    pub project_id: Uuid,
    pub token: String,
    pub created_by: Uuid,
    pub expires_at: Option<DateTime<Utc>>,
    pub view_count: i64,
    pub allow_download: bool,
    pub created_at: DateTime<Utc>,
}

/// API response wrapper.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
    pub message: Option<String>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            message: None,
        }
    }

    pub fn error_msg(message: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message.into()),
            message: None,
        }
    }
}

/// Paginated response wrapper.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResponse<T> {
    pub items: Vec<T>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
    pub total_pages: i64,
}

/// JWT claims for authentication.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String, // user id
    pub email: String,
    pub role: String,
    pub exp: i64,
    pub iat: i64,
}
