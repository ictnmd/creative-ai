//! # Worker - Background Job Processor
//!
//! Background worker process that handles async tasks like image generation,
//! email sending, and other long-running operations.
//!
//! Usage:
//!   - With `--worker` flag (or `-w`): runs as background worker (default in production)
//!   - Without `--worker` flag: runs as API server (for local development only)

use common::AppConfig;
use std::env;
use tracing::info;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

/// Initialize tracing/logging for the worker process.
fn init_tracing(config: &AppConfig) {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(&config.log_level));

    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(env_filter)
        .init();
}

/// Run the background worker process.
async fn run_worker(config: AppConfig) -> anyhow::Result<()> {
    info!("Starting Creative AI Studio Worker process");
    info!("Environment: {}", config.environment);
    info!(
        "API keys configured - OpenAI: {}, Gemini: {}, Anthropic: {}",
        config.openai_api_key.is_some(),
        config.gemini_api_key.is_some(),
        config.anthropic_api_key.is_some()
    );

    // Worker loop - process jobs from the queue
    // TODO(Plan 03): Implement actual job processing with DragonflyDB queue
    info!("Worker is ready and listening for jobs");

    // Keep the worker running until interrupted
    tokio::signal::ctrl_c().await?;

    info!("Worker shutting down gracefully");
    Ok(())
}

/// Detect if --worker flag was passed.
fn is_worker_mode() -> bool {
    env::args().any(|arg| arg == "--worker" || arg == "-w")
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = AppConfig::from_env();
    init_tracing(&config);

    if is_worker_mode() {
        info!("Mode: BACKGROUND WORKER");
        run_worker(config).await?;
    } else {
        info!("Mode: API SERVER (development only)");
        info!("For production, use: cargo run --bin app");
        // In development, allow running API server from worker binary
        // This enables `cargo run --bin worker` without --worker for quick testing
        // Production should always use the app binary
        run_api_server(config).await?;
    }

    Ok(())
}

/// Run the API server (for development convenience only).
async fn run_api_server(config: AppConfig) -> anyhow::Result<()> {
    use axum::{
        http::Method,
        response::IntoResponse,
        routing::get,
        Router,
    };
    use tower_http::{cors::CorsLayer, trace::TraceLayer};

    #[derive(Clone)]
    struct ApiState;

    async fn health() -> impl IntoResponse {
        (axum::http::StatusCode::OK, "healthy")
    }

    let cors = CorsLayer::new()
        .allow_origin(
            config
                .cors_origins
                .iter()
                .map(|origin| {
                    origin
                        .parse::<axum::http::HeaderValue>()
                        .unwrap_or_else(|_| "http://localhost:5173".parse().unwrap())
                })
                .collect::<Vec<_>>(),
        )
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::PATCH,
            Method::OPTIONS,
        ])
        .allow_credentials(true);

    let app = Router::new()
        .route("/health", get(health))
        .route("/api/v1/health", get(health))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(ApiState);

    let host = std::env::var("BACKEND_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = std::env::var("BACKEND_PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse::<u16>()
        .unwrap_or(8080);

    let addr: std::net::SocketAddr = format!("{}:{}", host, port).parse()?;
    info!("API server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
