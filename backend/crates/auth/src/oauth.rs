//! OAuth 2.0 service with PKCE support for Google and GitHub providers.
//!
//! Implements the OAuth 2.0 Authorization Code flow with PKCE (RFC 7636)
//! to secure the authentication flow against authorization code interception
//! and redirect_uri substitution attacks.

use std::collections::HashMap;
use std::sync::Arc;

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use common::{AppError, AppResult};
use rand::Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use parking_lot::RwLock;
use url::Url;

/// Supported OAuth providers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    Google,
    GitHub,
}

impl Provider {
    /// Returns the authorization endpoint for this provider.
    fn auth_endpoint(&self) -> &'static str {
        match self {
            Provider::Google => "https://accounts.google.com/o/oauth2/v2/auth",
            Provider::GitHub => "https://github.com/login/oauth/authorize",
        }
    }

    /// Returns the token endpoint for this provider.
    fn token_endpoint(&self) -> &'static str {
        match self {
            Provider::Google => "https://oauth2.googleapis.com/token",
            Provider::GitHub => "https://github.com/login/oauth/access_token",
        }
    }

    /// Returns the userinfo endpoint for this provider.
    fn userinfo_endpoint(&self) -> &'static str {
        match self {
            Provider::Google => "https://www.googleapis.com/oauth2/v3/userinfo",
            Provider::GitHub => "https://api.github.com/user",
        }
    }
}

/// OAuth state store for CSRF protection.
///
/// Stores state values with expiry timestamps in memory.
/// States are automatically cleaned up when they expire.
pub struct OAuthStateStore {
    states: Arc<RwLock<HashMap<String, chrono::DateTime<chrono::Utc>>>>,
}

impl Default for OAuthStateStore {
    fn default() -> Self {
        Self::new()
    }
}

impl OAuthStateStore {
    /// Creates a new empty state store.
    pub fn new() -> Self {
        Self {
            states: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Stores a state value with the current timestamp as expiry.
    ///
    /// The state is valid for 10 minutes.
    pub fn store(&self, state: &str) {
        let expiry = chrono::Utc::now() + chrono::Duration::minutes(10);
        let mut states = self.states.write();
        states.insert(state.to_string(), expiry);
    }

    /// Validates and consumes a state value.
    ///
    /// Returns `true` if the state existed and had not expired.
    /// The state is removed from the store after validation regardless
    /// of the result (one-time use).
    pub fn validate(&self, state: &str) -> bool {
        let mut states = self.states.write();
        if let Some(expiry) = states.remove(state) {
            return chrono::Utc::now() < expiry;
        }
        false
    }

    /// Cleans up expired states from the store.
    pub fn cleanup(&self) {
        let now = chrono::Utc::now();
        let mut states = self.states.write();
        states.retain(|_, expiry| now < *expiry);
    }
}

// ---------------------------------------------------------------------------
// PKCE (RFC 7636)
// ---------------------------------------------------------------------------

/// Minimum length for a PKCE code verifier (per RFC 7636).
const PKCE_MIN_VERIFIER_LEN: usize = 43;

/// Maximum length for a PKCE code verifier (per RFC 7636).
const PKCE_MAX_VERIFIER_LEN: usize = 128;

/// Generates a PKCE code verifier and its corresponding S256 code challenge.
///
/// The code verifier is a cryptographically random string using unreserved
/// characters (ASCII letters, digits, hyphen, period, underscore, tilde)
/// as specified in RFC 7636.
///
/// The code challenge is computed as `BASE64URL(SHA256(code_verifier))`
/// with no padding.
///
/// # Returns
///
/// A tuple of `(code_verifier, code_challenge)`:
///
/// - `code_verifier`: A random string between 43 and 128 characters.
/// - `code_challenge`: The S256 hash of the verifier, base64url-encoded.
pub fn generate_pkce_pair() -> (String, String) {
    let mut rng = rand::thread_rng();

    // RFC 7636 Section 4.1: unreserved characters are ALPHA / DIGIT / "-" / "." / "_" / "~"
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-._~";

    // Generate verifier between 43-128 chars. Using 64 chars gives good entropy.
    let len = rng.gen_range(PKCE_MIN_VERIFIER_LEN..=PKCE_MAX_VERIFIER_LEN);
    let verifier: String = (0..len)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect();

    // Compute SHA256 hash of the verifier
    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    let hash = hasher.finalize();

    // Encode as base64url (no padding) per RFC 7636
    let challenge = URL_SAFE_NO_PAD.encode(&hash);

    (verifier, challenge)
}

// ---------------------------------------------------------------------------
// Authorization URL builders
// ---------------------------------------------------------------------------

/// Google OAuth scopes requested.
const GOOGLE_SCOPES: &str = "openid email profile";

/// GitHub OAuth scopes requested.
const GITHUB_SCOPES: &str = "user:email read:user";

/// Builds the Google OAuth 2.0 authorization URL with PKCE.
///
/// # Arguments
///
/// * `client_id` - The OAuth client ID from Google Developer Console.
/// * `redirect_uri` - The URI Google redirects to after user consent.
/// * `state` - A random state value for CSRF protection.
/// * `code_challenge` - The PKCE code challenge (S256 method).
///
/// # Returns
///
/// The full authorization URL as a string.
pub fn google_auth_url(
    client_id: &str,
    redirect_uri: &str,
    state: &str,
    code_challenge: &str,
) -> String {
    let mut url = Url::parse(Provider::Google.auth_endpoint()).expect("valid Google auth URL");

    url.query_pairs_mut()
        .append_pair("client_id", client_id)
        .append_pair("redirect_uri", redirect_uri)
        .append_pair("response_type", "code")
        .append_pair("scope", GOOGLE_SCOPES)
        .append_pair("state", state)
        .append_pair("code_challenge", code_challenge)
        .append_pair("code_challenge_method", "S256");

    url.to_string()
}

/// Builds the GitHub OAuth 2.0 authorization URL with PKCE.
///
/// # Arguments
///
/// * `client_id` - The OAuth client ID from GitHub App settings.
/// * `redirect_uri` - The URI GitHub redirects to after user consent.
/// * `state` - A random state value for CSRF protection.
/// * `code_challenge` - The PKCE code challenge (S256 method).
///
/// # Returns
///
/// The full authorization URL as a string.
pub fn github_auth_url(
    client_id: &str,
    redirect_uri: &str,
    state: &str,
    code_challenge: &str,
) -> String {
    let mut url = Url::parse(Provider::GitHub.auth_endpoint()).expect("valid GitHub auth URL");

    url.query_pairs_mut()
        .append_pair("client_id", client_id)
        .append_pair("redirect_uri", redirect_uri)
        .append_pair("scope", GITHUB_SCOPES)
        .append_pair("state", state)
        .append_pair("code_challenge", code_challenge)
        .append_pair("code_challenge_method", "S256");

    url.to_string()
}

// ---------------------------------------------------------------------------
// Token exchange
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
#[allow(dead_code)]
struct GoogleTokenResponse {
    access_token: String,
    #[serde(rename = "token_type")]
    _token_type: String,
    _expires_in: Option<u64>,
    _refresh_token: Option<String>,
    #[serde(default)]
    _id_token: Option<String>,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct GitHubTokenResponse {
    access_token: String,
    #[serde(rename = "token_type")]
    _token_type: String,
    #[serde(default)]
    _scope: Option<String>,
}

/// Exchange an authorization code for an access token with Google.
///
/// Uses PKCE. The `code_verifier` must match the one used when building
/// the original authorization URL.
///
/// # Arguments
///
/// * `code` - The authorization code received at the redirect_uri.
/// * `client_id` - The OAuth client ID.
/// * `client_secret` - The OAuth client secret.
/// * `redirect_uri` - The same redirect_uri used in the authorization request.
/// * `code_verifier` - The original PKCE code verifier.
///
/// # Returns
///
/// The access token string on success.
pub async fn exchange_code_google(
    code: &str,
    client_id: &str,
    client_secret: &str,
    redirect_uri: &str,
    code_verifier: &str,
) -> AppResult<String> {
    let params = [
        ("grant_type", "authorization_code"),
        ("code", code),
        ("client_id", client_id),
        ("client_secret", client_secret),
        ("redirect_uri", redirect_uri),
        ("code_verifier", code_verifier),
    ];

    let client = reqwest::Client::new();
    let response = client
        .post(Provider::Google.token_endpoint())
        .form(&params)
        .send()
        .await
        .map_err(|e| AppError::Http(e))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(AppError::Unauthorized(format!(
            "Google token exchange failed ({}): {}",
            status, body
        )));
    }

    let body = response.text().await.map_err(AppError::Http)?;
    let token_resp: GoogleTokenResponse =
        serde_json::from_str(&body).map_err(AppError::Json)?;

    Ok(token_resp.access_token)
}

/// Exchange an authorization code for an access token with GitHub.
///
/// Uses PKCE. GitHub's token endpoint requires `Accept: application/json`
/// header for JSON responses (otherwise it returns form-encoded data).
///
/// # Arguments
///
/// * `code` - The authorization code received at the redirect_uri.
/// * `client_id` - The OAuth client ID.
/// * `client_secret` - The OAuth client secret.
/// * `redirect_uri` - The same redirect_uri used in the authorization request.
/// * `code_verifier` - The original PKCE code verifier.
///
/// # Returns
///
/// The access token string on success.
pub async fn exchange_code_github(
    code: &str,
    client_id: &str,
    client_secret: &str,
    redirect_uri: &str,
    code_verifier: &str,
) -> AppResult<String> {
    let params = [
        ("grant_type", "authorization_code"),
        ("code", code),
        ("client_id", client_id),
        ("client_secret", client_secret),
        ("redirect_uri", redirect_uri),
        ("code_verifier", code_verifier),
    ];

    let client = reqwest::Client::new();
    let response = client
        .post(Provider::GitHub.token_endpoint())
        .header("Accept", "application/json")
        .form(&params)
        .send()
        .await
        .map_err(|e| AppError::Http(e))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(AppError::Unauthorized(format!(
            "GitHub token exchange failed ({}): {}",
            status, body
        )));
    }

    let body = response.text().await.map_err(AppError::Http)?;
    let token_resp: GitHubTokenResponse =
        serde_json::from_str(&body).map_err(AppError::Json)?;

    Ok(token_resp.access_token)
}

// ---------------------------------------------------------------------------
// User info fetching
// ---------------------------------------------------------------------------

/// Google user info response fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleUserInfo {
    /// Google's unique identifier for the user.
    pub sub: String,
    /// The user's email address.
    pub email: String,
    /// The user's display name.
    pub name: String,
    /// URL to the user's profile picture.
    #[serde(default)]
    pub picture: Option<String>,
}

/// GitHub user info response fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubUserInfo {
    /// GitHub's unique identifier for the user.
    pub id: u64,
    /// The user's email address (may be null if not publicly visible).
    #[serde(default)]
    pub email: Option<String>,
    /// The user's display name.
    #[serde(default)]
    pub name: Option<String>,
    /// URL to the user's avatar image.
    #[serde(rename = "avatar_url")]
    pub avatar_url: String,
}

/// Fetches the authenticated user's profile from Google.
///
/// # Arguments
///
/// * `access_token` - A valid Google OAuth access token.
///
/// # Returns
///
/// `GoogleUserInfo` containing the user's profile data.
pub async fn get_google_user(access_token: &str) -> AppResult<GoogleUserInfo> {
    let client = reqwest::Client::new();
    let response = client
        .get(Provider::Google.userinfo_endpoint())
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|e| AppError::Http(e))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(AppError::Unauthorized(format!(
            "Google userinfo failed ({}): {}",
            status, body
        )));
    }

    let body = response.text().await.map_err(AppError::Http)?;
    let user_info: GoogleUserInfo =
        serde_json::from_str(&body).map_err(AppError::Json)?;

    Ok(user_info)
}

/// Fetches the authenticated user's profile from GitHub.
///
/// If the user has no public email, this makes a second request to the
/// GitHub emails endpoint to retrieve any verified private emails.
///
/// # Arguments
///
/// * `access_token` - A valid GitHub OAuth access token.
///
/// # Returns
///
/// `GitHubUserInfo` containing the user's profile data.
pub async fn get_github_user(access_token: &str) -> AppResult<GitHubUserInfo> {
    let client = reqwest::Client::new();

    // Fetch primary user info
    let response = client
        .get(Provider::GitHub.userinfo_endpoint())
        .header("Authorization", format!("Bearer {}", access_token))
        .header("User-Agent", "Creative-AI-Studio")
        .send()
        .await
        .map_err(|e| AppError::Http(e))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(AppError::Unauthorized(format!(
            "GitHub userinfo failed ({}): {}",
            status, body
        )));
    }

    let body = response.text().await.map_err(AppError::Http)?;
    let mut user_info: GitHubUserInfo =
        serde_json::from_str(&body).map_err(AppError::Json)?;

    // If email is missing, try to fetch it from the emails endpoint
    if user_info.email.is_none() {
        if let Some(email) = fetch_github_email(&client, access_token).await {
            user_info.email = Some(email);
        }
    }

    Ok(user_info)
}

#[derive(Deserialize)]
struct GitHubEmail {
    email: String,
    primary: bool,
    verified: bool,
}

async fn fetch_github_email(client: &reqwest::Client, access_token: &str) -> Option<String> {
    let response = client
        .get("https://api.github.com/user/emails")
        .header("Authorization", format!("Bearer {}", access_token))
        .header("User-Agent", "Creative-AI-Studio")
        .send()
        .await
        .ok()?;

    if !response.status().is_success() {
        return None;
    }

    let body = response.text().await.ok()?;
    let emails: Vec<GitHubEmail> = serde_json::from_str(&body).ok()?;

    // Prefer primary verified email
    emails
        .into_iter()
        .find(|e| e.primary && e.verified)
        .map(|e| e.email)
}

// ---------------------------------------------------------------------------
// State management
// ---------------------------------------------------------------------------

/// Generates a cryptographically random state string for CSRF protection.
///
/// The state is a Base64url-encoded (no padding) random 32-byte value.
pub fn generate_state() -> String {
    let mut rng = rand::thread_rng();
    let bytes: [u8; 32] = rng.gen();
    URL_SAFE_NO_PAD.encode(&bytes)
}

/// Validates an OAuth state value using the provided state store.
///
/// Returns `true` if the state is valid and has not expired.
/// The state is consumed after validation (one-time use).
///
/// # Arguments
///
/// * `state` - The state value to validate.
/// * `store` - The state store to check against.
pub fn validate_state(state: &str, store: &OAuthStateStore) -> bool {
    store.validate(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pkce_pair_generation() {
        let (verifier, challenge) = generate_pkce_pair();

        // Verifier length should be between 43 and 128
        assert!(
            verifier.len() >= PKCE_MIN_VERIFIER_LEN && verifier.len() <= PKCE_MAX_VERIFIER_LEN,
            "verifier length {} is outside valid range",
            verifier.len()
        );

        // Verifier should only contain unreserved characters per RFC 7636
        for c in verifier.chars() {
            assert!(
                c.is_ascii_alphanumeric() || c == '-' || c == '.' || c == '_' || c == '~',
                "invalid verifier char: {}",
                c
            );
        }

        // Challenge should be base64url encoded (no padding), 43 chars for SHA256
        assert_eq!(challenge.len(), 43);
        for c in challenge.chars() {
            assert!(
                c.is_ascii_alphanumeric() || c == '-' || c == '_',
                "invalid challenge char: {}",
                c
            );
        }

        // Verify the challenge matches SHA256(verifier)
        let mut hasher = Sha256::new();
        hasher.update(verifier.as_bytes());
        let hash = hasher.finalize();
        let expected = URL_SAFE_NO_PAD.encode(&hash);
        assert_eq!(challenge, expected);
    }

    #[test]
    fn test_pkce_pair_uniqueness() {
        let (v1, c1) = generate_pkce_pair();
        let (v2, c2) = generate_pkce_pair();

        // Verifiers should be different (random)
        assert_ne!(v1, v2);
        // And challenges should also differ
        assert_ne!(c1, c2);
    }

    #[test]
    fn test_google_auth_url_contains_params() {
        let url = google_auth_url(
            "my-client-id",
            "http://localhost:3000/auth/callback",
            "random-state",
            "challenge123",
        );

        assert!(url.contains("client_id=my-client-id"));
        assert!(url.contains("redirect_uri="));
        assert!(url.contains("response_type=code"));
        assert!(url.contains("scope="));
        assert!(url.contains("state=random-state"));
        assert!(url.contains("code_challenge=challenge123"));
        assert!(url.contains("code_challenge_method=S256"));
        assert!(url.contains("accounts.google.com"));
    }

    #[test]
    fn test_github_auth_url_contains_params() {
        let url = github_auth_url(
            "my-client-id",
            "http://localhost:3000/auth/callback",
            "random-state",
            "challenge123",
        );

        assert!(url.contains("client_id=my-client-id"));
        assert!(url.contains("redirect_uri="));
        assert!(url.contains("scope="));
        assert!(url.contains("state=random-state"));
        assert!(url.contains("code_challenge=challenge123"));
        assert!(url.contains("code_challenge_method=S256"));
        assert!(url.contains("github.com"));
    }

    #[test]
    fn test_generate_state_length() {
        let state = generate_state();
        // 32 bytes base64url encoded (no padding) = 43 chars
        assert_eq!(state.len(), 43);
    }

    #[test]
    fn test_generate_state_uniqueness() {
        let s1 = generate_state();
        let s2 = generate_state();
        assert_ne!(s1, s2);
    }

    #[test]
    fn test_oauth_state_store_validate_and_consume() {
        let store = OAuthStateStore::new();

        let state = "test-state-123";
        store.store(state);

        // Valid: should pass
        assert!(store.validate(state));

        // Consumed: should fail on second use
        assert!(!store.validate(state));

        // Unknown state: should fail
        assert!(!store.validate("unknown-state"));
    }

    #[test]
    fn test_oauth_state_store_cleanup() {
        let store = OAuthStateStore::new();

        let state = "expired-state";
        store.store(state);

        store.cleanup();

        // State should still be valid immediately after cleanup
        assert!(store.validate(state));
    }

    #[test]
    fn test_google_user_info_deserialize() {
        let json = r#"{
            "sub": "123456789",
            "email": "user@example.com",
            "name": "Test User",
            "picture": "https://example.com/photo.jpg"
        }"#;

        let user: GoogleUserInfo = serde_json::from_str(json).unwrap();
        assert_eq!(user.sub, "123456789");
        assert_eq!(user.email, "user@example.com");
        assert_eq!(user.name, "Test User");
        assert_eq!(user.picture.as_deref(), Some("https://example.com/photo.jpg"));
    }

    #[test]
    fn test_github_user_info_deserialize() {
        let json = r#"{
            "id": 98765,
            "email": "user@github.com",
            "name": "Test User",
            "avatar_url": "https://avatars.githubusercontent.com/u/98765"
        }"#;

        let user: GitHubUserInfo = serde_json::from_str(json).unwrap();
        assert_eq!(user.id, 98765);
        assert_eq!(user.email.as_deref(), Some("user@github.com"));
        assert_eq!(user.name.as_deref(), Some("Test User"));
        assert_eq!(
            user.avatar_url,
            "https://avatars.githubusercontent.com/u/98765"
        );
    }
}
