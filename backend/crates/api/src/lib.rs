//! # API Crate
//!
//! HTTP API layer built on Axum. Handles incoming HTTP requests,
//! routes them to the appropriate handlers, and returns responses.

use axum::{
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use serde::Serialize;
use std::sync::Arc;

/// Application state shared across routes.
#[derive(Clone)]
pub struct ApiState {
    pub inner: Arc<ApiStateInner>,
}

pub struct ApiStateInner {
    // Add shared state here as crates are implemented
}

/// Health check response.
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
}

/// Create the API router with all routes.
pub fn create_router(state: ApiState) -> Router {
    Router::new()
        .route("/health", get(health_handler))
        .route("/api/v1/health", get(health_handler))
        .with_state(state)
}

/// Health check endpoint.
async fn health_handler() -> impl IntoResponse {
    (StatusCode::OK, Json(HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    }))
}
