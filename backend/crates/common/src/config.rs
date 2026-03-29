//! Application configuration loaded from environment variables.
//!
//! Supports loading from environment variables with sensible defaults.

use once_cell::sync::Lazy;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::env;

/// Global application configuration.
pub static APP_CONFIG: Lazy<RwLock<AppConfig>> = Lazy::new(|| {
    let config = AppConfig::from_env();
    RwLock::new(config)
});

/// Load config from environment, panicking on missing required values.
pub fn load_config() -> AppConfig {
    APP_CONFIG.read().clone()
}

/// Parse a comma-separated list of origins into a vector of strings.
fn parse_cors_origins(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Application configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    // Shared
    pub database_url: String,
    pub redis_url: String,
    pub s3_endpoint: String,
    pub s3_access_key: String,
    pub s3_secret_key: String,
    pub s3_bucket: String,
    pub s3_region: String,
    pub jwt_secret: String,

    // Backend-only
    pub stripe_secret_key: Option<String>,
    pub stripe_webhook_secret: Option<String>,
    pub google_client_id: Option<String>,
    pub google_client_secret: Option<String>,
    pub github_client_id: Option<String>,
    pub github_client_secret: Option<String>,
    pub frontend_url: String,
    pub backend_url: String,
    pub cors_origins: Vec<String>,

    // Worker-only
    pub openai_api_key: Option<String>,
    pub gemini_api_key: Option<String>,
    pub anthropic_api_key: Option<String>,

    // Runtime
    pub log_level: String,
    pub environment: String,
}

impl AppConfig {
    /// Load configuration from environment variables.
    ///
    /// Required for all: DATABASE_URL, REDIS_URL, S3_ENDPOINT, S3_ACCESS_KEY,
    /// S3_SECRET_KEY, S3_BUCKET, JWT_SECRET
    ///
    /// Backend-only: STRIPE_SECRET_KEY, GOOGLE_CLIENT_ID, GOOGLE_CLIENT_SECRET,
    /// GITHUB_CLIENT_ID, GITHUB_CLIENT_SECRET, FRONTEND_URL, BACKEND_URL, CORS_ORIGINS
    ///
    /// Worker-only: OPENAI_API_KEY, GEMINI_API_KEY, ANTHROPIC_API_KEY
    pub fn from_env() -> Self {
        dotenvy::dotenv().ok();

        let environment = env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string());
        let log_level = env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string());

        let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        let redis_url = env::var("REDIS_URL").expect("REDIS_URL must be set");
        let s3_endpoint = env::var("S3_ENDPOINT").expect("S3_ENDPOINT must be set");
        let s3_access_key = env::var("S3_ACCESS_KEY").expect("S3_ACCESS_KEY must be set");
        let s3_secret_key = env::var("S3_SECRET_KEY").expect("S3_SECRET_KEY must be set");
        let s3_bucket = env::var("S3_BUCKET").expect("S3_BUCKET must be set");
        let s3_region = env::var("S3_REGION").unwrap_or_else(|_| "us-east-1".to_string());
        let jwt_secret = env::var("JWT_SECRET").expect("JWT_SECRET must be set");

        let frontend_url = env::var("FRONTEND_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());
        let backend_url = env::var("BACKEND_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());

        let cors_origins_str = env::var("CORS_ORIGINS").unwrap_or_else(|_| {
            if environment == "development" {
                "http://localhost:3000,http://localhost:3001".to_string()
            } else {
                frontend_url.clone()
            }
        });
        let cors_origins = parse_cors_origins(&cors_origins_str);

        let stripe_secret_key = env::var("STRIPE_SECRET_KEY").ok();
        let stripe_webhook_secret = env::var("STRIPE_WEBHOOK_SECRET").ok();
        let google_client_id = env::var("GOOGLE_CLIENT_ID").ok();
        let google_client_secret = env::var("GOOGLE_CLIENT_SECRET").ok();
        let github_client_id = env::var("GITHUB_CLIENT_ID").ok();
        let github_client_secret = env::var("GITHUB_CLIENT_SECRET").ok();

        let openai_api_key = env::var("OPENAI_API_KEY").ok();
        let gemini_api_key = env::var("GEMINI_API_KEY").ok();
        let anthropic_api_key = env::var("ANTHROPIC_API_KEY").ok();

        Self {
            database_url,
            redis_url,
            s3_endpoint,
            s3_access_key,
            s3_secret_key,
            s3_bucket,
            s3_region,
            jwt_secret,
            stripe_secret_key,
            stripe_webhook_secret,
            google_client_id,
            google_client_secret,
            github_client_id,
            github_client_secret,
            frontend_url,
            backend_url,
            cors_origins,
            openai_api_key,
            gemini_api_key,
            anthropic_api_key,
            log_level,
            environment,
        }
    }

    /// Check if running in development mode.
    pub fn is_development(&self) -> bool {
        self.environment == "development"
    }

    /// Check if running in production mode.
    pub fn is_production(&self) -> bool {
        self.environment == "production"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_cors_origins() {
        let result = parse_cors_origins("http://localhost:3000, http://localhost:3001, ");
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], "http://localhost:3000");
        assert_eq!(result[1], "http://localhost:3001");
    }

    #[test]
    fn test_parse_cors_origins_empty() {
        let result = parse_cors_origins("");
        assert!(result.is_empty());
    }
}
