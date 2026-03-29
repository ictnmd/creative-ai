//! # Authentication Routes
//!
//! Handles user registration, login, logout, token refresh, OAuth flows,
//! and session management.

use crate::extractors::AuthUser;
use crate::ApiState;
use axum::{
    extract::{Path, Query, State},
    http::{header::SET_COOKIE, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use axum_extra::extract::CookieJar;
use chrono::{Duration, Utc};
use common::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Shared Types
// ---------------------------------------------------------------------------

/// Request body for email/password registration.
#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub username: String,
    pub password: String,
}

/// Request body for email/password login.
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

/// Response body returned after successful registration/login/refresh.
#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub user: UserInfo,
    pub access_token: String,
    pub refresh_token: String,
}

/// Public user info returned in auth responses.
#[derive(Debug, Serialize)]
pub struct UserInfo {
    pub id: String,
    pub username: String,
    pub email: String,
    pub role: String,
    pub avatar_url: Option<String>,
    pub subscription_tier: String,
}

/// OAuth state stored in DragonflyDB/Redis during PKCE flow.
#[derive(Debug, Serialize, Deserialize)]
pub struct OAuthState {
    pub state: String,
    pub code_verifier: String,
    pub redirect_uri: Option<String>,
}

// ---------------------------------------------------------------------------
// Auth State
// ---------------------------------------------------------------------------

/// Application state for auth handlers.
#[derive(Clone)]
pub struct AuthState {
    pub inner: Arc<AuthStateInner>,
}

pub struct AuthStateInner {
    pub api_state: ApiState,
}

impl AuthState {
    pub fn new(api_state: ApiState) -> Self {
        Self {
            inner: Arc::new(AuthStateInner { api_state }),
        }
    }

    pub fn pool(&self) -> &sqlx::PgPool {
        &self.inner.api_state.inner.pool
    }

    pub fn redis(&self) -> &Arc<parking_lot::RwLock<redis::aio::ConnectionManager>> {
        &self.inner.api_state.inner.redis
    }

    fn config(&self) -> &common::AppConfig {
        &self.inner.api_state.inner.config
    }

    fn jwt_service(&self) -> auth::JwtService {
        let cfg = self.config();
        auth::JwtService::new(
            &cfg.jwt_secret,
            "creative-ai-studio",
            "creative-ai-studio-api",
        )
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

/// Create the auth router.
pub fn create_auth_router(state: ApiState) -> Router {
    let auth_state = AuthState::new(state);
    Router::new()
        .route("/api/v1/auth/register", post(register))
        .route("/api/v1/auth/login", post(login))
        .route("/api/v1/auth/logout", post(logout))
        .route("/api/v1/auth/refresh", post(refresh))
        .route("/api/v1/auth/revoke-all", post(revoke_all))
        .route("/api/v1/auth/oauth/:provider", get(initiate_oauth))
        .route("/api/v1/auth/oauth/:provider/callback", get(oauth_callback))
        .with_state(auth_state)
}

// ---------------------------------------------------------------------------
// Cookie helpers
// ---------------------------------------------------------------------------

const ACCESS_COOKIE_NAME: &str = "access_token";
const REFRESH_COOKIE_NAME: &str = "refresh_token";

/// Build a Set-Cookie header string for the access token (15-minute expiry).
fn access_cookie(value: &str, is_prod: bool) -> String {
    let max_age = 15 * 60;
    let mut parts = format!(
        "{}={}; Path=/; HttpOnly; Max-Age={}",
        ACCESS_COOKIE_NAME, value, max_age
    );
    if is_prod {
        parts.push_str("; Secure");
    }
    parts
}

/// Build a Set-Cookie header string for the refresh token (7-day expiry).
fn refresh_cookie_fn(value: &str, is_prod: bool) -> String {
    let max_age = 7 * 24 * 3600;
    let mut parts = format!(
        "{}={}; Path=/; HttpOnly; Max-Age={}",
        REFRESH_COOKIE_NAME, value, max_age
    );
    if is_prod {
        parts.push_str("; Secure");
    }
    parts
}

/// Build a Set-Cookie header string to clear a cookie.
fn cleared_cookie(name: &str) -> String {
    format!("{}={}; Path=/; HttpOnly; Max-Age=0", name, "")
}

/// Build a JSON response with Set-Cookie headers.
fn json_with_cookies<T: Serialize>(
    status: StatusCode,
    body: T,
    cookies: impl IntoIterator<Item = String>,
) -> Response {
    let body_bytes = serde_json::to_vec(&body).unwrap();
    let mut response = Response::new(axum::body::Body::from(body_bytes));
    *response.status_mut() = status;
    response
        .headers_mut()
        .insert(axum::http::header::CONTENT_TYPE, "application/json".parse().unwrap());
    for cookie in cookies {
        response
            .headers_mut()
            .insert(SET_COOKIE, cookie.parse().unwrap());
    }
    response
}

// ---------------------------------------------------------------------------
// Token helpers
// ---------------------------------------------------------------------------

/// Hash a refresh token using SHA-256 for storage.
fn hash_token(token: &str) -> String {
    let digest = Sha256::digest(token.as_bytes());
    hex::encode(digest)
}

/// Store a refresh token in the DB.
async fn store_refresh_token(
    pool: &sqlx::PgPool,
    user_id: Uuid,
    token: &str,
    expires_at: chrono::DateTime<Utc>,
) -> AppResult<()> {
    let hash = hash_token(token);
    db::queries::refresh_tokens::insert(pool, user_id, &hash, expires_at).await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// POST /api/v1/auth/register
///
/// Register a new user with email and password.
async fn register(
    State(state): State<AuthState>,
    Json(req): Json<RegisterRequest>,
) -> AppResult<Response> {
    // Validate email format
    if !req.email.contains('@') || req.email.len() > 255 {
        return Err(AppError::Validation("invalid email address".to_string()));
    }

    // Validate username
    if req.username.len() < 3 || req.username.len() > 50 {
        return Err(AppError::Validation(
            "username must be between 3 and 50 characters".to_string(),
        ));
    }
    if !req
        .username
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
    {
        return Err(AppError::Validation(
            "username may only contain letters, numbers, underscores, and hyphens"
                .to_string(),
        ));
    }

    // Validate password strength
    auth::validate_password_strength(&req.password)?;

    // Check if email already exists
    if db::queries::users::find_by_email(state.pool(), &req.email)
        .await?
        .is_some()
    {
        return Err(AppError::Conflict("email already registered".to_string()));
    }

    // Hash the password
    let password_hash = auth::hash_password(&req.password)?;

    // Create the user
    let user_id = db::queries::users::create_user(
        state.pool(),
        &req.email,
        &req.username,
        &password_hash,
    )
    .await?;

    // Fetch the created user for token generation
    let user = db::queries::users::get_user_by_id(state.pool(), user_id)
        .await?
        .ok_or_else(|| AppError::Internal("user creation failed".to_string()))?;

    // Create the user profile
    db::queries::users::create_profile(state.pool(), user_id).await?;

    // Generate tokens
    let jwt_svc = state.jwt_service();
    let access_token =
        jwt_svc.generate_token(&user.id.to_string(), &user.username, &user.role)?;
    let refresh_token = jwt_svc.generate_refresh_token(&user.id.to_string())?;

    // Store refresh token
    let refresh_expires = Utc::now() + Duration::days(7);
    store_refresh_token(state.pool(), user.id, &refresh_token, refresh_expires).await?;

    // Build cookies
    let is_prod = state.config().is_production();
    let access_ck = access_cookie(&access_token, is_prod);
    let refresh_ck = refresh_cookie_fn(&refresh_token, is_prod);

    let response = AuthResponse {
        user: UserInfo {
            id: user.id.to_string(),
            username: user.username,
            email: user.email,
            role: user.role,
            avatar_url: None,
            subscription_tier: "free".to_string(),
        },
        access_token,
        refresh_token,
    };

    Ok(json_with_cookies(StatusCode::CREATED, response, [refresh_ck, access_ck]))
}

/// POST /api/v1/auth/login
///
/// Authenticate a user with email and password.
async fn login(State(state): State<AuthState>, Json(req): Json<LoginRequest>) -> AppResult<Response> {
    // Find user by email
    let user = db::queries::users::find_by_email(state.pool(), &req.email)
        .await?
        .ok_or_else(|| AppError::Unauthorized("invalid credentials".to_string()))?;

    // Verify password
    let password_hash = user.password_hash.as_deref().unwrap_or("");
    if !auth::verify_password(&req.password, password_hash) {
        return Err(AppError::Unauthorized("invalid credentials".to_string()));
    }

    // Generate tokens
    let jwt_svc = state.jwt_service();
    let access_token =
        jwt_svc.generate_token(&user.id.to_string(), &user.username, &user.role)?;
    let refresh_token = jwt_svc.generate_refresh_token(&user.id.to_string())?;

    // Store refresh token
    let refresh_expires = Utc::now() + Duration::days(7);
    store_refresh_token(state.pool(), user.id, &refresh_token, refresh_expires).await?;

    // Build cookies
    let is_prod = state.config().is_production();
    let access_ck = access_cookie(&access_token, is_prod);
    let refresh_ck = refresh_cookie_fn(&refresh_token, is_prod);

    let response = AuthResponse {
        user: UserInfo {
            id: user.id.to_string(),
            username: user.username,
            email: user.email,
            role: user.role,
            avatar_url: None,
            subscription_tier: "free".to_string(),
        },
        access_token,
        refresh_token,
    };

    Ok(json_with_cookies(StatusCode::OK, response, [refresh_ck, access_ck]))
}

/// POST /api/v1/auth/logout
///
/// Clear the auth cookies and revoke all refresh tokens.
async fn logout(
    State(state): State<AuthState>,
    auth_user: AuthUser,
) -> AppResult<Response> {
    let user_id: Uuid = auth_user
        .user_id
        .parse()
        .map_err(|_| AppError::Validation("invalid user id".to_string()))?;

    // Delete all refresh tokens for this user
    db::queries::refresh_tokens::delete_all_for_user(state.pool(), user_id).await?;

    // Clear cookies
    let access_ck = cleared_cookie(ACCESS_COOKIE_NAME);
    let refresh_ck = cleared_cookie(REFRESH_COOKIE_NAME);

    Ok(json_with_cookies(
        StatusCode::OK,
        serde_json::json!({ "message": "logged out" }),
        [refresh_ck, access_ck],
    ))
}

/// POST /api/v1/auth/refresh
///
/// Exchange a refresh token for a new access + refresh token pair.
/// Implements refresh token rotation: old token is revoked, new tokens issued.
async fn refresh(
    State(state): State<AuthState>,
    cookies: CookieJar,
) -> AppResult<Response> {
    // Read the refresh token from cookie
    let cookie = cookies
        .get(REFRESH_COOKIE_NAME)
        .ok_or_else(|| AppError::Unauthorized("missing refresh token".to_string()))?;
    let token = cookie.value().to_string();

    // Verify refresh token
    let jwt_svc = state.jwt_service();
    let claims = jwt_svc.verify_refresh_token(&token)?;

    let user_id: Uuid = claims
        .sub
        .parse()
        .map_err(|_| AppError::Unauthorized("invalid refresh token".to_string()))?;

    // Verify the token hash exists in DB (rotation check)
    let hash = hash_token(&token);
    let stored = db::queries::refresh_tokens::find_by_hash(state.pool(), &hash).await?;
    if stored.is_none() {
        // Token not found in DB — could be a reused token, revoke all
        db::queries::refresh_tokens::delete_all_for_user(state.pool(), user_id).await?;
        return Err(AppError::Unauthorized(
            "refresh token reuse detected".to_string(),
        ));
    }

    // Revoke the old refresh token
    db::queries::refresh_tokens::delete_by_hash(state.pool(), &hash).await?;

    // Fetch user info for new access token
    let user = db::queries::users::get_user_by_id(state.pool(), user_id)
        .await?
        .ok_or_else(|| AppError::Unauthorized("user not found".to_string()))?;

    // Generate new tokens
    let new_access =
        jwt_svc.generate_token(&user_id.to_string(), &user.username, &user.role)?;
    let new_refresh = jwt_svc.generate_refresh_token(&user_id.to_string())?;

    // Store new refresh token
    let refresh_expires = Utc::now() + Duration::days(7);
    store_refresh_token(state.pool(), user_id, &new_refresh, refresh_expires).await?;

    // Build new cookies
    let is_prod = state.config().is_production();
    let access_ck = access_cookie(&new_access, is_prod);
    let refresh_ck = refresh_cookie_fn(&new_refresh, is_prod);

    let response = AuthResponse {
        user: UserInfo {
            id: user.id.to_string(),
            username: user.username,
            email: user.email,
            role: user.role,
            avatar_url: None,
            subscription_tier: "free".to_string(),
        },
        access_token: new_access,
        refresh_token: new_refresh,
    };

    Ok(json_with_cookies(StatusCode::OK, response, [refresh_ck, access_ck]))
}

/// POST /api/v1/auth/revoke-all
///
/// Revoke ALL refresh tokens for the authenticated user.
async fn revoke_all(
    State(state): State<AuthState>,
    auth_user: AuthUser,
) -> AppResult<Response> {
    let user_id: Uuid = auth_user
        .user_id
        .parse()
        .map_err(|_| AppError::Validation("invalid user id".to_string()))?;

    db::queries::refresh_tokens::delete_all_for_user(state.pool(), user_id).await?;

    Ok((
        StatusCode::OK,
        axum::Json(serde_json::json!({ "message": "all refresh tokens revoked" })),
    )
        .into_response())
}

// ---------------------------------------------------------------------------
// OAuth Handlers
// ---------------------------------------------------------------------------

/// GET /api/v1/auth/oauth/:provider
///
/// Initiate an OAuth flow with PKCE. Supports Google and GitHub.
async fn initiate_oauth(
    State(state): State<AuthState>,
    Path(provider): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let provider = provider.to_lowercase();

    // Validate provider
    match provider.as_str() {
        "google" | "github" => {}
        _ => return Err(AppError::Validation("unsupported OAuth provider".to_string())),
    }

    let config = state.config();

    // Get provider credentials
    let (client_id, _client_secret) = match provider.as_str() {
        "google" => (
            config
                .google_client_id
                .as_deref()
                .ok_or_else(|| AppError::Config("google_client_id not configured".to_string()))?,
            config
                .google_client_secret
                .as_deref()
                .ok_or_else(|| AppError::Config("google_client_secret not configured".to_string()))?,
        ),
        "github" => (
            config
                .github_client_id
                .as_deref()
                .ok_or_else(|| AppError::Config("github_client_id not configured".to_string()))?,
            config
                .github_client_secret
                .as_deref()
                .ok_or_else(|| AppError::Config("github_client_secret not configured".to_string()))?,
        ),
        _ => unreachable!(),
    };

    // Generate PKCE pair
    let state_param = Uuid::new_v4().to_string();
    let code_verifier = generate_code_verifier();
    let code_challenge = generate_code_challenge(&code_verifier);

    // Build the authorization URL
    let redirect_uri = format!("{}/api/v1/auth/oauth/{}/callback", config.backend_url, provider);
    let auth_url = match provider.as_str() {
        "google" => {
            let url = reqwest::Url::parse_with_params(
                "https://accounts.google.com/o/oauth2/v2/auth",
                &[
                    ("client_id", client_id),
                    ("redirect_uri", redirect_uri.as_str()),
                    ("response_type", "code"),
                    ("scope", "openid email profile"),
                    ("state", state_param.as_str()),
                    ("code_challenge", code_challenge.as_str()),
                    ("code_challenge_method", "S256"),
                ],
            )
            .map_err(|e| AppError::Config(format!("invalid OAuth URL: {}", e)))?;
            url.to_string()
        }
        "github" => {
            let url = reqwest::Url::parse_with_params(
                "https://github.com/login/oauth/authorize",
                &[
                    ("client_id", client_id),
                    ("redirect_uri", redirect_uri.as_str()),
                    ("scope", "user:email"),
                    ("state", state_param.as_str()),
                    ("code_challenge", code_challenge.as_str()),
                    ("code_challenge_method", "S256"),
                ],
            )
            .map_err(|e| AppError::Config(format!("invalid OAuth URL: {}", e)))?;
            url.to_string()
        }
        _ => unreachable!(),
    };

    // Store OAuth state in Redis (state -> {code_verifier, redirect_uri})
    let oauth_state = OAuthState {
        state: state_param.clone(),
        code_verifier,
        redirect_uri: None,
    };

    let state_json = serde_json::to_string(&oauth_state)
        .map_err(|e| AppError::Internal(format!("failed to serialize OAuth state: {}", e)))?;

    let redis_key = format!("oauth:state:{}", state_param);
    let conn = {
        let redis_guard = state.redis().read();
        (*redis_guard).clone()
    };
    redis::cmd("SETEX")
        .arg(&redis_key)
        .arg(600) // 10-minute TTL
        .arg(&state_json)
        .query_async::<()>(&mut conn.clone())
        .await
        .map_err(common::AppError::from)?;

    // Redirect to provider
    Ok((
        StatusCode::FOUND,
        [(axum::http::header::LOCATION, auth_url)],
    )
        .into_response())
}

/// GET /api/v1/auth/oauth/:provider/callback
///
/// Handle the OAuth callback from the provider, exchange code for tokens.
async fn oauth_callback(
    State(state): State<AuthState>,
    Path(provider): Path<String>,
    Query(params): Query<OAuthCallbackParams>,
) -> Result<impl IntoResponse, AppError> {
    let provider = provider.to_lowercase();

    let code = params
        .code
        .as_ref()
        .ok_or_else(|| AppError::Validation("missing OAuth code".to_string()))?;
    let state_param = params
        .state
        .as_ref()
        .ok_or_else(|| AppError::Validation("missing OAuth state".to_string()))?;

    // Validate provider
    match provider.as_str() {
        "google" | "github" => {}
        _ => return Err(AppError::Validation("unsupported OAuth provider".to_string())),
    }

    let config = state.config();

    // Retrieve and validate state from Redis
    let redis_key = format!("oauth:state:{}", state_param);
    let mut conn = {
        let redis_guard = state.redis().read();
        (*redis_guard).clone()
    };
    let state_json: Option<String> = redis::cmd("GET")
        .arg(&redis_key)
        .query_async(&mut conn)
        .await
        .map_err(common::AppError::from)?;

    let oauth_state: OAuthState = match state_json {
        Some(json) => serde_json::from_str(&json)
            .map_err(|e| AppError::Internal(format!("failed to deserialize OAuth state: {}", e)))?,
        None => {
            return Err(AppError::Unauthorized(
                "invalid or expired OAuth state".to_string(),
            ))
        }
    };

    // Delete state from Redis (one-time use)
    let mut conn2 = {
        let redis_guard = state.redis().read();
        (*redis_guard).clone()
    };
    redis::cmd("DEL")
        .arg(&redis_key)
        .query_async::<()>(&mut conn2)
        .await
        .map_err(common::AppError::from)?;

    // Exchange code for access token
    let (_provider_user_id, user_email, user_name, user_avatar) = match provider.as_str() {
        "google" => {
            exchange_google_code(
                code,
                &oauth_state.code_verifier,
                config.google_client_id.as_deref().unwrap(),
                config.google_client_secret.as_deref().unwrap(),
                &config.backend_url,
            )
            .await?
        }
        "github" => {
            exchange_github_code(
                code,
                &oauth_state.code_verifier,
                config.github_client_id.as_deref().unwrap(),
                config.github_client_secret.as_deref().unwrap(),
                &config.backend_url,
            )
            .await?
        }
        _ => unreachable!(),
    };

    // Find or create user
    let email = user_email.as_ref().ok_or_else(|| {
        AppError::Validation("OAuth provider did not return an email".to_string())
    })?;

    let user_id = if let Some(existing) =
        db::queries::users::find_oauth_user_by_email(state.pool(), email, &provider).await?
    {
        existing.id
    } else {
        // Determine username from provider data
        let raw_login = user_name
            .as_ref()
            .or(user_email.as_ref())
            .map(|s| s.split('@').next().unwrap_or("user"))
            .unwrap_or("user");
        let username = sanitize_username(raw_login);

        // Ensure username uniqueness
        let username = ensure_unique_username(state.pool(), &username).await?;

        db::queries::users::create_oauth_user(
            state.pool(),
            email,
            &username,
            &provider,
            user_name.as_deref(),
            user_avatar.as_deref(),
        )
        .await?
    };

    // Fetch user record
    let user = db::queries::users::get_user_by_id(state.pool(), user_id)
        .await?
        .ok_or_else(|| AppError::Internal("OAuth user not found after creation".to_string()))?;

    // Generate JWT tokens
    let jwt_svc = state.jwt_service();
    let access_token =
        jwt_svc.generate_token(&user_id.to_string(), &user.username, &user.role)?;
    let refresh_token = jwt_svc.generate_refresh_token(&user_id.to_string())?;

    // Store refresh token
    let refresh_expires = Utc::now() + Duration::days(7);
    store_refresh_token(state.pool(), user_id, &refresh_token, refresh_expires).await?;

    // Build cookies
    let is_prod = config.is_production();
    let access_ck = access_cookie(&access_token, is_prod);
    let refresh_ck = refresh_cookie_fn(&refresh_token, is_prod);

    // Redirect to frontend callback page. Tokens are already set as httpOnly cookies
    // via the Set-Cookie headers below. The callback page can read them from cookies.
    let redirect_url = format!("{}/auth/callback", config.frontend_url);

    let mut response = Response::new(axum::body::Body::empty());
    *response.status_mut() = StatusCode::FOUND;
    response
        .headers_mut()
        .insert(axum::http::header::LOCATION, redirect_url.parse().unwrap());
    response
        .headers_mut()
        .insert(SET_COOKIE, refresh_ck.parse().unwrap());
    response
        .headers_mut()
        .insert(SET_COOKIE, access_ck.parse().unwrap());
    Ok(response)
}

// ---------------------------------------------------------------------------
// OAuth Helpers
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct OAuthCallbackParams {
    code: Option<String>,
    state: Option<String>,
}

async fn exchange_google_code(
    code: &str,
    code_verifier: &str,
    client_id: &str,
    client_secret: &str,
    backend_url: &str,
) -> AppResult<(String, Option<String>, Option<String>, Option<String>)> {
    #[derive(Deserialize)]
    struct GoogleTokenResponse {
        access_token: String,
    }
    #[derive(Deserialize)]
    struct GoogleUserResponse {
        id: String,
        email: Option<String>,
        name: Option<String>,
        picture: Option<String>,
    }

    let redirect_uri = format!("{}/api/v1/auth/oauth/google/callback", backend_url);

    let token_resp: GoogleTokenResponse = reqwest::Client::new()
        .post("https://oauth2.googleapis.com/token")
        .form(&[
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("code", code),
            ("grant_type", "authorization_code"),
            ("redirect_uri", redirect_uri.as_str()),
            ("code_verifier", code_verifier),
        ])
        .send()
        .await
        .map_err(AppError::Http)?
        .json()
        .await
        .map_err(|e| AppError::Internal(format!("failed to parse Google token response: {}", e)))?;

    let user_resp: GoogleUserResponse = reqwest::Client::new()
        .get("https://www.googleapis.com/oauth2/v2/userinfo")
        .bearer_auth(&token_resp.access_token)
        .send()
        .await
        .map_err(AppError::Http)?
        .json()
        .await
        .map_err(|e| AppError::Internal(format!("failed to parse Google user response: {}", e)))?;

    Ok((
        user_resp.id,
        user_resp.email,
        user_resp.name,
        user_resp.picture,
    ))
}

async fn exchange_github_code(
    code: &str,
    code_verifier: &str,
    client_id: &str,
    client_secret: &str,
    backend_url: &str,
) -> AppResult<(String, Option<String>, Option<String>, Option<String>)> {
    #[derive(Deserialize)]
    struct GithubTokenResponse {
        access_token: String,
    }
    #[derive(Deserialize)]
    struct GithubUserResponse {
        id: i64,
        login: String,
        email: Option<String>,
        name: Option<String>,
        avatar_url: Option<String>,
    }
    #[derive(Deserialize)]
    struct GithubEmailResponse {
        email: String,
        primary: bool,
        verified: bool,
    }

    let redirect_uri = format!("{}/api/v1/auth/oauth/github/callback", backend_url);

    // Exchange code for access token
    let token_resp: GithubTokenResponse = reqwest::Client::new()
        .post("https://github.com/login/oauth/access_token")
        .header("Accept", "application/json")
        .form(&[
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("code", code),
            ("redirect_uri", redirect_uri.as_str()),
            ("code_verifier", code_verifier),
        ])
        .send()
        .await
        .map_err(AppError::Http)?
        .json()
        .await
        .map_err(|e| {
            AppError::Internal(format!("failed to parse GitHub token response: {}", e))
        })?;

    // Fetch user info
    let user_resp: GithubUserResponse = reqwest::Client::new()
        .get("https://api.github.com/user")
        .header("Accept", "application/vnd.github.v3+json")
        .bearer_auth(&token_resp.access_token)
        .send()
        .await
        .map_err(AppError::Http)?
        .json()
        .await
        .map_err(|e| AppError::Internal(format!("failed to parse GitHub user response: {}", e)))?;

    // Fetch primary verified email if not returned
    let email = if let Some(e) = user_resp.email {
        Some(e)
    } else {
        let emails: Vec<GithubEmailResponse> = reqwest::Client::new()
            .get("https://api.github.com/user/emails")
            .header("Accept", "application/vnd.github.v3+json")
            .bearer_auth(&token_resp.access_token)
            .send()
            .await
            .map_err(AppError::Http)?
            .json()
            .await
            .map_err(|_| AppError::Internal("failed to parse GitHub emails".to_string()))?;

        emails
            .into_iter()
            .find(|e| e.primary && e.verified)
            .map(|e| e.email)
    };

    Ok((
        user_resp.id.to_string(),
        email,
        user_resp.name.or(Some(user_resp.login)),
        user_resp.avatar_url,
    ))
}

/// Generate a random PKCE code verifier (43-128 chars, URL-safe base64).
fn generate_code_verifier() -> String {
    use rand::RngCore;
    let mut bytes = [0u8; 64];
    rand::thread_rng().fill_bytes(&mut bytes);
    base64::Engine::encode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, &bytes)
}

/// Generate a PKCE code challenge from a verifier using SHA-256.
fn generate_code_challenge(verifier: &str) -> String {
    let digest = Sha256::digest(verifier.as_bytes());
    base64::Engine::encode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, &digest)
}

/// Sanitize a username: lowercase, alphanumeric + underscore/hyphen only, max 50 chars.
fn sanitize_username(input: &str) -> String {
    let base: String = input
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
        .take(50)
        .collect();
    if base.len() < 3 {
        "user".to_string()
    } else {
        base.to_lowercase()
    }
}

/// Ensure username is unique in DB, append a number if needed.
async fn ensure_unique_username(pool: &sqlx::PgPool, base: &str) -> AppResult<String> {
    let mut username = base.to_string();
    let mut counter = 0;

    loop {
        let exists: Option<(i64,)> =
            sqlx::query_as("SELECT 1 FROM users WHERE username = $1")
                .bind(&username)
                .fetch_optional(pool)
                .await
                .map_err(AppError::from)?;

        if exists.is_none() {
            return Ok(username);
        }

        counter += 1;
        if counter > 100 {
            return Err(AppError::Internal(
                "could not generate unique username".to_string(),
            ));
        }
        username = format!("{}_{}", base, counter);
    }
}

// ---------------------------------------------------------------------------
// Re-exports
// ---------------------------------------------------------------------------

pub use auth::{Claims, JwtService, RefreshClaims, TokenType};
