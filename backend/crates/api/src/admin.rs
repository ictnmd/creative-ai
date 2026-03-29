//! # Admin API Routes
//!
//! Admin-only endpoints for user management, system API key management,
//! analytics, and content moderation.

use crate::extractors::AuthUser;
use crate::ApiState;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post, put},
    Json, Router,
};
use chrono::{Duration as ChronoDuration, NaiveDate, Utc};
use common::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Admin Middleware Extractor
// ---------------------------------------------------------------------------

/// Admin user extracted from a verified JWT token with admin role check.
///
/// This extractor wraps `AuthUser` and additionally verifies that the
/// authenticated user has the "admin" role. Requests by non-admin users
/// will receive a 403 Forbidden response.
#[derive(Debug, Clone)]
pub struct AdminUser {
    pub user_id: String,
    pub username: String,
    pub role: String,
}

impl AdminUser {
    pub fn from_auth_user(auth: AuthUser) -> Result<Self, AppError> {
        if auth.role != "admin" {
            return Err(AppError::Unauthorized(
                "admin access required".to_string(),
            ));
        }
        Ok(Self {
            user_id: auth.user_id,
            username: auth.username,
            role: auth.role,
        })
    }
}

impl<S: Send + Sync> axum::extract::FromRequestParts<S> for AdminUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let auth_user = AuthUser::from_request_parts(parts, state)
            .await
            .map_err(|e| AppError::Unauthorized(e.0))?;
        AdminUser::from_auth_user(auth_user)
    }
}

// ---------------------------------------------------------------------------
// Admin State
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct AdminState {
    pub inner: Arc<AdminStateInner>,
}

pub struct AdminStateInner {
    pub api_state: ApiState,
}

impl AdminState {
    pub fn new(api_state: ApiState) -> Self {
        Self {
            inner: Arc::new(AdminStateInner { api_state }),
        }
    }

    pub fn pool(&self) -> &sqlx::PgPool {
        &self.inner.api_state.inner.pool
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn create_admin_router(state: ApiState) -> Router {
    let admin_state = AdminState::new(state);
    Router::new()
        // User management
        .route("/api/v1/admin/users", get(list_users))
        .route("/api/v1/admin/users/:id", put(update_user))
        .route("/api/v1/admin/users/:id", delete(delete_user))
        // System API keys
        .route("/api/v1/admin/api-keys", get(list_api_keys))
        .route("/api/v1/admin/api-keys", post(create_api_key))
        .route("/api/v1/admin/api-keys/:id", delete(delete_api_key))
        // Analytics
        .route("/api/v1/admin/analytics", get(analytics))
        // Content moderation
        .route("/api/v1/admin/content", get(list_content))
        .route("/api/v1/admin/content/:id", delete(delete_content))
        .with_state(admin_state)
}

// ---------------------------------------------------------------------------
// Request / Response Types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct ListUsersQuery {
    pub page: Option<i64>,
    pub limit: Option<i64>,
    pub search: Option<String>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct AdminUserSummary {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub role: String,
    pub subscription_tier: String,
    pub created_at: chrono::DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct ListUsersResponse {
    pub users: Vec<AdminUserSummary>,
    pub total: i64,
    pub page: i64,
    pub limit: i64,
}

#[derive(Debug, Deserialize)]
pub struct UpdateUserBody {
    pub role: Option<String>,
    pub subscription_tier: Option<String>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct SystemApiKeySummary {
    pub id: Uuid,
    pub provider: String,
    pub is_active: bool,
    pub rate_limit: i32,
    pub created_at: chrono::DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateApiKeyBody {
    pub provider: String,
    pub encrypted_key: String,
    pub rate_limit: i32,
}

#[derive(Debug, Serialize)]
pub struct DailyUserGrowth {
    pub date: NaiveDate,
    pub new_users: i64,
    pub total_users: i64,
}

#[derive(Debug, Serialize)]
pub struct CostSummary {
    pub total_api_cost_usd: f64,
    pub cost_by_provider: HashMap<String, f64>,
}

#[derive(Debug, Serialize)]
pub struct AdminAnalytics {
    pub total_users: i64,
    pub total_generations: i64,
    pub generations_by_provider: HashMap<String, i64>,
    pub user_growth: Vec<DailyUserGrowth>,
    pub cost_summary: CostSummary,
}

#[derive(Debug, Deserialize)]
pub struct ListContentQuery {
    pub page: Option<i64>,
    pub limit: Option<i64>,
    pub status: Option<String>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ContentSummary {
    pub id: Uuid,
    pub user_id: Uuid,
    pub prompt: String,
    pub provider: String,
    pub model: String,
    pub status: String,
    pub visibility: String,
    pub created_at: chrono::DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct ListContentResponse {
    pub items: Vec<ContentSummary>,
    pub total: i64,
    pub page: i64,
    pub limit: i64,
}

// ---------------------------------------------------------------------------
// User Management Handlers
// ---------------------------------------------------------------------------

/// GET /api/v1/admin/users
///
/// Paginated list of all users with optional search.
async fn list_users(
    State(state): State<AdminState>,
    _admin: AdminUser,
    Query(query): Query<ListUsersQuery>,
) -> AppResult<Json<ListUsersResponse>> {
    let page = query.page.unwrap_or(1).max(1);
    let limit = query.limit.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * limit;
    let pool = state.pool();

    let search_pattern = query
        .search
        .as_ref()
        .map(|s| format!("%{}%", s.to_lowercase()));

    let (users, total) = if let Some(ref pattern) = search_pattern {
        let rows: Vec<AdminUserSummary> = sqlx::query_as(
            r#"
            SELECT id, username, email, role, subscription_tier, created_at
            FROM users
            WHERE LOWER(username) LIKE $1 OR LOWER(email) LIKE $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(pattern)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?;

        let total: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM users
            WHERE LOWER(username) LIKE $1 OR LOWER(email) LIKE $1
            "#,
        )
        .bind(pattern)
        .fetch_one(pool)
        .await?;

        (rows, total)
    } else {
        let rows: Vec<AdminUserSummary> = sqlx::query_as(
            r#"
            SELECT id, username, email, role, subscription_tier, created_at
            FROM users
            ORDER BY created_at DESC
            LIMIT $1 OFFSET $2
            "#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?;

        let total: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM users")
                .fetch_one(pool)
                .await?;

        (rows, total)
    };

    Ok(Json(ListUsersResponse {
        users,
        total,
        page,
        limit,
    }))
}

/// PUT /api/v1/admin/users/:id
///
/// Update a user's role and/or subscription tier.
async fn update_user(
    State(state): State<AdminState>,
    _admin: AdminUser,
    Path(user_id): Path<String>,
    Json(body): Json<UpdateUserBody>,
) -> AppResult<impl IntoResponse> {
    let user_uuid = Uuid::parse_str(&user_id)
        .map_err(|_| AppError::Validation("invalid user ID".to_string()))?;

    let mut updates = Vec::new();
    let mut param_count = 0;

    if let Some(ref _role) = body.role {
        param_count += 1;
        updates.push(format!("role = ${}", param_count));
    }
    if let Some(ref _tier) = body.subscription_tier {
        param_count += 1;
        updates.push(format!("subscription_tier = ${}", param_count));
    }

    if updates.is_empty() {
        return Err(AppError::Validation(
            "no fields to update".to_string(),
        ));
    }

    param_count += 1;
    let query = format!(
        "UPDATE users SET {} WHERE id = ${}",
        updates.join(", "),
        param_count
    );

    let mut q = sqlx::query(&query);
    if let Some(ref role) = body.role {
        q = q.bind(role);
    }
    if let Some(ref tier) = body.subscription_tier {
        q = q.bind(tier);
    }
    q = q.bind(user_uuid);

    let result = q.execute(state.pool()).await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("user not found".to_string()));
    }

    Ok((StatusCode::OK, Json(serde_json::json!({ "message": "user updated" }))).into_response())
}

/// DELETE /api/v1/admin/users/:id
///
/// Suspend a user by inserting into account_deletions (soft delete with scheduled purge).
async fn delete_user(
    State(state): State<AdminState>,
    _admin: AdminUser,
    Path(user_id): Path<String>,
) -> AppResult<impl IntoResponse> {
    let user_uuid = Uuid::parse_str(&user_id)
        .map_err(|_| AppError::Validation("invalid user ID".to_string()))?;

    // Insert into account_deletions (soft delete)
    let result = sqlx::query(
        r#"
        INSERT INTO account_deletions (user_id, deleted_at, purge_after)
        VALUES ($1, NOW(), NOW() + INTERVAL '30 days')
        ON CONFLICT (user_id) DO NOTHING
        "#,
    )
    .bind(user_uuid)
    .execute(state.pool())
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("user not found".to_string()));
    }

    Ok((StatusCode::OK, Json(serde_json::json!({ "message": "user suspended" }))).into_response())
}

// ---------------------------------------------------------------------------
// System API Key Handlers
// ---------------------------------------------------------------------------

/// GET /api/v1/admin/api-keys
///
/// List all system API keys.
async fn list_api_keys(
    State(state): State<AdminState>,
    _admin: AdminUser,
) -> AppResult<Json<Vec<SystemApiKeySummary>>> {
    let rows: Vec<SystemApiKeySummary> = sqlx::query_as(
        r#"
        SELECT id, provider, is_active, rate_limit, created_at
        FROM system_api_keys
        ORDER BY created_at DESC
        "#,
    )
    .fetch_all(state.pool())
    .await?;

    Ok(Json(rows))
}

/// POST /api/v1/admin/api-keys
///
/// Add a new system API key.
async fn create_api_key(
    State(state): State<AdminState>,
    _admin: AdminUser,
    Json(body): Json<CreateApiKeyBody>,
) -> AppResult<impl IntoResponse> {
    if body.provider.is_empty() {
        return Err(AppError::Validation("provider is required".to_string()));
    }
    if body.encrypted_key.is_empty() {
        return Err(AppError::Validation("encrypted_key is required".to_string()));
    }
    if body.rate_limit <= 0 {
        return Err(AppError::Validation("rate_limit must be positive".to_string()));
    }

    let id: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO system_api_keys (id, provider, encrypted_key, rate_limit)
        VALUES (uuid_generate_v4(), $1, $2, $3)
        RETURNING id
        "#,
    )
    .bind(&body.provider)
    .bind(&body.encrypted_key)
    .bind(body.rate_limit)
    .fetch_one(state.pool())
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "id": id.to_string(),
            "provider": body.provider,
            "rate_limit": body.rate_limit,
            "message": "API key created"
        })),
    )
        .into_response())
}

/// DELETE /api/v1/admin/api-keys/:id
///
/// Remove a system API key.
async fn delete_api_key(
    State(state): State<AdminState>,
    _admin: AdminUser,
    Path(key_id): Path<String>,
) -> AppResult<impl IntoResponse> {
    let key_uuid = Uuid::parse_str(&key_id)
        .map_err(|_| AppError::Validation("invalid API key ID".to_string()))?;

    let result = sqlx::query("DELETE FROM system_api_keys WHERE id = $1")
        .bind(key_uuid)
        .execute(state.pool())
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("API key not found".to_string()));
    }

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "message": "API key deleted" })),
    )
        .into_response())
}

// ---------------------------------------------------------------------------
// Analytics Handlers
// ---------------------------------------------------------------------------

/// GET /api/v1/admin/analytics
///
/// Return usage analytics including user counts, generation stats,
/// user growth over the last 30 days, and cost summary.
async fn analytics(
    State(state): State<AdminState>,
    _admin: AdminUser,
) -> AppResult<Json<AdminAnalytics>> {
    let pool = state.pool();

    // Total users
    let total_users: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM users")
            .fetch_one(pool)
            .await?;

    // Total generations
    let total_generations: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM generations")
            .fetch_one(pool)
            .await?;

    // Generations by provider
    #[derive(Debug, sqlx::FromRow)]
    struct ProviderCount {
        pub provider: String,
        pub count: i64,
    }
    let by_provider: Vec<ProviderCount> = sqlx::query_as(
        "SELECT provider, COUNT(*) as count FROM generations GROUP BY provider",
    )
    .fetch_all(pool)
    .await?;

    let mut generations_by_provider = HashMap::new();
    for row in by_provider {
        generations_by_provider.insert(row.provider, row.count);
    }

    // User growth over last 30 days
    let thirty_days_ago = (Utc::now() - ChronoDuration::days(30)).date_naive();

    #[derive(Debug, sqlx::FromRow)]
    struct DailySignup {
        pub date: NaiveDate,
        pub new_users: i64,
    }
    let signups: Vec<DailySignup> = sqlx::query_as(
        r#"
        SELECT DATE(created_at) as date, COUNT(*) as new_users
        FROM users
        WHERE created_at >= $1
        GROUP BY DATE(created_at)
        ORDER BY date ASC
        "#,
    )
    .bind(thirty_days_ago)
    .fetch_all(pool)
    .await?;

    let mut user_growth = Vec::new();
    let mut cumulative = 0i64;

    for signup in signups {
        cumulative += signup.new_users;
        user_growth.push(DailyUserGrowth {
            date: signup.date,
            new_users: signup.new_users,
            total_users: cumulative,
        });
    }

    // Fill in days with zero new users
    let today = Utc::now().date_naive();
    let mut current_date = thirty_days_ago;
    while current_date <= today {
        if !user_growth.iter().any(|g| g.date == current_date) {
            user_growth.push(DailyUserGrowth {
                date: current_date,
                new_users: 0,
                total_users: cumulative,
            });
        } else {
            // Update cumulative total from the entry
            if let Some(last) = user_growth.iter().filter(|g| g.date == current_date).last() {
                cumulative = last.total_users;
            }
        }
        current_date += ChronoDuration::days(1);
    }

    user_growth.sort_by_key(|g| g.date);

    // Cost summary from generations table (using credits_used as a proxy for cost)
    #[derive(Debug, sqlx::FromRow)]
    struct ProviderCost {
        pub provider: String,
        pub total_credits: i64,
    }
    let costs: Vec<ProviderCost> = sqlx::query_as(
        r#"
        SELECT provider, SUM(credits_used) as total_credits
        FROM generations
        GROUP BY provider
        "#,
    )
    .fetch_all(pool)
    .await?;

    // Convert credits to estimated USD (1 credit ~ $0.01 for rough estimation)
    let mut cost_by_provider = HashMap::new();
    let mut total_cost = 0.0;
    for c in costs {
        let cost = c.total_credits as f64 * 0.01;
        cost_by_provider.insert(c.provider, cost);
        total_cost += cost;
    }

    let cost_summary = CostSummary {
        total_api_cost_usd: total_cost,
        cost_by_provider,
    };

    Ok(Json(AdminAnalytics {
        total_users,
        total_generations,
        generations_by_provider,
        user_growth,
        cost_summary,
    }))
}

// ---------------------------------------------------------------------------
// Content Moderation Handlers
// ---------------------------------------------------------------------------

/// GET /api/v1/admin/content
///
/// Paginated list of all generations with optional status filter.
async fn list_content(
    State(state): State<AdminState>,
    _admin: AdminUser,
    Query(query): Query<ListContentQuery>,
) -> AppResult<Json<ListContentResponse>> {
    let page = query.page.unwrap_or(1).max(1);
    let limit = query.limit.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * limit;

    let status_filter = query.status.as_deref();

    let rows: Vec<ContentSummary> = sqlx::query_as(
        r#"
        SELECT id, user_id, prompt, provider, model, status, visibility, created_at
        FROM generations
        WHERE ($1::text IS NULL OR status = $1)
        ORDER BY created_at DESC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(status_filter)
    .bind(limit)
    .bind(offset)
    .fetch_all(state.pool())
    .await?;

    let total: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM generations
        WHERE ($1::text IS NULL OR status = $1)
        "#,
    )
    .bind(status_filter)
    .fetch_one(state.pool())
    .await?;

    Ok(Json(ListContentResponse {
        items: rows,
        total,
        page,
        limit,
    }))
}

/// DELETE /api/v1/admin/content/:id
///
/// Remove a flagged generation (hard delete).
async fn delete_content(
    State(state): State<AdminState>,
    _admin: AdminUser,
    Path(content_id): Path<String>,
) -> AppResult<impl IntoResponse> {
    let gen_uuid = Uuid::parse_str(&content_id)
        .map_err(|_| AppError::Validation("invalid content ID".to_string()))?;

    let result = sqlx::query("DELETE FROM generations WHERE id = $1")
        .bind(gen_uuid)
        .execute(state.pool())
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("content not found".to_string()));
    }

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "message": "content removed" })),
    )
        .into_response())
}
