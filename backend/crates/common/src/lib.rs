//! # Common Library
//!
//! Shared utilities, error types, and configuration for the Creative AI Studio backend.

pub mod config;
pub mod error;

pub use config::AppConfig;
pub use error::{AppError, AppResult};
