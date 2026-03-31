//! # OpenAI DALL-E Provider
//!
//! Implementation of the `ImageProvider` trait for OpenAI's DALL-E models.
//! Also implements `GenerationService` to work with the GenerationRouter.

use super::{download_image, AspectRatio, GenerationRequest, GenerationResponse, ImageFormat, ImageOutput, ImageProvider, ImageProviderError};
use crate::{GenerationInput, GenerationOutput, GenerationProvider, GenerationService, GenerationMetadata, ImageFormat as CrateImageFormat};
use async_trait::async_trait;
use std::time::Instant;

const BASE_URL: &str = "https://api.openai.com/v1/images/generations";

/// OpenAI DALL-E image generation provider.
#[derive(Clone)]
pub struct OpenAIProvider {
    api_key: Option<String>,
}

impl OpenAIProvider {
    pub fn new(api_key: Option<String>) -> Self {
        Self { api_key }
    }

    fn model_size(&self, model: &str, aspect_ratio: AspectRatio) -> Result<String, ImageProviderError> {
        let (w, h) = aspect_ratio.dimensions();
        match model {
            "dall-e-3" => Ok(format!("{}x{}", w, h)),
            "dall-e-2" => {
                // DALL-E 2 supports 256x256, 512x512, or 1024x1024
                if w == 1024 && h == 1024 {
                    Ok("1024x1024".to_string())
                } else if w == 1024 && h > 1024 {
                    Ok("1024x1792".to_string())
                } else if h == 1024 && w > 1024 {
                    Ok("1792x1024".to_string())
                } else {
                    Ok("1024x1024".to_string())
                }
            }
            _ => Err(ImageProviderError::UnsupportedModel(model.to_string())),
        }
    }

    fn model_response_format(&self) -> &'static str {
        "url"
    }
}

#[async_trait]
impl ImageProvider for OpenAIProvider {
    fn name(&self) -> &'static str {
        "openai"
    }

    fn supported_models(&self) -> Vec<&'static str> {
        vec!["dall-e-3", "dall-e-2"]
    }

    fn cost_per_image(&self, model: &str) -> u32 {
        match model {
            "dall-e-3" => 4,
            "dall-e-2" => 1,
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
        if !ImageProvider::is_available(self) && api_key.is_empty() {
            return Err(ImageProviderError::NotAvailable("no API key provided".to_string()));
        }

        let model = &request.model;
        if !self.supported_models().contains(&model.as_str()) {
            return Err(ImageProviderError::UnsupportedModel(model.clone()));
        }

        let size = self.model_size(model, request.aspect_ratio)?;
        let start = Instant::now();

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .map_err(|e| ImageProviderError::RequestFailed(e.to_string()))?;

        let mut body = serde_json::json!({
            "model": model,
            "prompt": request.prompt,
            "n": request.num_images.min(10),
            "size": size,
            "response_format": self.model_response_format(),
        });

        if let Some(neg) = &request.negative_prompt {
            if model == "dall-e-3" {
                body["negative_prompt"] = serde_json::json!(neg);
            }
        }

        let effective_key = if !api_key.is_empty() {
            api_key
        } else if let Some(ref k) = self.api_key {
            k.as_str()
        } else {
            return Err(ImageProviderError::NotAvailable("no API key".to_string()));
        };

        let response = client
            .post(BASE_URL)
            .header("Authorization", format!("Bearer {}", effective_key))
            .header("Content-Type", "application/json")
            .json(&body)
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
                "OpenAI API error ({}): {}",
                status, body_text
            )));
        }

        let parsed: serde_json::Value = serde_json::from_str(&body_text)
            .map_err(|e| ImageProviderError::ParseError(format!("failed to parse response: {}", e)))?;

        let data_array = parsed.get("data")
            .and_then(|d| d.as_array())
            .ok_or_else(|| ImageProviderError::ParseError("missing data array in response".to_string()))?;

        let mut images = Vec::with_capacity(data_array.len());

        for item in data_array {
            let url = item.get("url")
                .and_then(|u| u.as_str())
                .ok_or_else(|| ImageProviderError::ParseError("missing url in data item".to_string()))?;

            let revised_prompt = item.get("revised_prompt")
                .and_then(|p| p.as_str())
                .filter(|p| !p.is_empty())
                .map(String::from);

            // Download the image from the URL and return bytes
            let bytes = download_image(url).await?;

            // Determine format from bytes (default to PNG)
            let format = ImageFormat::Png;
            let (width, height) = request.aspect_ratio.dimensions();

            images.push(ImageOutput {
                bytes,
                original_url: Some(url.to_string()),
                revised_prompt,
                format,
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
impl GenerationService for OpenAIProvider {
    async fn generate(&self, input: GenerationInput) -> common::AppResult<GenerationOutput> {
        let model = input.model.as_deref().unwrap_or("dall-e-3");
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
            provider: GenerationProvider::Openai,
            metadata: GenerationMetadata {
                model_id: resp.model,
                inference_time_ms: resp.inference_time_ms,
                prompt_tokens: None,
                extra: serde_json::json!({}),
            },
        })
    }

    fn provider(&self) -> GenerationProvider {
        GenerationProvider::Openai
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
        (1024, 1792) | _ if height > width => AspectRatio::Ratio9x16,
        (1024, 768) => AspectRatio::Ratio4x3,
        (768, 1024) => AspectRatio::Ratio3x4,
        _ => AspectRatio::Ratio1x1,
    }
}
