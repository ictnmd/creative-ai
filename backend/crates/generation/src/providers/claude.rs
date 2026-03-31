//! # Anthropic Claude Provider
//!
//! Placeholder implementation. Anthropic does not provide a public
//! image generation API as of 2025. This module exists for future
//! expansion if Anthropic adds image generation capabilities.

use super::{GenerationRequest, GenerationResponse, ImageProvider, ImageProviderError};
use async_trait::async_trait;

/// Anthropic Claude provider stub.
///
/// Anthropic does not currently support image generation via API.
/// This provider always returns an error indicating unavailability.
#[derive(Clone)]
pub struct ClaudeProvider;

impl ClaudeProvider {
    pub fn new(_base_url: Option<String>) -> Self {
        Self
    }
}

impl Default for ClaudeProvider {
    fn default() -> Self {
        Self::new(None)
    }
}

#[async_trait]
impl ImageProvider for ClaudeProvider {
    fn name(&self) -> &'static str {
        "anthropic"
    }

    fn supported_models(&self) -> Vec<&'static str> {
        vec!["claude"]
    }

    fn cost_per_image(&self, _model: &str) -> u32 {
        0
    }

    fn is_available(&self) -> bool {
        false
    }

    async fn generate(
        &self,
        _api_key: &str,
        _request: &GenerationRequest,
    ) -> Result<GenerationResponse, ImageProviderError> {
        Err(ImageProviderError::NotAvailable(
            "Anthropic does not support image generation via API".to_string(),
        ))
    }
}
