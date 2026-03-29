//! JWT token service for generating and verifying access and refresh tokens.

use chrono::{Duration, Utc};
use common::{AppError, AppResult};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Access token expiry duration: 15 minutes.
const ACCESS_TOKEN_EXPIRY_MINUTES: i64 = 15;

/// Refresh token expiry duration: 7 days.
const REFRESH_TOKEN_EXPIRY_DAYS: i64 = 7;

/// JWT service for generating and verifying JWT tokens.
#[derive(Clone)]
pub struct JwtService {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    validation: Validation,
    issuer: String,
    audience: String,
}

impl JwtService {
    /// Create a new JwtService from a secret string.
    ///
    /// `issuer` is the token issuer (e.g., "creative-ai-studio").
    /// `audience` is the intended recipient (e.g., "creative-ai-studio-api").
    pub fn new(secret: &str, issuer: &str, audience: &str) -> Self {
        let encoding_key = EncodingKey::from_secret(secret.as_bytes());
        let decoding_key = DecodingKey::from_secret(secret.as_bytes());

        let mut validation = Validation::default();
        validation.set_issuer(&[issuer]);
        validation.set_audience(&[audience]);

        Self {
            encoding_key,
            decoding_key,
            validation,
            issuer: issuer.to_string(),
            audience: audience.to_string(),
        }
    }

    /// Generate a new access token for the given user.
    ///
    /// The token contains `user_id`, `username`, `role`, and a unique `jti`.
    /// Expires after 15 minutes.
    pub fn generate_token(
        &self,
        user_id: &str,
        username: &str,
        role: &str,
    ) -> AppResult<String> {
        let now = Utc::now();
        let jti = Uuid::new_v4().to_string();

        let claims = Claims {
            sub: user_id.to_string(),
            username: username.to_string(),
            role: role.to_string(),
            jti: jti.clone(),
            exp: (now + Duration::minutes(ACCESS_TOKEN_EXPIRY_MINUTES)).timestamp(),
            iat: now.timestamp(),
            token_type: TokenType::Access,
            iss: self.issuer.clone(),
            aud: self.audience.clone(),
        };

        let header = Header::new(jsonwebtoken::Algorithm::HS256);
        encode(&header, &claims, &self.encoding_key)
            .map_err(AppError::Jwt)
    }

    /// Verify and decode an access token.
    ///
    /// Returns the claims if valid, or an error if the token is invalid or expired.
    pub fn verify_token(&self, token: &str) -> AppResult<Claims> {
        let token_data = decode::<Claims>(token, &self.decoding_key, &self.validation)
            .map_err(AppError::Jwt)?;

        let claims = token_data.claims;

        if claims.token_type != TokenType::Access {
            return Err(AppError::Unauthorized(
                "invalid token type: expected access token".to_string(),
            ));
        }

        Ok(claims)
    }

    /// Generate a new refresh token for the given user.
    ///
    /// The token contains `user_id` and a unique `jti`.
    /// Expires after 7 days.
    pub fn generate_refresh_token(&self, user_id: &str) -> AppResult<String> {
        let now = Utc::now();
        let jti = Uuid::new_v4().to_string();

        let claims = RefreshClaims {
            sub: user_id.to_string(),
            jti,
            exp: (now + Duration::days(REFRESH_TOKEN_EXPIRY_DAYS)).timestamp(),
            iat: now.timestamp(),
            token_type: TokenType::Refresh,
            iss: self.issuer.clone(),
            aud: self.audience.clone(),
        };

        let header = Header::new(jsonwebtoken::Algorithm::HS256);
        encode(&header, &claims, &self.encoding_key)
            .map_err(AppError::Jwt)
    }

    /// Verify and decode a refresh token.
    ///
    /// Returns the refresh claims if valid, or an error if the token is invalid or expired.
    pub fn verify_refresh_token(&self, token: &str) -> AppResult<RefreshClaims> {
        let token_data = decode::<RefreshClaims>(token, &self.decoding_key, &self.validation)
            .map_err(AppError::Jwt)?;

        let claims = token_data.claims;

        if claims.token_type != TokenType::Refresh {
            return Err(AppError::Unauthorized(
                "invalid token type: expected refresh token".to_string(),
            ));
        }

        Ok(claims)
    }
}

/// Token type discriminator used in JWT claims.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TokenType {
    Access,
    Refresh,
}

/// JWT claims for access tokens.
///
/// Contains user identity (`sub`, `username`, `role`) plus a unique
/// token identifier (`jti`) and standard timestamp fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Subject: the user's ID.
    pub sub: String,
    /// The user's username.
    pub username: String,
    /// The user's role.
    pub role: String,
    /// JWT ID: a unique identifier for this token (used for revocation).
    pub jti: String,
    /// Expiration time (Unix timestamp).
    pub exp: i64,
    /// Issued-at time (Unix timestamp).
    pub iat: i64,
    /// Token type discriminator.
    pub token_type: TokenType,
    /// Issuer claim: identifies the principal that issued the JWT.
    pub iss: String,
    /// Audience claim: identifies the recipients that the JWT is intended for.
    pub aud: String,
}

/// JWT claims for refresh tokens.
///
/// Contains the user ID and a unique token identifier. Simpler than
/// access token claims since refresh tokens are only used for issuing
/// new access tokens.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshClaims {
    /// Subject: the user's ID.
    pub sub: String,
    /// JWT ID: a unique identifier for this token (used for revocation).
    pub jti: String,
    /// Expiration time (Unix timestamp).
    pub exp: i64,
    /// Issued-at time (Unix timestamp).
    pub iat: i64,
    /// Token type discriminator.
    pub token_type: TokenType,
    /// Issuer claim.
    pub iss: String,
    /// Audience claim.
    pub aud: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_and_verify_access_token() {
        let service = JwtService::new("test-secret-key-12345", "test-issuer", "test-audience");
        let token = service
            .generate_token("user-123", "testuser", "user")
            .expect("token generation should succeed");

        let claims = service
            .verify_token(&token)
            .expect("token verification should succeed");

        assert_eq!(claims.sub, "user-123");
        assert_eq!(claims.username, "testuser");
        assert_eq!(claims.role, "user");
        assert!(!claims.jti.is_empty());
    }

    #[test]
    fn test_generate_and_verify_refresh_token() {
        let service = JwtService::new("test-secret-key-12345", "test-issuer", "test-audience");
        let token = service
            .generate_refresh_token("user-456")
            .expect("refresh token generation should succeed");

        let claims = service
            .verify_refresh_token(&token)
            .expect("refresh token verification should succeed");

        assert_eq!(claims.sub, "user-456");
        assert!(!claims.jti.is_empty());
    }

    #[test]
    fn test_wrong_token_type_fails() {
        let service = JwtService::new("test-secret-key-12345", "test-issuer", "test-audience");

        let access_token = service
            .generate_token("user-123", "testuser", "user")
            .expect("access token generation should succeed");

        let result = service.verify_refresh_token(&access_token);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_token_fails() {
        let service = JwtService::new("test-secret-key-12345", "test-issuer", "test-audience");
        let result = service.verify_token("not.a.valid.token");
        assert!(result.is_err());
    }

    #[test]
    fn test_different_secret_fails() {
        let service1 = JwtService::new("secret-one", "test-issuer", "test-audience");
        let service2 = JwtService::new("secret-two", "test-issuer", "test-audience");

        let token = service1
            .generate_token("user-123", "testuser", "user")
            .expect("token generation should succeed");

        let result = service2.verify_token(&token);
        assert!(result.is_err());
    }

    #[test]
    fn test_iss_aud_claims() {
        let service = JwtService::new("secret", "creative-ai-studio", "creative-ai-studio-api");
        let token = service
            .generate_token("user-123", "testuser", "user")
            .expect("token generation should succeed");

        let claims = service
            .verify_token(&token)
            .expect("token verification should succeed");

        assert_eq!(claims.iss, "creative-ai-studio");
        assert_eq!(claims.aud, "creative-ai-studio-api");
    }
}
