//! # Generation Crate
//!
//! AI image generation engine supporting multiple providers (OpenAI, Gemini, Anthropic).
//! Handles prompt processing, model routing, and result handling.

pub mod enhance;
pub mod providers;
pub mod queue;
pub mod worker;

use common::AppResult;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Generation provider type.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum GenerationProvider {
    Openai,
    Gemini,
    Anthropic,
    Internal,
}

/// Generation request input.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationInput {
    pub prompt: String,
    pub negative_prompt: Option<String>,
    pub width: u32,
    pub height: u32,
    pub model: Option<String>,
    pub seed: Option<i64>,
    pub steps: Option<u32>,
    pub guidance_scale: Option<f32>,
}

/// Generation output with image data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationOutput {
    pub image_data: Vec<u8>,
    pub image_format: ImageFormat,
    pub width: u32,
    pub height: u32,
    pub seed: i64,
    pub provider: GenerationProvider,
    pub metadata: GenerationMetadata,
}

/// Supported image formats.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ImageFormat {
    Png,
    Jpeg,
    Webp,
}

/// Generation metadata from the provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationMetadata {
    pub model_id: String,
    pub inference_time_ms: u64,
    pub prompt_tokens: Option<u32>,
    pub extra: serde_json::Value,
}

/// Generation service trait for pluggable providers.
#[async_trait::async_trait]
pub trait GenerationService: Send + Sync {
    /// Generate an image from the given input.
    async fn generate(&self, input: GenerationInput) -> AppResult<GenerationOutput>;

    /// Get the provider name.
    fn provider(&self) -> GenerationProvider;

    /// Check if this provider is available (API key set).
    fn is_available(&self) -> bool;
}

/// Generation router that routes requests to the appropriate provider.
pub struct GenerationRouter {
    services: Vec<Arc<dyn GenerationService>>,
}

impl GenerationRouter {
    /// Create a new generation router.
    pub fn new() -> Self {
        Self { services: Vec::new() }
    }

    /// Add a generation service.
    pub fn add_service(&mut self, service: Arc<dyn GenerationService>) {
        self.services.push(service);
    }

    /// Generate using the best available provider.
    pub async fn generate(&self, input: GenerationInput) -> AppResult<GenerationOutput> {
        let provider = input.model.as_deref().unwrap_or("openai");

        let service = self.services.iter().find(|s| {
            let p = format!("{:?}", s.provider()).to_lowercase();
            p.contains(&provider.to_lowercase())
        }).ok_or_else(|| {
            common::AppError::Generation(format!("No available generation provider for: {}", provider))
        })?;

        service.generate(input).await
    }
}

impl Default for GenerationRouter {
    fn default() -> Self {
        Self::new()
    }
}
