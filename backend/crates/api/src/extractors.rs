//! # Request Extractors
//!
//! Custom Axum extractors for extracting authenticated user from requests.

use axum::{
    extract::FromRequestParts,
    http::header::AUTHORIZATION,
    http::StatusCode,
    response::{IntoResponse, Response},
    Extension,
};
use axum_extra::extract::CookieJar;
use common::AppError;

use crate::ApiState;

/// Authenticated user extracted from a verified JWT token.
///
/// Carried through request handlers to access the identity of the
/// currently logged-in user.
#[derive(Debug, Clone)]
pub struct AuthUser {
    /// The user's UUID.
    pub user_id: String,
    /// The user's username.
    pub username: String,
    /// The user's role.
    pub role: String,
}

impl AuthUser {
    /// Create a new AuthUser from JWT claims.
    pub fn from_claims(
        user_id: impl Into<String>,
        username: impl Into<String>,
        role: impl Into<String>,
    ) -> Self {
        Self {
            user_id: user_id.into(),
            username: username.into(),
            role: role.into(),
        }
    }
}

impl<S: Send + Sync> FromRequestParts<S> for AuthUser {
    type Rejection = AuthError;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        // Extract ApiState from the Extension layer
        let Extension(api_state): Extension<ApiState> =
            Extension::from_request_parts(parts, _state)
                .await
                .map_err(|_| AuthError("failed to extract app state".into()))?;

        let config = &api_state.inner.config;

        // Build JWT service from the config
        let jwt_service = auth::JwtService::new(
            &config.jwt_secret,
            "creative-ai-studio",
            "creative-ai-studio-api",
        );

        // Try to extract token from cookie first, then Authorization header
        let token = extract_token(parts).await?;

        // Verify the token
        let claims = jwt_service
            .verify_token(&token)
            .map_err(|e| AuthError(format!("invalid token: {}", e)))?;

        Ok(AuthUser::from_claims(claims.sub, claims.username, claims.role))
    }
}

/// Extract the JWT token from the request.
///
/// Checks cookies first (`access_token`), then falls back to the
/// `Authorization: Bearer <token>` header.
async fn extract_token(
    parts: &mut axum::http::request::Parts,
) -> Result<String, AuthError> {
    // Try cookie first
    let cookies = CookieJar::from_request_parts(parts, &())
        .await
        .map_err(|_| AuthError("failed to extract cookies".into()))?;
    if let Some(token) = cookies.get("access_token") {
        let val = token.value();
        return Ok(val.to_string());
    }

    // Fall back to Authorization header
    let headers = &parts.headers;
    if let Some(value) = headers.get(AUTHORIZATION) {
        if let Ok(auth_str) = value.to_str() {
            if let Some(token) = auth_str.strip_prefix("Bearer ") {
                return Ok(token.trim().to_string());
            }
        }
    }

    Err(AuthError("missing authentication token".into()))
}

/// Authentication error that maps to a 401 response.
#[derive(Debug)]
pub struct AuthError(pub String);

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let status = StatusCode::UNAUTHORIZED;
        let body = axum::Json(serde_json::json!({
            "error": self.0,
            "code": status.as_u16()
        }));
        (status, body).into_response()
    }
}

impl From<AuthError> for AppError {
    fn from(e: AuthError) -> Self {
        AppError::Unauthorized(e.0)
    }
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for AuthError {}
