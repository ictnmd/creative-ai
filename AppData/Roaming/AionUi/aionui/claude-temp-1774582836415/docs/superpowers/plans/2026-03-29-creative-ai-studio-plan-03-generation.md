# Plan 03: Generation Core

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` (recommended) or `superpowers:executing-plans` to implement this plan task-by-task.
>
> **⚠️ CRITICAL:** Every `// ...` stub comment must be fully implemented. Stubs are NOT final code.
>
> **Known issues from review:** (1) Add `redis-rs` dependency to generation/Cargo.toml. (2) OpenAI, Gemini, Claude providers must be fully implemented — real API calls, response parsing, S3 upload. (3) Router must implement model-to-provider mapping. (4) Worker must implement dequeue loop, retry, WebSocket pub/sub. (5) Create `db/src/queries/generations.rs`. (6) WS endpoint path: `/ws/generations` (spec defines this). (7) Stripe billing must be fully implemented.

**Goal:** Implement the core generation pipeline — provider abstraction (OpenAI, Gemini, Claude), DragonflyDB job queue, worker process, WebSocket real-time updates, S3 storage, and quota/credit billing integration.

**Prerequisites:** Plan 01 (scaffold) and Plan 02 (auth) must be completed first.

**Architecture:** Generation requests go through: API → DB record → DragonflyDB queue → Worker picks up → Provider API call → Download image to S3 → Update DB → Push WebSocket notification.

---

## SECTION 1: Provider Abstraction

### Task 1: Image Provider Trait

**Files:**
- Create: `backend/crates/generation/Cargo.toml` (new)
- Create: `backend/crates/generation/src/lib.rs`
- Create: `backend/crates/generation/src/providers/openai.rs`
- Create: `backend/crates/generation/src/providers/gemini.rs`
- Create: `backend/crates/generation/src/providers/claude.rs`
- Create: `backend/crates/generation/src/router.rs`

- [ ] **Step 1: Create generation/Cargo.toml**

```toml
[package]
name = "generation"
version.workspace = true
edition.workspace = true

[dependencies]
common = { path = "../common" }
models = { path = "../models" }
storage = { path = "../storage" }
reqwest = { version = "0.11", features = ["json", "multipart"] }
serde = { workspace = true }
serde_json = { workspace = true }
anyhow = { workspace = true }
thiserror = { workspace = true }
tokio = { workspace = true }
uuid = { workspace = true }
url = "2"
base64 = "0.21"
```

- [ ] **Step 2: Create generation/src/lib.rs**

```rust
mod providers;
mod router;

pub use router::GenerationRouter;
pub use providers::{ImageProvider, GenerationRequest, GenerationResponse, ImageProviderError};
```

- [ ] **Step 3: Create generation/src/providers/mod.rs**

```rust
pub mod openai;
pub mod gemini;
pub mod claude;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct GenerationRequest {
    pub prompt: String,
    pub model: String,
    pub aspect_ratio: String,    // "1:1", "16:9", etc.
    pub num_images: u32,
    pub reference_images: Vec<String>,  // S3 URLs or data URLs
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationResponse {
    pub images: Vec<ImageOutput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageOutput {
    pub url: String,        // S3 URL after download
    pub original_url: String,  // Provider's URL
    pub revised_prompt: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum ImageProviderError {
    #[error("API key not configured")]
    ApiKeyMissing,
    #[error("API request failed: {0}")]
    RequestFailed(String),
    #[error("Rate limited")]
    RateLimited,
    #[error("Invalid response: {0}")]
    InvalidResponse(String),
    #[error("Quota exceeded")]
    QuotaExceeded,
}

#[async_trait]
pub trait ImageProvider: Send + Sync {
    fn name(&self) -> &'static str;
    fn supported_models(&self) -> Vec<&'static str>;
    fn cost_per_image(&self, model: &str) -> u32;  // credits

    async fn generate(
        &self,
        api_key: &str,
        request: &GenerationRequest,
    ) -> Result<GenerationResponse, ImageProviderError>;
}
```

- [ ] **Step 4: Create OpenAI provider**

```rust
// backend/crates/generation/src/providers/openai.rs
// Implements ImageProvider for OpenAI DALL-E 2/3
// - POST to https://api.openai.com/v1/images/generations
// - Handles API key from user BYOK or system key
// - Converts aspect ratio to OpenAI format (256x256, 512x512, 1024x1024, etc.)
// - Downloads returned image URL to S3
// - Returns GenerationResponse with S3 URLs
```

- [ ] **Step 5: Create Gemini provider**

```rust
// backend/crates/generation/src/providers/gemini.rs
// Implements ImageProvider for Google Gemini Imagen
// - POST to https://generativelanguage.googleapis.com/v1beta/models/imagen-3-generate:...
// - Uses API key from request
// - Converts aspect ratio to Gemini format
// - Downloads result to S3
```

- [ ] **Step 6: Create Claude provider**

```rust
// backend/crates/generation/src/providers/claude.rs
// Implements ImageProvider for Claude (Anthropic)
// - Uses Anthropic API for image generation
// - Handles Claude's specific response format
```

- [ ] **Step 7: Create generation router**

```rust
// backend/crates/generation/src/router.rs
// GenerationRouter: picks the right provider based on model name
// Maps model strings to providers: "dall-e-3" -> OpenAI, "imagen-3" -> Gemini, etc.
```

- [ ] **Step 8: Commit**

```bash
git add backend/crates/generation/
git commit -m "feat(generation): add provider abstraction layer for OpenAI, Gemini, Claude"
```

---

## SECTION 2: Job Queue & Worker

### Task 2: DragonflyDB Queue

**Files:**
- Create: `backend/crates/generation/src/queue.rs`
- Create: `backend/crates/generation/src/worker.rs`

- [ ] **Step 1: Create queue.rs**

```rust
// Queue operations using DragonflyDB (Redis-compatible)
// Uses LIST for job queue (RPUSH to enqueue, BLMPOP to dequeue)
// Key: "generation:queue"
// Each job is JSON-serialized GenerationJob

#[derive(Debug, Serialize, Deserialize)]
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

pub struct Queue {
    client: redis::aio::MultiplexedConnection,
}

impl Queue {
    pub async fn enqueue(&self, job: &GenerationJob) -> anyhow::Result<()> {
        let json = serde_json::to_string(job)?;
        redis::cmd("RPUSH")
            .arg("generation:queue")
            .arg(&json)
            .query_async(&mut self.client.clone())
            .await?;
        Ok(())
    }

    pub async fn dequeue(&self) -> anyhow::Result<Option<GenerationJob>> {
        // BLMPOP is not in redis-rs yet, use BRPOP as fallback
        let result: Option<(String, String)> = redis::cmd("BRPOP")
            .arg("generation:queue")
            .arg(0)  // no timeout
            .query_async(&mut self.client.clone())
            .await?;

        if let Some((_, json)) = result {
            let job: GenerationJob = serde_json::from_str(&json)?;
            Ok(Some(job))
        } else {
            Ok(None)
        }
    }
}
```

- [ ] **Step 2: Create worker.rs**

```rust
// Worker process: picks jobs from queue, processes them
// 1. DEQUEUE job from DragonflyDB
// 2. Get user's API key (BYOK first, fallback to system key)
// 3. Route to correct provider
// 4. Download image(s) to S3
// 5. Update generation record in PostgreSQL (status = completed)
// 6. Notify via WebSocket (Redis pub/sub on "generation:{id}:updates")
// 7. On failure: retry with exponential backoff (max 3)
// 8. On final failure: update status = failed

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Connect to DragonflyDB, PostgreSQL, S3
    // Loop: dequeue -> process -> update
    // Graceful shutdown on SIGTERM
}
```

- [ ] **Step 3: Commit**

```bash
git add backend/crates/generation/src/queue.rs backend/crates/generation/src/worker.rs
git commit -m "feat(worker): add DragonflyDB job queue and worker process"
```

---

## SECTION 3: Generation API

### Task 3: Generation REST + WebSocket Endpoints

**Files:**
- Create: `backend/crates/api/src/generations.rs`
- Create: `backend/crates/storage/Cargo.toml` (new)
- Create: `backend/crates/storage/src/lib.rs`

- [ ] **Step 1: Create storage/Cargo.toml and lib.rs**

```toml
[package]
name = "storage"
version.workspace = true
edition.workspace = true

[dependencies]
aws-sdk-s3 = { workspace = true }
anyhow = { workspace = true }
uuid = { workspace = true }
tokio = { workspace = true }
```

```rust
// storage/src/lib.rs
// S3 operations:
// - presign_upload_url(): Generate presigned URL for client upload (reference images)
// - upload_from_url(): Download image from URL and upload to S3
// - download_to_s3(): Stream download from provider URL to S3
// - generate_presigned_download_url(): Generate short-lived download URL
```

- [ ] **Step 2: Create generations.rs**

```rust
// API endpoints:
// POST /api/v1/generations - Create new generation
//   1. Validate request (prompt, model, etc.)
//   2. Check quota/credits
//   3. Create generation record (status = pending)
//   4. Enqueue job in DragonflyDB
//   5. Return generation_id immediately (async processing)
//
// GET /api/v1/generations - List user's generations (paginated)
// GET /api/v1/generations/:id - Get generation details
// DELETE /api/v1/generations/:id - Delete generation
// GET /api/v1/generations/:id/status - Poll status
```

- [ ] **Step 3: Create WebSocket handler**

```rust
// WS /ws/generations
// - Validate JWT from Authorization header
// - Subscribe to Redis pub/sub channel "generation:{id}:updates"
// - Forward messages to WebSocket client
// - Handle subscribe/unsubscribe messages from client
```

- [ ] **Step 4: Commit**

```bash
git add backend/crates/storage/ backend/crates/api/src/generations.rs
git commit -m "feat(api): add generation REST endpoints, WebSocket handler, and S3 storage"
```

---

## SECTION 4: Quota & Credit Billing

### Task 4: Billing Module

**Files:**
- Create: `backend/crates/billing/Cargo.toml` (new)
- Create: `backend/crates/billing/src/lib.rs`
- Create: `backend/crates/api/src/billing.rs`
- Modify: `backend/crates/api/src/generations.rs` (integrate billing check)

- [ ] **Step 1: Create billing/Cargo.toml and lib.rs**

```toml
[package]
name = "billing"
version.workspace = true
edition.workspace = true

[dependencies]
common = { path = "../common" }
models = { path = "../models" }
db = { path = "../db" }
stripe = { version = "0.26", features = ["runtime-tokio"] }
anyhow = { workspace = true }
```

```rust
// billing/src/lib.rs
// - get_effective_tier(user_id) -> subscription_tier
// - get_quota_remaining(user_id) -> quota_left
// - get_credit_balance(user_id) -> credits (from SUM of transactions)
// - deduct_quota(user_id) -> bool
// - deduct_credits(user_id, amount) -> Result<(), AppError>
// - get_provider_cost(provider, model) -> credits
// - determine_payment_source(user_id, provider, model) -> QuotaOrCredits
```

- [ ] **Step 2: Create billing API routes**

```rust
// GET /api/v1/subscriptions - List tiers with pricing
// POST /api/v1/subscriptions/subscribe - Create Stripe checkout session
// DELETE /api/v1/subscriptions/:id - Cancel subscription
// POST /api/v1/credits/purchase - Create Stripe checkout for credits
// GET /api/v1/credits/history - Transaction history
// POST /webhooks/stripe - Stripe webhook (subscription events, payment events)
```

- [ ] **Step 3: Commit**

```bash
git add backend/crates/billing/ backend/crates/api/src/billing.rs
git commit -m "feat(billing): add Stripe integration, quota/credit deduction, and billing endpoints"
```

---

## SECTION 5: Prompt Enhancement

### Task 5: AI Prompt Enhancement

**Files:**
- Create: `backend/crates/generation/src/enhance.rs`
- Create: `backend/crates/api/src/enhance.rs` (optional enhancement endpoint)

- [ ] **Step 1: Create enhance.rs**

```rust
// Uses Anthropic Claude API to enhance user prompts
// System prompt: "You are an expert at crafting detailed image generation prompts.
// Given a user's brief prompt, expand it with: subject details, lighting, composition,
// style reference, mood, camera angle, quality modifiers. Return ONLY the enhanced
// prompt, no explanation."
//
// POST to Anthropic API with user prompt
// Returns enhanced_prompt string
```

- [ ] **Step 2: Commit**

```bash
git add backend/crates/generation/src/enhance.rs
git commit -m "feat(generation): add AI prompt enhancement using Claude"
```

---

## SECTION 6: Presets & Reference Images

### Task 6: Presets & Storage API

**Files:**
- Create: `backend/crates/api/src/presets.rs`
- Create: `backend/crates/api/src/reference.rs` (S3 upload for reference images)

- [ ] **Step 1: Create presets.rs**

```rust
// GET /api/v1/presets - List available presets (builtin + user's custom)
// POST /api/v1/presets - Create custom preset
// PUT /api/v1/presets/:id - Update preset
// DELETE /api/v1/presets/:id - Delete preset
```

- [ ] **Step 2: Create reference.rs**

```rust
// POST /api/v1/reference/upload - Get presigned S3 upload URL for reference image
// POST /api/v1/reference/from-url - Fetch image from URL and upload to S3 (server-side)
// Handles max 10MB validation
```

- [ ] **Step 3: Commit**

```bash
git add backend/crates/api/src/presets.rs backend/crates/api/src/reference.rs
git commit -m "feat(api): add preset management and reference image upload endpoints"
```
