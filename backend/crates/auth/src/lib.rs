//! # Auth Crate
//!
//! Authentication and authorization utilities including JWT handling,
//! password hashing, OAuth flows, and session management.

use common::{AppError, AppResult};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// JWT configuration for token generation and validation.
#[derive(Clone)]
pub struct JwtAuth {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    validation: Validation,
}

impl JwtAuth {
    /// Create a new JwtAuth instance from a secret string.
    pub fn new(secret: &str) -> Self {
        let encoding_key = EncodingKey::from_secret(secret.as_bytes());
        let decoding_key = DecodingKey::from_secret(secret.as_bytes());
        let validation = Validation::default();

        Self {
            encoding_key,
            decoding_key,
            validation,
        }
    }

    /// Generate a JWT token for a user.
    pub fn generate_token(&self, user_id: &str, email: &str, role: &str) -> AppResult<String> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_secs() as i64;

        let claims = JwtClaims {
            sub: user_id.to_string(),
            email: email.to_string(),
            role: role.to_string(),
            exp: now + 86400 * 7, // 7 days
            iat: now,
        };

        encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(AppError::Jwt)
    }

    /// Validate and decode a JWT token.
    pub fn validate_token(&self, token: &str) -> AppResult<JwtClaims> {
        let token_data = decode::<JwtClaims>(token, &self.decoding_key, &self.validation)
            .map_err(AppError::Jwt)?;
        Ok(token_data.claims)
    }
}

/// JWT claims structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtClaims {
    pub sub: String,
    pub email: String,
    pub role: String,
    pub exp: i64,
    pub iat: i64,
}

/// Password hashing utilities using argon2.
pub mod password {
    use argon2::{
        password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
        Argon2,
    };
    use common::AppResult;

    /// Hash a password using argon2.
    pub fn hash(password: &str) -> AppResult<String> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| common::AppError::Password(e.to_string()))?;
        Ok(hash.to_string())
    }

    /// Verify a password against a hash.
    pub fn verify(password: &str, hash: &str) -> AppResult<bool> {
        let parsed_hash = PasswordHash::new(hash)
            .map_err(|e| common::AppError::Password(e.to_string()))?;
        let argon2 = Argon2::default();
        Ok(argon2
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok())
    }
}
