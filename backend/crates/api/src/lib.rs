//! # API Crate
//!
//! HTTP API layer built on Axum. Handles incoming HTTP requests,
//! routes them to the appropriate handlers, and returns responses.

pub mod account;
pub mod admin;
pub mod auth;
pub mod billing;
pub mod extractors;
pub mod generations;
pub mod presets;
pub mod reference;
pub mod sharing;
pub mod social;
mod users;
pub mod ws_generations;


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
    let users = users::create_users_router(state.clone());
    let auth = auth::create_auth_router(state.clone());
    let account = account::routes(state.clone());
    let generations = generations::create_generations_router(state.clone());
    let billing = billing::create_billing_router(state.clone());
    let presets = presets::create_presets_router(state.clone());
    let reference = reference::create_reference_router(state.clone());
    let social = social::create_social_router(state.clone());
    let admin = admin::create_admin_router(state.clone());
    let sharing = sharing::create_sharing_router(state.clone());
    let ws_generations = ws_generations::ws_router(state);

    Router::new()
        .route("/health", get(health_handler))
        .route("/api/v1/health", get(health_handler))
        .merge(users)
        .merge(auth)
        .merge(account)
        .merge(generations)
        .merge(billing)
        .merge(presets)
        .merge(reference)
        .merge(social)
        .merge(admin)
        .merge(sharing)
        .merge(ws_generations)
}

/// Health check endpoint.
async fn health_handler() -> impl IntoResponse {
    (StatusCode::OK, Json(HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    }))
}
