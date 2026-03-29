//! # Provider Router
//!
//! Routes model names to the appropriate image generation provider.

use super::{ImageProvider, ImageProviderError};
use std::collections::HashMap;

/// Routes model identifiers to their corresponding providers.
pub struct ProviderRouter {
    /// Map from model name to provider instance.
    providers: HashMap<String, Box<dyn ImageProvider>>,
}

impl ProviderRouter {
    /// Create a new empty router.
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
        }
    }

    /// Register a provider, mapping all its supported models.
    pub fn register<P: ImageProvider + 'static>(&mut self, provider: P) {
        for model in provider.supported_models() {
            self.providers.insert(model.to_string(), Box::new(provider));
            break; // Box only takes one, use register_boxed for multiple models
        }
    }

    /// Register a boxed provider for a specific model.
    pub fn register_boxed(&mut self, model: &str, provider: Box<dyn ImageProvider>) {
        self.providers.insert(model.to_string(), provider);
    }

    /// Find the provider for a given model.
    pub fn get(&self, model: &str) -> Option<&dyn ImageProvider> {
        self.providers.get(model).map(|p| p.as_ref())
    }

    /// Get the provider for a model, returning an error if not found.
    pub fn route(&self, model: &str) -> Result<&dyn ImageProvider, ImageProviderError> {
        self.get(model)
            .ok_or_else(|| ImageProviderError::UnsupportedModel(format!(
                "no provider registered for model: {}",
                model
            )))
    }

    /// Check if a model is supported.
    pub fn supports(&self, model: &str) -> bool {
        self.providers.contains_key(model)
    }

    /// List all supported model names.
    pub fn supported_models(&self) -> Vec<&str> {
        self.providers.keys().map(|s| s.as_str()).collect()
    }
}

impl Default for ProviderRouter {
    fn default() -> Self {
        Self::new()
    }
}
