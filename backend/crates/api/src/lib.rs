//! # API Crate
//!
//! HTTP API layer built on Axum. Handles incoming HTTP requests,
//! routes them to the appropriate handlers, and returns responses.

pub mod account;
pub mod auth;
pub mod extractors;
mod users;

use auth::AuthState;
use axum::{
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use common::AppConfig;
use parking_lot::RwLock;
use redis::aio::ConnectionManager;
use serde::Serialize;
use sqlx::PgPool;
use std::sync::Arc;

/// Application state shared across routes.
#[derive(Clone)]
pub struct ApiState {
    pub inner: Arc<ApiStateInner>,
}

pub struct ApiStateInner {
    pub pool: PgPool,
    pub redis: Arc<RwLock<ConnectionManager>>,
    pub config: AppConfig,
}

/// Health check response.
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
}

/// Create the API router with all routes.
pub fn create_router(state: ApiState) -> Router {
    let users_state = users::UsersState {
        pool: state.inner.pool.clone(),
    };

    let auth_state = AuthState::new(state.clone());

    Router::new()
        .route("/health", get(health_handler))
        .route("/api/v1/health", get(health_handler))
        .merge(users::create_users_router(users_state))
        .merge(auth::create_auth_router(auth_state.clone()))
        .merge(account::routes(auth_state))
        .with_state(state)
}

/// Health check endpoint.
async fn health_handler() -> impl IntoResponse {
    (StatusCode::OK, Json(HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    }))
}
