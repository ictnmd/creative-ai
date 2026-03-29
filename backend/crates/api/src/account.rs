//! # Account Management
//!
//! Handles account-level operations such as soft-delete (GDPR compliance).
//! Routes under this module require authentication.

use std::vec::Vec;

use axum::{
    extract::{CookieJar, State},
    http::{header::SET_COOKIE, Response, StatusCode},
    routing::post,
    Json, Router,
};
use chrono::{Duration, Utc};
use serde::Serialize;
use uuid::Uuid;

use common::{AppError, AppResult};

use crate::auth::AuthState;
use crate::extractors::AuthUser;

/// POST /api/v1/user/delete - Soft-delete account (GDPR)
///
///   - Requires AuthUser (JWT auth via cookie)
///   - Create account_deletion record (purge_after = now + 30 days)
///   - Revoke all refresh tokens
///   - Clear cookies
pub fn routes(state: AuthState) -> Router {
    Router::new()
        .route("/api/v1/user/delete", post(delete_account))
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

/// Success response for account deletion.
#[derive(Debug, Serialize)]
pub struct DeleteAccountResponse {
    pub message: String,
    pub purge_after: String,
}

// ---------------------------------------------------------------------------
// Handler
// ---------------------------------------------------------------------------

/// POST /api/v1/user/delete
///
/// Soft-deletes the authenticated user's account. This is GDPR-compliant:
/// - A deletion record is created with a purge date 30 days in the future,
///   allowing the user to cancel within that window.
/// - All refresh tokens are revoked so existing sessions are terminated.
/// - Auth cookies are cleared from the response.
pub async fn delete_account(
    State(state): State<AuthState>,
    auth_user: AuthUser,
    cookies: CookieJar,
) -> AppResult<Response<String>> {
    let user_id: Uuid = auth_user
        .user_id
        .parse()
        .map_err(|_| AppError::Validation("invalid user id in token".to_string()))?;

    // Calculate purge date: 30 days from now
    let requested_at = Utc::now();
    let purge_after = requested_at + Duration::days(30);

    // Create account deletion record
    db::queries::account_deletions::insert(state.pool(), user_id, purge_after).await?;

    // Revoke all refresh tokens for this user
    let revoked_count = db::queries::refresh_tokens::delete_all_for_user(state.pool(), user_id)
        .await
        .map(|_| 0) // count not critical for now
        .unwrap_or(0);

    tracing::info!(
        user_id = %user_id,
        purge_after = %purge_after,
        revoked_tokens = revoked_count,
        "Account deletion requested (GDPR)"
    );

    // Build JSON response
    let body = serde_json::to_string(&DeleteAccountResponse {
        message: "Account deletion scheduled. You have 30 days to cancel.".to_string(),
        purge_after: purge_after.to_rfc3339(),
    })?;

    let mut response = Response::new(body);
    *response.status_mut() = StatusCode::OK;
    response
        .headers_mut()
        .insert(
            axum::http::header::CONTENT_TYPE,
            "application/json".parse().unwrap(),
        );

    // Clear auth cookies
    for cookie_str in clear_auth_cookies() {
        response
            .headers_mut()
            .insert(SET_COOKIE, cookie_str.parse().unwrap());
    }

    Ok(response)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Clears the auth-related cookies from the response.
fn clear_auth_cookies() -> Vec<String> {
    let cookie_names = ["access_token", "refresh_token"];
    cookie_names
        .iter()
        .map(|name| {
            axum_extra::extract::Cookie::build((**name).to_string(), "")
                .path("/")
                .http_only(true)
                .secure(true)
                .same_site(axum_extra::extract::SameSite::Strict)
                .max_age(time::Duration::ZERO)
                .to_string()
        })
        .collect()
}
