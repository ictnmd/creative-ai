//! # Sharing Crate
//!
//! Share link generation and management for projects.
//! Handles token generation, expiry, and view tracking.

use rand::Rng;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Share link with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareLinkInfo {
    pub id: Uuid,
    pub project_id: Uuid,
    pub token: String,
    pub created_by: Uuid,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    pub view_count: i64,
    pub allow_download: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Share link creation request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateShareLinkRequest {
    pub project_id: Uuid,
    pub created_by: Uuid,
    pub expires_in_hours: Option<i64>,
    pub allow_download: bool,
}

/// Token generator for share links.
pub struct ShareTokenGenerator {
    length: usize,
}

impl ShareTokenGenerator {
    /// Create a new token generator.
    pub fn new(length: usize) -> Self {
        Self { length }
    }

    /// Generate a random alphanumeric token.
    pub fn generate(&self) -> String {
        const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
        let mut rng = rand::thread_rng();
        (0..self.length)
            .map(|_| {
                let idx = rng.gen_range(0..CHARSET.len());
                CHARSET[idx] as char
            })
            .collect()
    }
}

impl Default for ShareTokenGenerator {
    fn default() -> Self {
        Self::new(24)
    }
}

impl ShareLinkInfo {
    /// Create a new share link.
    pub fn new(request: CreateShareLinkRequest) -> Self {
        let generator = ShareTokenGenerator::default();
        let now = chrono::Utc::now();

        Self {
            id: Uuid::new_v4(),
            project_id: request.project_id,
            token: generator.generate(),
            created_by: request.created_by,
            expires_at: request.expires_in_hours.map(|h| {
                now + chrono::TimeDelta::hours(h)
            }),
            view_count: 0,
            allow_download: request.allow_download,
            created_at: now,
        }
    }

    /// Check if the share link has expired.
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            expires_at < chrono::Utc::now()
        } else {
            false
        }
    }
}
