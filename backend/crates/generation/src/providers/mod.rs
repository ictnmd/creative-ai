//! # Generation Providers
//!
//! Abstraction layer for multiple AI image generation providers.
//! Each provider implements the `ImageProvider` trait and handles
//! its own API communication, response parsing, and error handling.

mod claude;
mod gemini;
mod openai;
pub mod router;

pub use claude::ClaudeProvider;
pub use gemini::GeminiProvider;
pub use openai::OpenAIProvider;
pub use router::ProviderRouter;

/// Generation request sent to a provider.
#[derive(Debug, Clone)]
pub struct GenerationRequest {
    /// The prompt text for image generation.
    pub prompt: String,
    /// Optional negative prompt to exclude certain elements.
    pub negative_prompt: Option<String>,
    /// Number of images to generate.
    pub num_images: u32,
    /// Aspect ratio of the output image.
    pub aspect_ratio: AspectRatio,
    /// Model identifier (provider-specific).
    pub model: String,
    /// Output format for the image.
    pub format: ImageFormat,
}

/// Supported aspect ratios for image generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AspectRatio {
    Ratio1x1,
    Ratio16x9,
    Ratio9x16,
    Ratio4x3,
    Ratio3x4,
}

impl AspectRatio {
    pub fn as_str(&self) -> &'static str {
        match self {
            AspectRatio::Ratio1x1 => "1:1",
            AspectRatio::Ratio16x9 => "16:9",
            AspectRatio::Ratio9x16 => "9:16",
            AspectRatio::Ratio4x3 => "4:3",
            AspectRatio::Ratio3x4 => "3:4",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "1:1" => Some(AspectRatio::Ratio1x1),
            "16:9" => Some(AspectRatio::Ratio16x9),
            "9:16" => Some(AspectRatio::Ratio9x16),
            "4:3" => Some(AspectRatio::Ratio4x3),
            "3:4" => Some(AspectRatio::Ratio3x4),
            _ => None,
        }
    }

    /// Returns (width, height) in pixels for this aspect ratio.
    pub fn dimensions(&self) -> (u32, u32) {
        match self {
            AspectRatio::Ratio1x1 => (1024, 1024),
            AspectRatio::Ratio16x9 => (1792, 1024),
            AspectRatio::Ratio9x16 => (1024, 1792),
            AspectRatio::Ratio4x3 => (1024, 768),
            AspectRatio::Ratio3x4 => (768, 1024),
        }
    }
}

/// Supported output image formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    Png,
    Jpeg,
    Webp,
}

impl ImageFormat {
    pub fn as_str(&self) -> &'static str {
        match self {
            ImageFormat::Png => "png",
            ImageFormat::Jpeg => "jpeg",
            ImageFormat::Webp => "webp",
        }
    }

    pub fn mime_type(&self) -> &'static str {
        match self {
            ImageFormat::Png => "image/png",
            ImageFormat::Jpeg => "image/jpeg",
            ImageFormat::Webp => "image/webp",
        }
    }
}

/// A generated image output from a provider.
#[derive(Debug, Clone)]
pub struct ImageOutput {
    /// Raw image bytes.
    pub bytes: Vec<u8>,
    /// Original URL from the provider (if available).
    pub original_url: Option<String>,
    /// Revised/enhanced prompt (if provided by the model).
    pub revised_prompt: Option<String>,
    /// Output format.
    pub format: ImageFormat,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
}

/// Response from a generation request.
#[derive(Debug, Clone)]
pub struct GenerationResponse {
    /// Generated images.
    pub images: Vec<ImageOutput>,
    /// Provider-specific model identifier.
    pub model: String,
    /// Credits consumed for this request.
    pub credits_used: u32,
    /// Inference time in milliseconds.
    pub inference_time_ms: u64,
}

/// Errors specific to image generation providers.
#[derive(Debug, Clone)]
pub enum ImageProviderError {
    /// The request failed (network, API error, etc.).
    RequestFailed(String),
    /// The provider does not support this model.
    UnsupportedModel(String),
    /// The provider is not available (no API key, etc.).
    NotAvailable(String),
    /// Invalid request parameters.
    InvalidRequest(String),
    /// Failed to parse the provider response.
    ParseError(String),
    /// Authentication failed.
    AuthenticationFailed(String),
    /// Rate limit exceeded.
    RateLimited(String),
    /// Insufficient credits.
    InsufficientCredits,
}

impl std::fmt::Display for ImageProviderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImageProviderError::RequestFailed(msg) => write!(f, "request failed: {}", msg),
            ImageProviderError::UnsupportedModel(model) => write!(f, "unsupported model: {}", model),
            ImageProviderError::NotAvailable(reason) => write!(f, "provider not available: {}", reason),
            ImageProviderError::InvalidRequest(msg) => write!(f, "invalid request: {}", msg),
            ImageProviderError::ParseError(msg) => write!(f, "parse error: {}", msg),
            ImageProviderError::AuthenticationFailed(msg) => write!(f, "authentication failed: {}", msg),
            ImageProviderError::RateLimited(msg) => write!(f, "rate limited: {}", msg),
            ImageProviderError::InsufficientCredits => write!(f, "insufficient credits"),
        }
    }
}

impl std::error::Error for ImageProviderError {}

impl From<ImageProviderError> for common::AppError {
    fn from(e: ImageProviderError) -> Self {
        common::AppError::Generation(e.to_string())
    }
}

/// Cost per image for each supported model.
pub fn credits_per_image(model: &str) -> u32 {
    match model {
        "dall-e-3" => 4,
        "dall-e-2" => 1,
        "imagen-3" => 3,
        _ => 0,
    }
}

/// Trait for image generation providers.
#[async_trait::async_trait]
pub trait ImageProvider: Send + Sync {
    /// Human-readable provider name.
    fn name(&self) -> &'static str;

    /// List of model identifiers supported by this provider.
    fn supported_models(&self) -> Vec<&'static str>;

    /// Cost in credits per generated image for a given model.
    fn cost_per_image(&self, model: &str) -> u32;

    /// Check if this provider is available (API key configured).
    fn is_available(&self) -> bool;

    /// Generate images from the given request.
    async fn generate(
        &self,
        api_key: &str,
        request: &GenerationRequest,
    ) -> Result<GenerationResponse, ImageProviderError>;
}

/// Download image bytes from a URL.
pub async fn download_image(url: &str) -> Result<Vec<u8>, ImageProviderError> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| ImageProviderError::RequestFailed(e.to_string()))?;

    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| ImageProviderError::RequestFailed(e.to_string()))?;

    if !response.status().is_success() {
        return Err(ImageProviderError::RequestFailed(format!(
            "download failed with status {}",
            response.status()
        )));
    }

    response
        .bytes()
        .await
        .map_err(|e| ImageProviderError::RequestFailed(e.to_string()))
        .map(|b| b.to_vec())
}
