//! Unified error types for the Creative AI Studio backend.

use thiserror::Error;

/// Application-level result type alias.
pub type AppResult<T> = Result<T, AppError>;

/// Unified application error type.
///
/// This enum encompasses all possible error conditions across the backend
/// including database errors, authentication failures, storage issues,
/// generation errors, billing problems, and validation errors.
#[derive(Debug, Error)]
pub enum AppError {
    // Database errors
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),

    // Redis/DragonflyDB errors
    #[error("cache error: {0}")]
    Cache(#[from] redis::RedisError),

    // HTTP/Network errors
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),

    // Axum/HTTP errors
    #[error("http error: {0}")]
    Axum(#[from] axum::http::Error),

    // JSON serialization errors
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    // YAML serialization errors
    #[error("yaml error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    // Authentication errors
    #[error("unauthorized: {0}")]
    Unauthorized(String),

    // JWT errors
    #[error("jwt error: {0}")]
    Jwt(#[from] jsonwebtoken::errors::Error),

    // Password hashing errors
    #[error("password error: {0}")]
    Password(String),

    // bcrypt errors
    #[error("bcrypt error: {0}")]
    Bcrypt(String),

    // Validation errors
    #[error("validation error: {0}")]
    Validation(String),

    // Storage / S3 errors
    #[error("storage error: {0}")]
    Storage(String),

    // AWS SDK errors
    #[error("aws error: {0}")]
    Aws(String),

    // Image processing errors
    #[error("image error: {0}")]
    Image(#[from] image::ImageError),

    // Base64 decoding errors
    #[error("base64 error: {0}")]
    Base64(#[from] base64::DecodeError),

    // Stripe / billing errors
    #[error("billing error: {0}")]
    Billing(String),

    // Generation errors
    #[error("generation error: {0}")]
    Generation(String),

    // Rate limiting errors
    #[error("rate limit exceeded: {0}")]
    RateLimit(String),

    // Not found errors
    #[error("not found: {0}")]
    NotFound(String),

    // Conflict errors
    #[error("conflict: {0}")]
    Conflict(String),

    // Configuration errors
    #[error("config error: {0}")]
    Config(String),

    // IO errors
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    // Internal server errors
    #[error("internal error: {0}")]
    Internal(String),

    // Timeout errors
    #[error("timeout: {0}")]
    Timeout(String),
}

impl AppError {
    /// Returns the HTTP status code for this error.
    pub fn status_code(&self) -> axum::http::StatusCode {
        use axum::http::StatusCode;
        match self {
            AppError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            AppError::Validation(_) => StatusCode::BAD_REQUEST,
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::Conflict(_) => StatusCode::CONFLICT,
            AppError::RateLimit(_) => StatusCode::TOO_MANY_REQUESTS,
            AppError::Database(sqlx::Error::RowNotFound) => StatusCode::NOT_FOUND,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl axum::response::IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        use axum::Json;

        let status = self.status_code();
        let body = serde_json::json!({
            "error": self.to_string(),
            "code": status.as_u16()
        });

        (status, Json(body)).into_response()
    }
}
