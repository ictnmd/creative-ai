//! # App - Main API Server
//!
//! The main HTTP API server built on Axum. Handles incoming HTTP requests,
//! sets up CORS from CORS_ORIGINS env var, and routes to API handlers.

use api::{ApiState, ApiStateInner};
use axum::{
    http::{Method, StatusCode},
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use common::AppConfig;
use db;
use serde::Serialize;
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::info;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

/// Health check response.
#[derive(Serialize)]
struct HealthResponse {
    status: String,
    version: String,
    environment: String,
}

/// Health endpoint handler (served from the root router, outside api::create_router).
async fn health_handler(state: Arc<ApiStateInner>) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        environment: state.config.environment.clone(),
    })
}

/// Root endpoint handler.
async fn root_handler() -> impl IntoResponse {
    (StatusCode::OK, "Creative AI Studio API")
}

/// Build CORS layer from CORS_ORIGINS env var.
fn build_cors_layer(config: &AppConfig) -> CorsLayer {
    use tower_http::cors::AllowHeaders;

    let cors = CorsLayer::new()
        .allow_origin(
            config
                .cors_origins
                .iter()
                .map(|origin| {
                    origin
                        .parse::<axum::http::HeaderValue>()
                        .unwrap_or_else(|_| "http://localhost:3000".parse().unwrap())
                })
                .collect::<Vec<_>>(),
        )
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE, Method::PATCH, Method::OPTIONS])
        .allow_headers(AllowHeaders::any())
        .expose_headers(vec![
            axum::http::header::CONTENT_TYPE,
            axum::http::header::AUTHORIZATION,
        ])
        .allow_credentials(true);

    cors
}

/// Initialize tracing/logging.
fn init_tracing(config: &AppConfig) {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(&config.log_level));

    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(env_filter)
        .init();
}

/// Create the root router with health endpoints, mounted over the full API.
fn create_root_router(state: ApiState, config: &AppConfig) -> Router {
    let cors = build_cors_layer(config);

    Router::new()
        .route("/", get(root_handler))
        .route("/health", get({
            let inner = state.inner.clone();
            move || health_handler(inner)
        }))
        .route("/api/v1/health", get({
            let inner = state.inner.clone();
            move || health_handler(inner)
        }))
        .nest("/api/v1", api::create_router(state.clone()))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
}

/// Run the API server.
#[tokio::main]
async fn run() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration from environment
    let config = AppConfig::from_env();

    // Initialize tracing
    init_tracing(&config);

    info!(
        "Starting Creative AI Studio API server (env: {}, {} origins configured)",
        config.environment,
        config.cors_origins.len()
    );

    // Initialize PostgreSQL pool
    info!("Connecting to PostgreSQL...");
    let pool = db::init_pg_pool(&config.database_url).await?;
    info!("PostgreSQL pool initialized");

    // Initialize DragonflyDB / Redis
    info!("Connecting to DragonflyDB...");
    db::init_redis(&config.redis_url).await?;
    let redis = db::redis_manager()?;
    info!("DragonflyDB connection manager initialized");

    // Create ApiState
    let state = ApiState {
        inner: Arc::new(ApiStateInner {
            pool,
            redis,
            config: config.clone(),
        }),
    };

    // Create the router
    let app = create_root_router(state, &config);

    // Determine bind address
    let host = std::env::var("BACKEND_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = std::env::var("BACKEND_PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse::<u16>()
        .unwrap_or(8080);

    let addr = format!("{}:{}", host, port);
    let socket_addr: SocketAddr = addr.parse().expect("Invalid socket address");

    info!("API server listening on {}", socket_addr);

    let listener = tokio::net::TcpListener::bind(socket_addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("Server error: {}", e);
        std::process::exit(1);
    }
}
