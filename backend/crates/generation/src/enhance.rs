//! # Prompt Enhancement
//!
//! Uses Anthropic Claude API to enhance image generation prompts.
//! The enhancement adds subject details, lighting, composition,
//! style references, mood, camera angles, and quality modifiers.

const ENHANCE_API_URL: &str = "https://api.anthropic.com/v1/messages";
const ENHANCE_MODEL: &str = "claude-sonnet-4-20250514";

/// System prompt used for prompt enhancement.
const ENHANCEMENT_SYSTEM_PROMPT: &str =
    "You are an expert at crafting detailed image generation prompts. \
     Given a user's brief prompt, expand it with: subject details, \
     lighting, composition, style reference, mood, camera angle, \
     and quality modifiers. Return ONLY the enhanced prompt, no explanation.";

/// Enhance a prompt using Anthropic Claude API.
///
/// Takes a user-provided prompt and returns an expanded, detailed version
/// suitable for image generation models.
pub async fn enhance_prompt(prompt: &str, api_key: &str) -> Result<String, crate::providers::ImageProviderError> {
    if api_key.is_empty() {
        return Err(crate::providers::ImageProviderError::NotAvailable(
            "Anthropic API key is required for prompt enhancement".to_string(),
        ));
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| crate::providers::ImageProviderError::RequestFailed(e.to_string()))?;

    #[derive(serde::Serialize)]
    struct Message {
        role: &'static str,
        content: String,
    }

    #[derive(serde::Serialize)]
    struct EnhanceRequest {
        model: &'static str,
        max_tokens: u32,
        system: &'static str,
        messages: Vec<Message>,
    }

    #[derive(serde::Deserialize)]
    struct ContentBlock {
        #[serde(rename = "type")]
        block_type: String,
        text: Option<String>,
    }

    #[derive(serde::Deserialize)]
    struct SuccessResponse {
        content: Vec<ContentBlock>,
    }

    #[derive(serde::Deserialize)]
    struct ErrorResponse {
        #[serde(rename = "type")]
        error_type: String,
        message: String,
    }

    let request_body = EnhanceRequest {
        model: ENHANCE_MODEL,
        max_tokens: 1024,
        system: ENHANCEMENT_SYSTEM_PROMPT,
        messages: vec![Message {
            role: "user",
            content: format!("Original prompt: {}", prompt),
        }],
    };

    let response = client
        .post(ENHANCE_API_URL)
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await
        .map_err(|e| crate::providers::ImageProviderError::RequestFailed(e.to_string()))?;

    let status = response.status();
    let body_text = response
        .text()
        .await
        .map_err(|e| crate::providers::ImageProviderError::RequestFailed(e.to_string()))?;

    if !status.is_success() {
        return Err(crate::providers::ImageProviderError::RequestFailed(format!(
            "Anthropic API error ({}): {}",
            status, body_text
        )));
    }

    let json: serde_json::Value = serde_json::from_str(&body_text)
        .map_err(|e| crate::providers::ImageProviderError::ParseError(format!(
            "failed to parse Anthropic response: {}",
            e
        )))?;

    // Check for error in response
    if let Some(error_obj) = json.get("error") {
        let error: ErrorResponse = serde_json::from_value(error_obj.clone())
            .map_err(|e| crate::providers::ImageProviderError::ParseError(format!(
                "failed to parse error object: {}", e
            )))?;
        return Err(crate::providers::ImageProviderError::RequestFailed(format!(
            "Anthropic API error: {} - {}",
            error.error_type, error.message
        )));
    }

    let success: SuccessResponse = serde_json::from_value(json.clone())
        .map_err(|e| crate::providers::ImageProviderError::ParseError(format!(
            "failed to parse success response: {}",
            e
        )))?;

    let content_blocks = success.content;

    let text = content_blocks
        .iter()
        .filter(|b| b.block_type == "text")
        .map(|b| b.text.as_deref().unwrap_or(""))
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string();

    if text.is_empty() {
        return Err(crate::providers::ImageProviderError::ParseError(
            "no text content in Anthropic response".to_string()
        ));
    }

    Ok(text)
}
