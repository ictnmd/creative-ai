//! # DragonflyDB Queue
//!
//! Job queue for generation requests using DragonflyDB as a broker.
//! Provides enqueue/dequeue operations and pub/sub for real-time updates.

use anyhow::Context;
use chrono::{DateTime, Utc};
use redis::aio::MultiplexedConnection;
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

const GENERATION_QUEUE_KEY: &str = "generation:queue";

/// A generation job to be processed by the worker.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationJob {
    pub generation_id: Uuid,
    pub user_id: Uuid,
    pub prompt: String,
    pub enhanced_prompt: Option<String>,
    pub provider: String,
    pub model: String,
    pub style_preset_id: Option<Uuid>,
    pub reference_images: Vec<String>,
    pub sketch_data: Option<String>,
    pub aspect_ratio: String,
    pub num_images: u32,
    pub idempotency_key: Uuid,
    pub created_at: DateTime<Utc>,
}

/// A real-time update message published via Redis pub/sub.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationUpdate {
    pub generation_id: Uuid,
    pub status: String,
    pub message: String,
    pub timestamp: DateTime<Utc>,
}

/// DragonflyDB-backed queue for generation jobs.
#[derive(Clone)]
pub struct Queue {
    conn: MultiplexedConnection,
}

impl Queue {
    /// Create a new Queue wrapping an existing multiplexed connection.
    pub fn new(conn: MultiplexedConnection) -> Self {
        Self { conn }
    }

    /// Enqueue a job by pushing it onto the tail of the queue list.
    pub async fn enqueue(&self, job: &GenerationJob) -> anyhow::Result<()> {
        let payload = serde_json::to_string(job)
            .context("failed to serialize GenerationJob")?;
        let mut conn = self.conn.clone();
        conn.rpush::<_, _, ()>(GENERATION_QUEUE_KEY, payload)
            .await
            .context("failed to RPUSH to generation queue")?;
        Ok(())
    }

    /// Dequeue a job by blocking-popping from the head of the queue.
    /// Returns None if no job is available within the timeout.
    pub async fn dequeue(&self, timeout_secs: u64) -> anyhow::Result<Option<GenerationJob>> {
        let mut conn = self.conn.clone();
        let result: Option<(String, String)> = conn
            .blpop(GENERATION_QUEUE_KEY, timeout_secs as f64)
            .await
            .context("failed to BLPOP from generation queue")?;

        match result {
            Some((_, payload)) => {
                let job: GenerationJob = serde_json::from_str(&payload)
                    .context("failed to deserialize GenerationJob")?;
                Ok(Some(job))
            }
            None => Ok(None),
        }
    }

    /// Publish a generation status update to a Redis pub/sub channel.
    pub async fn publish_update(
        &self,
        generation_id: Uuid,
        status: &str,
        message: &str,
    ) -> anyhow::Result<()> {
        let channel = format!("generation:{}:updates", generation_id);
        let update = GenerationUpdate {
            generation_id,
            status: status.to_string(),
            message: message.to_string(),
            timestamp: Utc::now(),
        };
        let payload = serde_json::to_string(&update)
            .context("failed to serialize GenerationUpdate")?;
        let mut conn = self.conn.clone();
        conn.publish::<_, _, ()>(&channel, payload)
            .await
            .context("failed to publish generation update")?;
        Ok(())
    }
}
