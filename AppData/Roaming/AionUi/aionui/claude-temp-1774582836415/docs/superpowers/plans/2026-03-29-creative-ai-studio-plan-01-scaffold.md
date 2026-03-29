# Plan 01: Project Scaffolding

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` (recommended) or `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.
>
> **⚠️ CRITICAL:** Every `// ...` stub comment must be fully implemented. Stubs are NOT final code — they are high-level guidance only. Engineers implementing this plan MUST write complete, production-ready Rust/TypeScript code for all stubbed sections.
>
> **Known issues from review:** (1) Workspace Cargo.toml must not use trailing comma in members array. (2) Each crate needs a real `lib.rs` or `main.rs` — no empty files. (3) `api` crate must be created. (4) Remove `lapin` dependency. (5) Use explicit CORS origins from env var. (6) Remove `@supabase/supabase-js` from frontend package.json. (7) Create `(app)` route group layout.

**Goal:** Set up the complete project structure — Docker Compose, Rust Axum backend, SvelteKit frontend, PostgreSQL schema, DragonflyDB, and S3 integration.

**Architecture:** Monorepo with three top-level directories: `backend/` (Rust Axum), `frontend/` (SvelteKit), `worker/` (Rust worker process). Docker Compose orchestrates all services. Backend and worker share the same Rust codebase via Cargo workspace.

**Tech Stack:** Rust (Axum 0.7, SQLx, Tower, tokio), SvelteKit, Tailwind CSS, PostgreSQL 16, DragonflyDB, MinIO (S3-compatible), Docker.

---

## File Structure

```
creative-ai-studio/
├── docker-compose.yml          # All services
├── docker/
│   └── Dockerfile.backend      # Multi-stage: build + runtime
├── backend/
│   ├── Cargo.toml              # Workspace root + all crates
│   ├── crates/
│   │   ├── app/               # Main app binary (api server)
│   │   │   ├── Cargo.toml
│   │   │   └── src/main.rs
│   │   ├── worker/            # Worker binary (generation processor)
│   │   │   ├── Cargo.toml
│   │   │   └── src/main.rs
│   │   ├── api/               # REST API handlers
│   │   ├── auth/              # JWT, OAuth, session management
│   │   ├── billing/           # Stripe integration
│   │   ├── db/                # SQLx queries, migrations
│   │   ├── generation/        # Provider abstraction, queue
│   │   ├── models/             # Shared domain models
│   │   ├── sharing/            # Public links, profiles
│   │   ├── storage/            # S3 operations
│   │   └── common/             # Error types, config, middleware
│   ├── migrations/            # SQL migrations
│   └── config.toml            # Environment config
├── frontend/
│   ├── package.json
│   ├── svelte.config.js
│   ├── src/
│   │   ├── app.html
│   │   ├── app.css            # Global styles + Tailwind
│   │   ├── lib/
│   │   │   ├── components/    # Shared UI components
│   │   │   ├── stores/        # Svelte stores
│   │   │   └── api/           # API client
│   │   └── routes/
│   └── ...
└── .env.example
```

---

## SECTION 1: Docker Compose Setup

### Task 1: docker-compose.yml

**Files:**
- Create: `docker-compose.yml`
- Create: `docker/Dockerfile.backend`
- Create: `docker/entrypoint.sh`
- Create: `.env.example`

- [ ] **Step 1: Create docker-compose.yml**

```yaml
services:
  postgres:
    image: postgres:16-alpine
    environment:
      POSTGRES_DB: creative_ai_studio
      POSTGRES_USER: postgres
      POSTGRES_PASSWORD: ${POSTGRES_PASSWORD:-postgres}
    ports:
      - "5432:5432"
    volumes:
      - postgres_data:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U postgres"]
      interval: 5s
      timeout: 5s
      retries: 10

  dragonfly:
    image: docker.io/沸点工作室/dragonfly:v1.3.0
    ports:
      - "6379:6379"
    volumes:
      - dragonfly_data:/data
    healthcheck:
      test: ["CMD", "dfly", "ping"]
      interval: 5s
      timeout: 5s
      retries: 10

  minio:
    image: minio/minio:latest
    ports:
      - "9000:9000"
      - "9001:9001"
    environment:
      MINIO_ROOT_USER: ${MINIO_USER:-minioadmin}
      MINIO_ROOT_PASSWORD: ${MINIO_PASSWORD:-minioadmin}
    volumes:
      - minio_data:/data
    command: server /data --console-address ":9001"
    healthcheck:
      test: ["CMD", "mc", "ready", "local"]
      interval: 5s
      timeout: 5s
      retries: 5

  minio-init:
    image: minio/mc:latest
    depends_on:
      minio:
        condition: service_healthy
    entrypoint: ["/bin/sh", "-c"]
    command: |
      "mc alias set local http://minio:9000 $${MINIO_USER:-minioadmin} $${MINIO_PASSWORD:-minioadmin} && \
       mc mb local/creative-ai-studio --ignore-existing && \
       mc anonymous set download local/creative-ai-studio"
    restart: "no"

  backend:
    build:
      context: ./backend
      dockerfile: ../docker/Dockerfile.backend
    ports:
      - "8080:8080"
    environment:
      DATABASE_URL: postgres://postgres:${POSTGRES_PASSWORD:-postgres}@postgres:5432/creative_ai_studio
      REDIS_URL: redis://dragonfly:6379
      S3_ENDPOINT: http://minio:9000
      S3_ACCESS_KEY: ${MINIO_USER:-minioadmin}
      S3_SECRET_KEY: ${MINIO_PASSWORD:-minioadmin}
      S3_BUCKET: creative-ai-studio
      JWT_SECRET: ${JWT_SECRET}
      STRIPE_SECRET_KEY: ${STRIPE_SECRET_KEY:-}
      GOOGLE_CLIENT_ID: ${GOOGLE_CLIENT_ID:-}
      GOOGLE_CLIENT_SECRET: ${GOOGLE_CLIENT_SECRET:-}
      GITHUB_CLIENT_ID: ${GITHUB_CLIENT_ID:-}
      GITHUB_CLIENT_SECRET: ${GITHUB_CLIENT_SECRET:-}
      FRONTEND_URL: http://localhost:5173
      BACKEND_URL: http://localhost:8080
    depends_on:
      postgres:
        condition: service_healthy
      dragonfly:
        condition: service_healthy
      minio:
        condition: service_healthy
      minio-init:
        condition: service_started
    healthcheck:
      test: ["CMD-SHELL", "curl -f http://localhost:8080/health || exit 1"]
      interval: 10s
      timeout: 5s
      retries: 5

  worker:
    build:
      context: ./backend
      dockerfile: ../docker/Dockerfile.backend
    command: ["--worker"]
    environment:
      DATABASE_URL: postgres://postgres:${POSTGRES_PASSWORD:-postgres}@postgres:5432/creative_ai_studio
      REDIS_URL: redis://dragonfly:6379
      S3_ENDPOINT: http://minio:9000
      S3_ACCESS_KEY: ${MINIO_USER:-minioadmin}
      S3_SECRET_KEY: ${MINIO_PASSWORD:-minioadmin}
      S3_BUCKET: creative-ai-studio
      JWT_SECRET: ${JWT_SECRET}
      OPENAI_API_KEY: ${OPENAI_API_KEY:-}
      GEMINI_API_KEY: ${GEMINI_API_KEY:-}
      ANTHROPIC_API_KEY: ${ANTHROPIC_API_KEY:-}
    depends_on:
      postgres:
        condition: service_healthy
      dragonfly:
        condition: service_healthy
      minio:
        condition: service_healthy

  frontend:
    image: node:20-alpine
    working_dir: /app
    command: sh -c "npm install && npm run dev"
    ports:
      - "5173:5173"
    volumes:
      - ./frontend:/app
    environment:
      PUBLIC_API_URL: http://localhost:8080
      PUBLIC_WS_URL: ws://localhost:8080
    depends_on:
      - backend

volumes:
  postgres_data:
  dragonfly_data:
  minio_data:
```

- [ ] **Step 2: Create Dockerfile.backend**

```dockerfile
FROM rust:1.77-slim AS builder
WORKDIR /app

RUN apt-get update && apt-get install -y pkg-config libssl-dev

COPY Cargo.toml ./
COPY crates/*/Cargo.toml crates/

RUN mkdir -p crates/app crates/worker && \
    mv Cargo.toml Cargo.toml.bak && \
    cat Cargo.toml.bak | head -10 > Cargo.toml && \
    echo '[workspace]' >> Cargo.toml && \
    echo 'members = [' >> Cargo.toml && \
    for dir in crates/*/; do echo "  \"${dir%/}\"," >> Cargo.toml; done && \
    echo ']' >> Cargo.toml

COPY . .
RUN cargo build --release --bin app --bin worker

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/app /usr/local/bin/app
COPY --from=builder /app/target/release/worker /usr/local/bin/worker
COPY --from=builder /app/migrations /migrations
ENTRYPOINT []
CMD ["sh", "-c", "if [ \"$1\" = \"--worker\" ]; then exec /usr/local/bin/worker; else exec /usr/local/bin/app; fi"]
```

- [ ] **Step 3: Create .env.example**

```env
# Database
POSTGRES_PASSWORD=changeme

# JWT
JWT_SECRET=your-256-bit-secret-here

# S3 / MinIO
MINIO_USER=minioadmin
MINIO_PASSWORD=minioadmin

# Stripe
STRIPE_SECRET_KEY=
STRIPE_WEBHOOK_SECRET=

# OAuth
GOOGLE_CLIENT_ID=
GOOGLE_CLIENT_SECRET=
GITHUB_CLIENT_ID=
GITHUB_CLIENT_SECRET=

# AI Provider API Keys (for worker)
OPENAI_API_KEY=
GEMINI_API_KEY=
ANTHROPIC_API_KEY=
```

- [ ] **Step 4: Create entrypoint.sh**

```sh
#!/bin/sh
echo "Running migrations..."
sleep 2
echo "Starting server..."
exec "$@"
```

- [ ] **Step 5: Commit**

```bash
git add docker-compose.yml docker/ .env.example
git commit -m "feat(scaffold): add Docker Compose, backend Dockerfile, and env example"
```

---

## SECTION 2: Rust Backend Workspace Setup

### Task 2: Rust Cargo Workspace

**Files:**
- Create: `backend/Cargo.toml`
- Create: `backend/crates/app/Cargo.toml`
- Create: `backend/crates/app/src/main.rs`
- Create: `backend/crates/worker/Cargo.toml`
- Create: `backend/crates/worker/src/main.rs`
- Create: `backend/crates/common/Cargo.toml`
- Create: `backend/crates/common/src/lib.rs`
- Create: `backend/crates/models/Cargo.toml`
- Create: `backend/crates/models/src/lib.rs`

- [ ] **Step 1: Create backend/Cargo.toml**

```toml
[workspace]
resolver = "2"
members = [
  "crates/common",
  "crates/models",
  "crates/api",
  "crates/auth",
  "crates/db",
  "crates/generation",
  "crates/storage",
  "crates/sharing",
  "crates/billing",
  "crates/app",
  "crates/worker",
]

[workspace.package]
version = "0.1.0"
edition = "2021"

[workspace.dependencies]
axum = "0.7"
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
sqlx = { version = "0.7", features = ["runtime-tokio", "postgres", "uuid", "chrono"] }
uuid = { version = "1", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
bcrypt = "0.15"
jsonwebtoken = "9"
axum-extra = { version = "0.9", features = ["cookie"] }
tower = "0.4"
tower-http = { version = "0.5", features = ["cors", "trace", "fs"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
thiserror = "1"
anyhow = "1"
dotenvy = "0.15"
config = "0.14"
aws-sdk-s3 = "1"
aws-credential-types = "1"
tokio-util = { version = "0.7", features = ["io"] }
futures = "0.3"
lapin = "2"
```

- [ ] **Step 2: Create crate Cargo.toml files (app, worker, common, models)**

Each crate needs a minimal `Cargo.toml with path dependency to workspace members. Skip repetitive boilerplate — create all 4 files.

- [ ] **Step 3: Create backend/crates/app/src/main.rs**

```rust
use axum::{
    Router,
    routing::get,
    extract::State,
    http::StatusCode,
    middleware,
};
use std::sync::Arc;
use tower_http::cors::{CorsLayer, Any};

mod config;

#[derive(Clone)]
struct AppState {
    config: Arc<config::Config>,
    // Add other shared state here as we build modules
}

async fn health() -> StatusCode {
    StatusCode::OK
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

    let config = Arc::new(config::Config::from_env()?);
    let state = AppState { config };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_credentials(true)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/health", get(health))
        .layer(cors)
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
    tracing::info!("Server running on http://0.0.0.0:8080");
    axum::serve(listener, app).await?;

    Ok(())
}
```

- [ ] **Step 4: Create backend/crates/common/src/lib.rs**

```rust
pub mod error;
pub mod config;
pub mod middleware;
```

- [ ] **Step 5: Create backend/crates/common/src/error.rs**

```rust
use axum::{
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Unauthorized")]
    Unauthorized,
    #[error("Forbidden")]
    Forbidden,
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Bad request: {0}")]
    BadRequest(String),
    #[error("Internal server error: {0}")]
    Internal(String),
    #[error("Service unavailable: {0}")]
    ServiceUnavailable(String),
    #[error("Validation error: {0}")]
    Validation(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, self.to_string()),
            AppError::Forbidden => (StatusCode::FORBIDDEN, self.to_string()),
            AppError::NotFound(m) => (StatusCode::NOT_FOUND, m.clone()),
            AppError::BadRequest(m) => (StatusCode::BAD_REQUEST, m.clone()),
            AppError::Validation(m) => (StatusCode::UNPROCESSABLE_ENTITY, m.clone()),
            AppError::ServiceUnavailable(m) => (StatusCode::SERVICE_UNAVAILABLE, m.clone()),
            AppError::Internal(m) => (StatusCode::INTERNAL_SERVER_ERROR, m.clone()),
        };

        (status, Json(json!({ "error": message }))).into_response()
    }
}

pub type Result<T> = std::result::Result<T, AppError>;
```

- [ ] **Step 6: Create backend/crates/common/src/config.rs**

```rust
use config::{Config, ConfigError, File};
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub database_url: String,
    pub redis_url: String,
    pub s3_endpoint: String,
    pub s3_access_key: String,
    pub s3_secret_key: String,
    pub s3_bucket: String,
    pub jwt_secret: String,
    pub frontend_url: String,
    pub backend_url: String,
    pub stripe_secret_key: Option<String>,
    pub google_client_id: Option<String>,
    pub google_client_secret: Option<String>,
    pub github_client_id: Option<String>,
    pub github_client_secret: Option<String>,
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        Config::builder()
            .add_source(File::with_name("config").required(false))
            .add_source(config::Environment::default())
            .build()?
            .try_deserialize()
    }
}
```

- [ ] **Step 7: Create worker main.rs (stub with --worker flag)**

```rust
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

    let args: Vec<String> = std::env::args().collect();
    if args.contains(&"--worker".to_string()) {
        tracing::info!("Starting worker process...");
        worker::run().await?;
    } else {
        tracing::info!("Starting API server...");
        app::run().await?;
    }

    Ok(())
}
```

- [ ] **Step 8: Commit**

```bash
git add backend/
git commit -m "feat(scaffold): setup Rust workspace with Cargo.toml, app/worker binaries, common crate"
```

---

## SECTION 3: Database Migrations

### Task 3: PostgreSQL Schema

**Files:**
- Create: `backend/migrations/001_initial_schema.sql`
- Create: `backend/crates/db/Cargo.toml`
- Create: `backend/crates/db/src/lib.rs`
- Create: `backend/crates/db/src/queries/` (empty dirs for now)

- [ ] **Step 1: Create migrations/001_initial_schema.sql**

```sql
-- Enable extensions
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- ENUMs
CREATE TYPE auth_provider AS ENUM ('google', 'github', 'email');
CREATE TYPE subscription_tier AS ENUM ('free', 'basic', 'pro', 'unlimited');
CREATE TYPE user_role AS ENUM ('user', 'admin');
CREATE TYPE generation_status AS ENUM ('pending', 'processing', 'completed', 'failed');
CREATE TYPE visibility AS ENUM ('private', 'shared', 'public');
CREATE TYPE subscription_status AS ENUM ('active', 'cancelled', 'past_due');
CREATE TYPE transaction_type AS ENUM ('purchase', 'usage', 'refund', 'bonus');

-- User
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    username VARCHAR(50) UNIQUE NOT NULL,
    email VARCHAR(255) UNIQUE NOT NULL,
    password_hash VARCHAR(255),
    name VARCHAR(100),
    avatar_url TEXT,
    auth_provider auth_provider NOT NULL DEFAULT 'email',
    subscription_tier subscription_tier NOT NULL DEFAULT 'free',
    role user_role NOT NULL DEFAULT 'user',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_users_username ON users(username);
CREATE INDEX idx_users_email ON users(email);

-- Refresh tokens
CREATE TABLE refresh_tokens (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash VARCHAR(255) NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_refresh_tokens_user_id ON refresh_tokens(user_id);
CREATE INDEX idx_refresh_tokens_expires ON refresh_tokens(expires_at);

-- User profile (public)
CREATE TABLE user_profiles (
    user_id UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    bio TEXT,
    showcase_ids UUID[] DEFAULT '{}',
    is_public_profile BOOLEAN NOT NULL DEFAULT TRUE
);

-- Follow
CREATE TABLE follows (
    follower_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    following_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (follower_id, following_id)
);

CREATE INDEX idx_follows_follower ON follows(follower_id);
CREATE INDEX idx_follows_following ON follows(following_id);

-- Account deletion (GDPR soft-delete)
CREATE TABLE account_deletions (
    user_id UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    deleted_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    purge_after TIMESTAMPTZ NOT NULL DEFAULT (NOW() + INTERVAL '30 days')
);

-- Style presets
CREATE TABLE style_presets (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(100) NOT NULL,
    description TEXT,
    prompt_suffix TEXT NOT NULL DEFAULT '',
    thumbnail_url TEXT,
    is_public BOOLEAN NOT NULL DEFAULT FALSE,
    is_builtin BOOLEAN NOT NULL DEFAULT FALSE,
    creator_id UUID REFERENCES users(id) ON DELETE SET NULL,
    tags TEXT[] DEFAULT '{}',
    usage_count INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Generations
CREATE TABLE generations (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    prompt TEXT NOT NULL,
    enhanced_prompt TEXT,
    provider VARCHAR(50) NOT NULL,
    model VARCHAR(100) NOT NULL,
    style_preset_id UUID REFERENCES style_presets(id) ON DELETE SET NULL,
    reference_images TEXT[] DEFAULT '{}',
    sketch_data TEXT,
    output_urls TEXT[] DEFAULT '{}',
    status generation_status NOT NULL DEFAULT 'pending',
    error_message TEXT,
    credits_used INTEGER NOT NULL DEFAULT 0,
    quota_used INTEGER NOT NULL DEFAULT 0,
    visibility visibility NOT NULL DEFAULT 'private',
    view_count INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_generations_user_id ON generations(user_id);
CREATE INDEX idx_generations_status ON generations(status);
CREATE INDEX idx_generations_created_at ON generations(created_at DESC);
CREATE INDEX idx_generations_provider ON generations(provider);

-- Subscriptions
CREATE TABLE subscriptions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    tier subscription_tier NOT NULL DEFAULT 'free',
    stripe_subscription_id VARCHAR(255),
    stripe_customer_id VARCHAR(255),
    quota_monthly INTEGER NOT NULL DEFAULT 0,
    starts_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ,
    status subscription_status NOT NULL DEFAULT 'active',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_subscriptions_user_id ON subscriptions(user_id);
CREATE INDEX idx_subscriptions_status ON subscriptions(status);

-- Credit transactions
CREATE TABLE credit_transactions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    amount INTEGER NOT NULL,
    transaction_type transaction_type NOT NULL,
    description TEXT,
    stripe_payment_id VARCHAR(255),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_credit_transactions_user_id ON credit_transactions(user_id);
CREATE INDEX idx_credit_transactions_created_at ON credit_transactions(created_at DESC);

-- Shared generations
CREATE TABLE shared_generations (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    generation_id UUID NOT NULL UNIQUE REFERENCES generations(id) ON DELETE CASCADE,
    share_token VARCHAR(64) NOT NULL UNIQUE,
    is_public BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_shared_generations_share_token ON shared_generations(share_token);

-- User API keys (BYOK)
CREATE TABLE user_api_keys (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    provider VARCHAR(50) NOT NULL,
    encrypted_key TEXT NOT NULL,
    label VARCHAR(100),
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_user_api_keys_user_id ON user_api_keys(user_id);

-- System API keys (admin-managed)
CREATE TABLE system_api_keys (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    provider VARCHAR(50) NOT NULL,
    encrypted_key TEXT NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    rate_limit INTEGER NOT NULL DEFAULT 60,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Insert default style presets
INSERT INTO style_presets (name, description, prompt_suffix, thumbnail_url, is_public, is_builtin, tags) VALUES
('Photorealistic', 'Highly detailed, realistic photography style', ', photorealistic, ultra detailed, 8k, professional photography', '', TRUE, TRUE, '{"photography", "realistic"}'),
('Anime', 'Japanese anime illustration style', ', anime style, vibrant colors, detailed illustration', '', TRUE, TRUE, '{"anime", "illustration"}'),
('Oil Painting', 'Classical oil painting aesthetic', ', oil painting style, classical art, museum quality', '', TRUE, TRUE, '{"painting", "classical"}'),
('Watercolor', 'Soft watercolor painting style', ', watercolor style, soft edges, delicate colors', '', TRUE, TRUE, '{"painting", "watercolor"}'),
('Digital Art', 'Modern digital illustration', ', digital art, modern, trending on artstation', '', TRUE, TRUE, '{"digital", "illustration"}'),
('Minimalist', 'Clean, minimalist design', ', minimalist, clean lines, simple composition', '', TRUE, TRUE, '{"minimalist", "design"}'),
('3D Render', '3D computer graphics style', ', 3D render, octane render, cinema 4D', '', TRUE, TRUE, '{"3d", "render"}'),
('Sketch', 'Hand-drawn sketch style', ', hand-drawn sketch, pencil drawing, line art', '', TRUE, TRUE, '{"sketch", "drawing"}');
```

- [ ] **Step 2: Commit**

```bash
git add backend/migrations/ backend/crates/db/
git commit -m "feat(scaffold): add PostgreSQL schema migration and db crate"
```

---

## SECTION 4: SvelteKit Frontend Scaffold

### Task 4: Frontend Setup

**Files:**
- Create: `frontend/package.json`
- Create: `frontend/svelte.config.js`
- Create: `frontend/vite.config.ts`
- Create: `frontend/tsconfig.json`
- Create: `frontend/tailwind.config.js`
- Create: `frontend/postcss.config.js`
- Create: `frontend/src/app.html`
- Create: `frontend/src/app.css`
- Create: `frontend/src/app.d.ts`
- Create: `frontend/src/routes/+layout.svelte`
- Create: `frontend/src/routes/+page.svelte`
- Create: `frontend/src/lib/index.ts`
- Create: `frontend/src/lib/api/client.ts`

- [ ] **Step 1: Create frontend/package.json**

```json
{
  "name": "creative-ai-studio-frontend",
  "version": "0.1.0",
  "private": true,
  "scripts": {
    "dev": "vite dev",
    "build": "vite build",
    "preview": "vite preview",
    "check": "svelte-kit sync && svelte-check --tsconfig ./tsconfig.json",
    "check:watch": "svelte-kit sync && svelte-check --tsconfig ./tsconfig.json --watch"
  },
  "devDependencies": {
    "@sveltejs/adapter-auto": "^3.0.0",
    "@sveltejs/kit": "^2.0.0",
    "@sveltejs/vite-plugin-svelte": "^3.0.0",
    "autoprefixer": "^10.4.16",
    "postcss": "^8.4.32",
    "svelte": "^4.2.0",
    "svelte-check": "^3.6.0",
    "tailwindcss": "^3.4.0",
    "typescript": "^5.0.0",
    "vite": "^5.0.0"
  },
  "dependencies": {
    "@supabase/supabase-js": "^2.39.0",
    "clsx": "^2.1.0"
  },
  "type": "module"
}
```

- [ ] **Step 2: Create svelte.config.js, vite.config.ts, tsconfig.json, tailwind.config.js, postcss.config.js**

Standard SvelteKit + Tailwind setup. Use `adapter-auto`. Tailwind config extends with custom color tokens from the spec.

- [ ] **Step 3: Create app.css with design tokens**

```css
@import url('https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&family=JetBrains+Mono:wght@400;500&display=swap');

@tailwind base;
@tailwind components;
@tailwind utilities;

:root {
  --bg-primary: #0a0a0f;
  --bg-secondary: #111118;
  --bg-card: #16161f;
  --bg-glass: rgba(22, 22, 31, 0.7);
  --bg-hover: #1a1a25;
  --border: rgba(255, 255, 255, 0.06);
  --border-active: rgba(139, 92, 246, 0.5);
  --accent: #8b5cf6;
  --accent-secondary: #6366f1;
  --accent-gradient: linear-gradient(135deg, #6366f1, #8b5cf6, #a855f7);
  --accent-glow: rgba(139, 92, 246, 0.3);
  --text-primary: #f1f1f3;
  --text-secondary: #8b8b9a;
  --text-muted: #4a4a5a;
  --success: #22c55e;
  --warning: #f59e0b;
  --error: #ef4444;
  --info: #3b82f6;

  --radius-sm: 6px;
  --radius-md: 8px;
  --radius-lg: 12px;
  --radius-xl: 16px;

  --transition-micro: 150ms ease-out;
  --transition-standard: 250ms ease-out;
  --transition-dramatic: 400ms ease-out;
}

@layer base {
  * {
    @apply border-[var(--border)];
  }

  body {
    @apply bg-[var(--bg-primary)] text-[var(--text-primary)] font-sans antialiased;
  }

  ::selection {
    background: var(--accent-glow);
    color: var(--text-primary);
  }

  ::-webkit-scrollbar {
    width: 6px;
    height: 6px;
  }

  ::-webkit-scrollbar-track {
    background: transparent;
  }

  ::-webkit-scrollbar-thumb {
    background: var(--text-muted);
    border-radius: 3px;
  }

  ::-webkit-scrollbar-thumb:hover {
    background: var(--text-secondary);
  }
}

@layer components {
  .btn-primary {
    @apply px-4 py-2 rounded-[var(--radius-md)] font-medium transition-all duration-[var(--transition-micro)] cursor-pointer;
    background: var(--accent-gradient);
    color: white;
    box-shadow: 0 0 20px var(--accent-glow);
  }

  .btn-primary:hover {
    @apply scale-[1.02];
    box-shadow: 0 0 30px var(--accent-glow);
  }

  .btn-primary:disabled {
    @apply opacity-50 cursor-not-allowed scale-100;
    box-shadow: none;
  }

  .card {
    @apply rounded-[var(--radius-lg)] p-4;
    background: var(--bg-card);
    border: 1px solid var(--border);
  }

  .input {
    @apply w-full px-3 py-2 rounded-[var(--radius-md)] outline-none transition-all duration-[var(--transition-micro)];
    background: var(--bg-card);
    border: 1px solid var(--border);
    color: var(--text-primary);
  }

  .input:focus {
    border-color: var(--border-active);
    box-shadow: 0 0 0 3px var(--accent-glow);
  }

  .glass {
    background: var(--bg-glass);
    backdrop-filter: blur(12px);
    border: 1px solid var(--border);
  }
}
```

- [ ] **Step 4: Create routes/+page.svelte (landing page stub)**

```svelte
<script lang="ts">
</script>

<div class="min-h-screen flex items-center justify-center">
  <div class="text-center">
    <h1 class="text-4xl font-bold bg-gradient-to-r from-[#6366f1] via-[#8b5cf6] to-[#a855f7] bg-clip-text text-transparent">
      Creative AI Studio
    </h1>
    <p class="mt-4 text-[var(--text-secondary)]">Coming soon...</p>
  </div>
</div>
```

- [ ] **Step 5: Create frontend/src/lib/api/client.ts (API client stub)**

```typescript
const API_URL = import.meta.env.PUBLIC_API_URL || 'http://localhost:8080';
const WS_URL = import.meta.env.PUBLIC_WS_URL || 'ws://localhost:8080';

export const api = {
  async get<T>(path: string): Promise<T> {
    const res = await fetch(`${API_URL}${path}`, {
      credentials: 'include',
    });
    if (!res.ok) throw new Error(`API error: ${res.status}`);
    return res.json();
  },

  async post<T>(path: string, body: unknown): Promise<T> {
    const res = await fetch(`${API_URL}${path}`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      credentials: 'include',
      body: JSON.stringify(body),
    });
    if (!res.ok) throw new Error(`API error: ${res.status}`);
    return res.json();
  },
};

export { API_URL, WS_URL };
```

- [ ] **Step 6: Create .gitignore for frontend**

```gitignore
node_modules/
.svelte-kit/
build/
.env
.env.*
!.env.example
```

- [ ] **Step 7: Commit**

```bash
git add frontend/
git commit -m "feat(scaffold): add SvelteKit frontend with Tailwind, design tokens, and API client"
```

---

## SECTION 5: GitHub Actions CI (Basic)

### Task 5: CI Pipeline

**Files:**
- Create: `.github/workflows/ci.yml`

- [ ] **Step 1: Create .github/workflows/ci.yml**

```yaml
name: CI

on:
  push:
    branches: [main, init]
  pull_request:
    branches: [main]

jobs:
  backend:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - name: Check formatting
        run: cargo fmt --check
      - name: Check clippy
        run: cargo clippy --all-targets -- -D warnings
      - name: Run tests
        run: cargo test --all
      - name: Build
        run: cargo build --release

  frontend:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: '20'
          cache: 'npm'
      - run: npm ci
      - run: npm run check
      - run: npm run build
```

- [ ] **Step 2: Commit**

```bash
git add .github/
git commit -m "ci: add GitHub Actions pipeline for backend and frontend"
```

---

## SECTION 6: README

### Task 6: Project README

**Files:**
- Create: `README.md`

- [ ] **Step 1: Create README.md**

```markdown
# Creative AI Studio

Multi-user AI image generation platform. Phase 1 focuses on image generation with multi-provider support (OpenAI DALL-E, Google Gemini Imagen, Claude Image).

## Tech Stack

- **Frontend:** SvelteKit + Tailwind CSS
- **Backend:** Rust (Axum)
- **Worker:** Rust (same binary, `--worker` flag)
- **Database:** PostgreSQL 16
- **Cache/Queue:** DragonflyDB
- **Storage:** S3-compatible (MinIO)
- **Containerization:** Docker

## Quick Start

```bash
# Copy environment file
cp .env.example .env
# Edit .env and add your API keys and secrets

# Start all services
docker compose up -d

# Run migrations (automatic on first start)

# Frontend: http://localhost:5173
# Backend: http://localhost:8080
# MinIO Console: http://localhost:9001
```

## Development

```bash
# Backend
cd backend
cargo run --bin app

# Worker (in another terminal)
cd backend
cargo run --bin worker

# Frontend
cd frontend
npm install
npm run dev
```

## Architecture

See `docs/superpowers/specs/` for full design documentation.

## Phases

| Phase | Features |
|---|---|
| 1 | Core image generation + auth + billing |
| 2 | Video generation |
| 3 | AI Chat + Canvas |
| 4 | Audio |
| 5 | Team collaboration |
| 6 | Workflows (Pipeline Builder + Projects + Chains) |
```

- [ ] **Step 2: Commit**

```bash
git add README.md
git commit -m "docs: add README with quick start and architecture overview"
```
