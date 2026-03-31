//! # Generation Worker
//!
//! Background worker process that dequeues generation jobs from DragonflyDB,
//! routes them to the appropriate AI provider, uploads results to S3, and
//! updates the database. Implements retry with exponential backoff.

use crate::queue::{GenerationJob, Queue};
use crate::providers::{
    AspectRatio, ClaudeProvider, GenerationRequest, GenerationResponse, GeminiProvider,
    ImageFormat, ImageProvider, OpenAIProvider,
};
use common::AppConfig;
use std::time::Duration;
use tracing::{error, info, warn};

const MAX_RETRIES: u32 = 3;
const BASE_BACKOFF_SECS: u64 = 2;

/// Worker state holding connections to DragonflyDB, PostgreSQL, and S3.
pub struct GenerationWorker {
    queue: Queue,
    pool: sqlx::PgPool,
    config: AppConfig,
}

impl GenerationWorker {
    /// Create a new generation worker.
    pub fn new(queue: Queue, pool: sqlx::PgPool, config: AppConfig) -> Self {
        Self {
            queue,
            pool,
            config,
        }
    }

    /// Route a generation request to the appropriate provider and execute it.
    async fn route_and_generate(
        &self,
        model: &str,
        request: &crate::providers::GenerationRequest,
    ) -> anyhow::Result<crate::providers::GenerationResponse> {
        let model_lower = model.to_lowercase();

        if model_lower.contains("dall-e") || model_lower.contains("openai") {
            let provider = crate::providers::OpenAIProvider::new(
                self.config.openai_api_key.clone(),
                self.config.openai_base_url.clone(),
            );
            if !provider.is_available() {
                anyhow::bail!("OpenAI provider not available (no API key)");
            }
            return provider
                .generate("", request)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e));

        } else if model_lower.contains("imagen") || model_lower.contains("gemini") {
            let provider = crate::providers::GeminiProvider::new(
                self.config.gemini_api_key.clone(),
                self.config.gemini_base_url.clone(),
            );
            if !provider.is_available() {
                anyhow::bail!("Gemini provider not available (no API key)");
            }
            return provider
                .generate("", request)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e));

        } else if model_lower.contains("claude") {
            let provider = crate::providers::ClaudeProvider::new(
                self.config.anthropic_base_url.clone(),
            );
            return provider
                .generate("", request)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e));
        }

        anyhow::bail!("unsupported model: {}", model)
    }

    /// Run the main worker loop: dequeue jobs, process them, handle retries.
    pub async fn run(&self) {
        info!("Generation worker started");
        loop {
            match self.queue.dequeue(5).await {
                Ok(Some(job)) => {
                    info!(generation_id = %job.generation_id, "dequeued job");
                    self.process_job(job).await;
                }
                Ok(None) => {
                    // No job available within timeout, loop and retry
                }
                Err(e) => {
                    error!(error = %e, "failed to dequeue job");
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
        }
    }

    /// Process a single generation job with retry logic.
    async fn process_job(&self, job: GenerationJob) {
        let mut attempts = 0;
        let mut last_error: Option<String> = None;

        while attempts < MAX_RETRIES {
            attempts += 1;

            match self.execute_job(&job).await {
                Ok(_) => {
                    info!(
                        generation_id = %job.generation_id,
                        attempts = attempts,
                        "job completed successfully"
                    );
                    return;
                }
                Err(e) => {
                    last_error = Some(e.to_string());
                    warn!(
                        generation_id = %job.generation_id,
                        attempt = attempts,
                        error = %e,
                        "job failed"
                    );

                    if attempts < MAX_RETRIES {
                        let backoff = BASE_BACKOFF_SECS * 2u64.pow(attempts - 1);
                        let duration = Duration::from_secs(backoff);
                        tokio::time::sleep(duration).await;
                    }
                }
            }
        }

        // All retries exhausted — mark as failed
        let err_msg = last_error.unwrap_or_else(|| "unknown error".to_string());
        if let Err(e) = self.notify_failure(&job, &err_msg).await {
            error!(generation_id = %job.generation_id, error = %e, "failed to mark job as failed");
        }
    }

    /// Execute a generation job: route to provider, upload to S3, update DB.
    async fn execute_job(&self, job: &GenerationJob) -> anyhow::Result<()> {
        use crate::GenerationRouter;

        // Notify: processing
        self.queue
            .publish_update(job.generation_id, "processing", "Starting generation...")
            .await
            .ok();

        // Update DB status to processing
        db::queries::generations::update_status(&self.pool, job.generation_id, "processing", None)
            .await
            .map_err(|e| anyhow::anyhow!("DB update failed: {}", e))?;

        // Build the GenerationRouter and generate
        let config = &self.config;

        // Build GenerationRequest from the job
        let request = crate::providers::GenerationRequest {
            prompt: job.enhanced_prompt.clone().unwrap_or_else(|| job.prompt.clone()),
            negative_prompt: None,
            num_images: 1,
            aspect_ratio: crate::providers::AspectRatio::Ratio1x1,
            model: job.model.clone(),
            format: crate::providers::ImageFormat::Png,
        };

        // Route to the appropriate provider based on model prefix
        let output = self.route_and_generate(&job.model, &request).await?;

        // Download the generated image bytes and upload to S3
        let s3_key = format!("generations/{}/{}.png", job.generation_id, 0);
        let storage = storage::S3Storage::new(storage::StorageConfig {
            endpoint: self.config.s3_endpoint.clone(),
            access_key: self.config.s3_access_key.clone(),
            secret_key: self.config.s3_secret_key.clone(),
            bucket: self.config.s3_bucket.clone(),
            region: "auto".to_string(),
        })
        .await?;

        let s3_url = storage
            .upload_bytes(&s3_key, output.images[0].bytes.clone(), "image/png")
            .await?;

        // Update DB with output URL and status
        db::queries::generations::update_status(
            &self.pool,
            job.generation_id,
            "completed",
            Some(&[s3_url]),
        )
        .await
        .map_err(|e| anyhow::anyhow!("DB update failed: {}", e))?;

        // Notify: completed
        self.queue
            .publish_update(job.generation_id, "completed", "Generation complete.")
            .await
            .ok();

        Ok(())
    }

    /// Notify failure status via pub/sub and update DB.
    async fn notify_failure(&self, job: &GenerationJob, error: &str) -> anyhow::Result<()> {
        db::queries::generations::update_status(&self.pool, job.generation_id, "failed", None)
            .await
            .map_err(|e| anyhow::anyhow!("DB update failed: {}", e))?;

        // Store error message
        db::queries::generations::set_error(&self.pool, job.generation_id, error)
            .await
            .ok();

        self.queue
            .publish_update(job.generation_id, "failed", error)
            .await?;

        Ok(())
    }
}
