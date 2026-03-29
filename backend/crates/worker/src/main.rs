//! # Worker - Background Job Processor
//!
//! Background worker process that handles async tasks like image generation,
//! email sending, and other long-running operations.
//!
//! Usage:
//!   - Without `--worker` flag: runs as API server (app mode)
//!   - With `--worker` flag: runs as background worker (worker mode)

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
async fn run_worker(config: AppConfig) -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting Creative AI Studio Worker process");
    info!("Environment: {}", config.environment);
    info!(
        "API keys configured - OpenAI: {}, Gemini: {}, Anthropic: {}",
        config.openai_api_key.is_some(),
        config.gemini_api_key.is_some(),
        config.anthropic_api_key.is_some()
    );

    // Worker loop - process jobs from the queue
    // This is a placeholder that will be implemented with the actual job processing logic
    info!("Worker is ready and listening for jobs");

    // For now, just keep the worker running
    // In production, this would connect to Redis/DragonflyDB for job queues
    tokio::signal::ctrl_c().await?;

    info!("Worker shutting down gracefully");
    Ok(())
}

/// Run the API server mode (fallback when not using dedicated app binary).
async fn run_api(config: AppConfig) -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting Creative AI Studio API server (worker mode)");
    info!("Environment: {}", config.environment);
    info!("{} CORS origins configured", config.cors_origins.len());

    use axum::{
        http::{Method, StatusCode},
        response::IntoResponse,
        routing::get,
        Router,
    };
    use tower_http::{cors::CorsLayer, trace::TraceLayer};
    use std::net::SocketAddr;

    #[derive(Clone)]
    struct ApiState;

    async fn health() -> impl IntoResponse {
        (StatusCode::OK, "healthy")
    }

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

    let addr: SocketAddr = format!("{}:{}", host, port).parse().expect("Invalid socket address");
    info!("API server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

/// Detect if --worker flag was passed.
fn is_worker_mode() -> bool {
    env::args().any(|arg| arg == "--worker" || arg == "-w")
}

/// Main entry point.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration from environment
    let config = AppConfig::from_env();

    // Initialize tracing
    init_tracing(&config);

    if is_worker_mode() {
        info!("Mode: BACKGROUND WORKER");
        run_worker(config).await?;
    } else {
        info!("Mode: API SERVER");
        run_api(config).await?;
    }

    Ok(())
}
