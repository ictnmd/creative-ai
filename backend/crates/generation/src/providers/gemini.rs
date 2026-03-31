//! # Google Gemini Imagen Provider
//!
//! Implementation of the `ImageProvider` trait for Google's Gemini Imagen models.
//! Also implements `GenerationService` to work with the GenerationRouter.

use super::{AspectRatio, GenerationRequest, GenerationResponse, ImageFormat, ImageOutput, ImageProvider, ImageProviderError};
use crate::{GenerationInput, GenerationOutput, GenerationProvider, GenerationService, GenerationMetadata, ImageFormat as CrateImageFormat};
use async_trait::async_trait;
use std::time::Instant;

const BASE_URL: &str = "https://generativelanguage.googleapis.com/v1beta/models";

/// Google Gemini Imagen image generation provider.
#[derive(Clone)]
pub struct GeminiProvider {
    api_key: Option<String>,
}

impl GeminiProvider {
    pub fn new(api_key: Option<String>) -> Self {
        Self { api_key }
    }

    fn gemini_aspect_ratio(&self, ratio: AspectRatio) -> &'static str {
        match ratio {
            AspectRatio::Ratio1x1 => "1:1",
            AspectRatio::Ratio16x9 => "16:9",
            AspectRatio:: Ratio9x16 => "9:16",
            AspectRatio::Ratio4x3 => "4:3",
            AspectRatio::Ratio3x4 => "3:4",
        }
    }
}

#[async_trait]
impl ImageProvider for GeminiProvider {
    fn name(&self) -> &'static str {
        "gemini"
    }

    fn supported_models(&self) -> Vec<&'static str> {
        vec!["imagen-3", "gemini"]
    }

    fn cost_per_image(&self, model: &str) -> u32 {
        match model {
            "imagen-3" => 3,
            "gemini" => 3,
            _ => 0,
        }
    }

    fn is_available(&self) -> bool {
        self.api_key.as_ref().map_or(false, |k| !k.is_empty())
    }

    async fn generate(
        &self,
        api_key: &str,
        request: &GenerationRequest,
    ) -> Result<GenerationResponse, ImageProviderError> {
        let effective_key = if !api_key.is_empty() {
            api_key.to_string()
        } else if let Some(ref k) = self.api_key {
            k.clone()
        } else {
            return Err(ImageProviderError::NotAvailable("no API key".to_string()));
        };

        let model = &request.model;
        if !self.supported_models().contains(&model.as_str()) {
            return Err(ImageProviderError::UnsupportedModel(model.clone()));
        }

        let start = Instant::now();

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(180))
            .build()
            .map_err(|e| ImageProviderError::RequestFailed(e.to_string()))?;

        let aspect_ratio_str = self.gemini_aspect_ratio(request.aspect_ratio);
        let num_images = request.num_images.min(4);

        let request_body = serde_json::json!({
            "prompt": request.prompt,
            "sampleCount": num_images,
            "aspectRatio": aspect_ratio_str,
        });

        let model_name = if model == "imagen-3" {
            "imagen-3.0-generate"
        } else {
            "imagen-3.0-generate"
        };

        let url = format!(
            "{}/{}/samplerInfo?key={}",
            BASE_URL, model_name, effective_key
        );

        let response = client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| ImageProviderError::RequestFailed(e.to_string()))?;

        let status = response.status();
        let body_text = response
            .text()
            .await
            .map_err(|e| ImageProviderError::RequestFailed(e.to_string()))?;

        if !status.is_success() {
            return Err(ImageProviderError::RequestFailed(format!(
                "Gemini API error ({}): {}",
                status, body_text
            )));
        }

        let parsed: serde_json::Value = serde_json::from_str(&body_text)
            .map_err(|e| ImageProviderError::ParseError(format!("failed to parse response: {}", e)))?;

        // Gemini Imagen returns base64-encoded image bytes in predictions[].bytesBase64Encoded
        let predictions = parsed.get("predictions")
            .and_then(|p| p.as_array())
            .ok_or_else(|| ImageProviderError::ParseError("missing predictions array in response".to_string()))?;

        let mut images = Vec::with_capacity(predictions.len());
        let (width, height) = request.aspect_ratio.dimensions();

        for prediction in predictions {
            let b64_data = prediction.get("bytesBase64Encoded")
                .and_then(|b| b.as_str())
                .ok_or_else(|| ImageProviderError::ParseError("missing bytesBase64Encoded in prediction".to_string()))?;

            let image_bytes = base64::Engine::decode(
                &base64::engine::general_purpose::STANDARD,
                b64_data,
            ).map_err(|e| ImageProviderError::ParseError(format!("failed to decode base64: {}", e)))?;

            images.push(ImageOutput {
                bytes: image_bytes,
                original_url: None,
                revised_prompt: None,
                format: ImageFormat::Png,
                width,
                height,
            });
        }

        let credits = self.cost_per_image(model) * images.len() as u32;

        Ok(GenerationResponse {
            images,
            model: model.clone(),
            credits_used: credits,
            inference_time_ms: start.elapsed().as_millis() as u64,
        })
    }
}

#[async_trait::async_trait]
impl GenerationService for GeminiProvider {
    async fn generate(&self, input: GenerationInput) -> common::AppResult<GenerationOutput> {
        let model = input.model.as_deref().unwrap_or("imagen-3");
        let aspect_ratio = aspect_ratio_from_dims(input.width, input.height);

        let req = GenerationRequest {
            prompt: input.prompt.clone(),
            negative_prompt: input.negative_prompt.clone(),
            num_images: 1,
            aspect_ratio,
            model: model.to_string(),
            format: ImageFormat::Png,
        };

        let api_key = self.api_key.as_deref().unwrap_or("");
        let resp = ImageProvider::generate(self, api_key, &req)
            .await
            .map_err(|e| common::AppError::Generation(e.to_string()))?;

        let img = resp.images.into_iter().next().unwrap_or_else(|| ImageOutput {
            bytes: Vec::new(),
            original_url: None,
            revised_prompt: None,
            format: ImageFormat::Png,
            width: input.width,
            height: input.height,
        });

        Ok(GenerationOutput {
            image_data: img.bytes,
            image_format: CrateImageFormat::Png,
            width: img.width,
            height: img.height,
            seed: input.seed.unwrap_or(0),
            provider: GenerationProvider::Gemini,
            metadata: GenerationMetadata {
                model_id: resp.model,
                inference_time_ms: resp.inference_time_ms,
                prompt_tokens: None,
                extra: serde_json::json!({}),
            },
        })
    }

    fn provider(&self) -> GenerationProvider {
        GenerationProvider::Gemini
    }

    fn is_available(&self) -> bool {
        self.api_key.as_ref().map_or(false, |k| !k.is_empty())
    }
}

/// Convert width/height dimensions to an AspectRatio.
fn aspect_ratio_from_dims(width: u32, height: u32) -> AspectRatio {
    match (width, height) {
        (1024, 1024) => AspectRatio::Ratio1x1,
        (1792, 1024) | _ if width > height => AspectRatio::Ratio16x9,
        (1024, 1792) | _ if height > width => AspectRatio:: Ratio9x16,
        (1024, 768) => AspectRatio::Ratio4x3,
        (768, 1024) => AspectRatio::Ratio3x4,
        _ => AspectRatio::Ratio1x1,
    }
}
