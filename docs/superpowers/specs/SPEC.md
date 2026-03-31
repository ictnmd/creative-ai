# Creative AI Studio - Phase 1 Specification

**Version:** 1.0.0
**Phase:** 1 - Core Generation
**Status:** Implemented
**Last Updated:** 2026-03-31

---

## Overview

Creative AI Studio is a multi-user AI image generation platform. Phase 1 focuses on core image generation capabilities, user authentication, and generation queue management.

### Key Features

- User registration/login with email + OAuth (Google, GitHub)
- AI image generation via multiple providers (OpenAI DALL-E 3, Google Gemini Imagen)
- Generation queue with DragonflyDB for async processing
- Real-time status updates via WebSocket
- Prompt enhancement using Claude API
- S3/MinIO storage for generated images
- Credit-based billing system with Stripe integration
- Generation history and management

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                        Creative AI Studio                             │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  ┌──────────────┐     ┌──────────────────┐     ┌────────────────┐  │
│  │   Frontend   │────▶│  Backend (API)   │────▶│  PostgreSQL    │  │
│  │  (SvelteKit) │     │    (Axum/Rust)   │     │                │  │
│  └──────────────┘     └────────┬─────────┘     └────────────────┘  │
│       ▲                        │                                   │
│       │ WebSocket              │ RPUSH/BLPOP                       │
│       │                        ▼                                   │
│       │               ┌────────────────┐                            │
│       │               │   DragonflyDB  │                            │
│       │               │   (Job Queue)  │                            │
│       │               └────────┬───────┘                            │
│       │                        │                                    │
│       │                        ▼                                    │
│       │               ┌────────────────┐     ┌────────────────┐   │
│       └───────────────▶│    Worker      │────▶│   S3/MinIO     │   │
│         (Status PubSub) │  (Generation)  │     │   (Storage)    │   │
│                        └────────────────┘     └────────────────┘   │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

### Tech Stack

| Layer          | Technology                                      |
|----------------|------------------------------------------------|
| Frontend       | SvelteKit, Tailwind CSS, TypeScript            |
| Backend API    | Rust (Axum), SQLx                              |
| Database       | PostgreSQL 16                                  |
| Cache / Queue  | DragonflyDB                                    |
| Storage        | S3 / MinIO                                     |
| Containerization | Docker, Docker Compose                        |

### Backend Crates (Rust Workspace)

| Crate          | Purpose                                        |
|----------------|------------------------------------------------|
| `api`          | HTTP API layer (Axum handlers, routes)        |
| `auth`         | JWT, OAuth (Google/GitHub), password hashing  |
| `billing`      | Stripe client, credit transactions, plans       |
| `common`       | Shared config, error types, AppResult          |
| `db`           | PostgreSQL connection pool, query modules      |
| `generation`   | AI providers, queue, worker, prompt enhance    |
| `models`       | Domain models (User, Generation, etc.)         |
| `sharing`      | Share links, public profiles, visibility        |
| `storage`      | S3 operations (upload, presigned URLs)         |
| `app`          | Main API server binary                         |
| `worker`       | Background generation worker binary             |

---

## Database Schema

PostgreSQL schema is defined in `backend/migrations/001_initial_schema.sql`.

### Tables

#### `users`
Primary user account table.

| Column              | Type                    | Notes                     |
|--------------------|-------------------------|---------------------------|
| id                 | UUID                    | Primary key               |
| username           | VARCHAR(50)             | Unique                    |
| email              | VARCHAR(255)            | Unique                    |
| password_hash      | VARCHAR(255)            | Nullable (OAuth users)    |
| name               | VARCHAR(100)            | Nullable                  |
| avatar_url         | TEXT                    | Nullable                  |
| auth_provider      | auth_provider (enum)    | 'google', 'github', 'email' |
| subscription_tier  | subscription_tier (enum)| 'free', 'basic', 'pro', 'unlimited' |
| role               | user_role (enum)        | 'user', 'admin'           |
| created_at         | TIMESTAMPTZ             | Default NOW()             |
| updated_at         | TIMESTAMMPT             | Auto-updated via trigger  |

#### `refresh_tokens`
JWT refresh tokens for session management.

| Column              | Type                    | Notes                     |
|--------------------|-------------------------|---------------------------|
| id                 | UUID                    | Primary key               |
| user_id            | UUID                    | FK → users(id)           |
| token_hash         | VARCHAR(255)            | Hashed token              |
| expires_at         | TIMESTAMPTZ             | Token expiration          |
| created_at         | TIMESTAMPTZ             | Default NOW()             |

#### `user_profiles`
Extended user profile data.

| Column              | Type                    | Notes                     |
|--------------------|-------------------------|---------------------------|
| user_id            | UUID                    | PK, FK → users(id)       |
| bio                | TEXT                    | Nullable                  |
| showcase_ids       | UUID[]                  | Featured generations      |
| is_public_profile  | BOOLEAN                 | Default TRUE              |

#### `follows`
User follow relationships.

| Column              | Type                    | Notes                     |
|--------------------|-------------------------|---------------------------|
| follower_id        | UUID                    | PK, FK → users(id)       |
| following_id       | UUID                    | PK, FK → users(id)       |
| created_at         | TIMESTAMPTZ             | Default NOW()             |

#### `account_deletions`
Scheduled account deletion requests.

| Column              | Type                    | Notes                     |
|--------------------|-------------------------|---------------------------|
| user_id            | UUID                    | PK, FK → users(id)       |
| deleted_at         | TIMESTAMPTZ             | Default NOW()             |
| purge_after        | TIMESTAMPTZ             | Default NOW() + 30 days  |

#### `style_presets`
Image style presets (built-in and custom).

| Column              | Type                    | Notes                     |
|--------------------|-------------------------|---------------------------|
| id                 | UUID                    | Primary key               |
| name               | VARCHAR(100)            |                           |
| description        | TEXT                    | Nullable                  |
| prompt_suffix      | TEXT                    | Appended to user prompts  |
| thumbnail_url      | TEXT                    | Nullable                  |
| is_public          | BOOLEAN                 | Default FALSE             |
| is_builtin         | BOOLEAN                 | Default FALSE             |
| creator_id         | UUID                    | FK → users(id), nullable  |
| tags               | TEXT[]                  | Search/browse tags        |
| usage_count        | INTEGER                 | Default 0                 |
| created_at         | TIMESTAMPTZ             | Default NOW()            |

**Built-in presets:** Photorealistic, Anime, Oil Painting, Watercolor, Digital Art, Minimalist, 3D Render, Sketch

#### `generations`
Image generation requests and results.

| Column              | Type                    | Notes                     |
|--------------------|-------------------------|---------------------------|
| id                 | UUID                    | Primary key               |
| user_id            | UUID                    | FK → users(id)           |
| prompt             | TEXT                    | User's prompt             |
| enhanced_prompt    | TEXT                    | Claude-enhanced prompt    |
| provider           | VARCHAR(50)             | 'openai', 'gemini', etc. |
| model              | VARCHAR(100)            | Model identifier          |
| style_preset_id    | UUID                    | FK → style_presets(id)   |
| reference_images   | TEXT[]                  | URLs to reference images  |
| sketch_data        | TEXT                    | Nullable                  |
| output_urls        | TEXT[]                  | S3 URLs of generated images |
| status             | generation_status (enum)| 'pending', 'processing', 'completed', 'failed' |
| error_message      | TEXT                    | Nullable                  |
| credits_used       | INTEGER                 | Default 0                 |
| quota_used         | INTEGER                 | Default 0                 |
| visibility         | visibility (enum)       | 'private', 'shared', 'public' |
| view_count         | INTEGER                 | Default 0                 |
| created_at         | TIMESTAMPTZ             | Default NOW()             |

#### `subscriptions`
User subscription plans.

| Column              | Type                    | Notes                     |
|--------------------|-------------------------|---------------------------|
| id                 | UUID                    | Primary key               |
| user_id            | UUID                    | FK → users(id)           |
| tier               | subscription_tier       |                           |
| stripe_subscription_id | VARCHAR(255)         | Nullable                  |
| stripe_customer_id | VARCHAR(255)             | Nullable                  |
| quota_monthly      | INTEGER                 | Default 0                 |
| starts_at          | TIMESTAMPTZ             | Default NOW()             |
| expires_at         | TIMESTAMPTZ             |                           |
| status             | subscription_status     | 'active', 'cancelled', 'past_due' |
| created_at         | TIMESTAMPTZ             | Default NOW()             |

#### `credit_transactions`
Credit purchase and usage history.

| Column              | Type                    | Notes                     |
|--------------------|-------------------------|---------------------------|
| id                 | UUID                    | Primary key               |
| user_id            | UUID                    | FK → users(id)           |
| amount             | INTEGER                 | Positive = purchase, negative = usage |
| transaction_type   | transaction_type (enum) | 'purchase', 'usage', 'refund', 'bonus' |
| description        | TEXT                    | Nullable                  |
| stripe_payment_id  | VARCHAR(255)            | Nullable                  |
| created_at         | TIMESTAMPTZ             | Default NOW()             |

#### `shared_generations`
Share link metadata for generations.

| Column              | Type                    | Notes                     |
|--------------------|-------------------------|---------------------------|
| id                 | UUID                    | Primary key               |
| generation_id      | UUID                    | FK → generations(id), UNIQUE |
| share_token        | VARCHAR(64)             | UNIQUE, nanoid(12)       |
| is_public          | BOOLEAN                 | Default FALSE             |
| created_at         | TIMESTAMPTZ             | Default NOW()             |

#### `user_api_keys`
User-provided API keys for Bring-Your-Own-Key.

| Column              | Type                    | Notes                     |
|--------------------|-------------------------|---------------------------|
| id                 | UUID                    | Primary key               |
| user_id            | UUID                    | FK → users(id)           |
| provider           | VARCHAR(50)             | 'openai', 'gemini', etc. |
| encrypted_key      | TEXT                    | Encrypted API key         |
| label              | VARCHAR(100)            | Nullable                  |
| is_active          | BOOLEAN                 | Default TRUE              |
| created_at         | TIMESTAMPTZ             | Default NOW()             |

#### `system_api_keys`
System-level API keys for AI providers.

| Column              | Type                    | Notes                     |
|--------------------|-------------------------|---------------------------|
| id                 | UUID                    | Primary key               |
| provider           | VARCHAR(50)             | 'openai', 'gemini', etc. |
| encrypted_key      | TEXT                    | Encrypted API key         |
| is_active          | BOOLEAN                 | Default TRUE              |
| rate_limit         | INTEGER                 | Default 60 req/min        |
| created_at         | TIMESTAMPTZ             | Default NOW()             |

---

## API Endpoints

Base URL: `http://localhost:8080/api/v1`

### Authentication (`/auth`)

| Method | Path                    | Auth | Description                    |
|--------|-------------------------|------|--------------------------------|
| POST   | /auth/register          | No   | Register new user              |
| POST   | /auth/login             | No   | Login with email/password      |
| POST   | /auth/logout            | Yes  | Logout (invalidate refresh token) |
| POST   | /auth/refresh           | No   | Refresh access token           |
| GET    | /auth/oauth/google      | No   | Initiate Google OAuth          |
| GET    | /auth/oauth/google/callback | No | Google OAuth callback         |
| GET    | /auth/oauth/github      | No   | Initiate GitHub OAuth          |
| GET    | /auth/oauth/github/callback | No | GitHub OAuth callback         |

### Users (`/users`)

| Method | Path                    | Auth | Description                    |
|--------|-------------------------|------|--------------------------------|
| GET    | /users/me               | Yes  | Get current user profile       |
| PUT    | /users/me               | Yes  | Update profile                 |
| DELETE | /users/me               | Yes  | Request account deletion       |
| GET    | /users/:username        | No   | Get public user info           |

### Account (`/account`)

| Method | Path                         | Auth | Description                    |
|--------|------------------------------|------|--------------------------------|
| POST   | /account/delete/confirm      | Yes  | Confirm account deletion       |
| GET    | /account/deletion/status      | Yes  | Check deletion request status  |

### Generations (`/generations`)

| Method | Path                            | Auth | Description                    |
|--------|---------------------------------|------|--------------------------------|
| POST   | /generations                    | Yes  | Create new generation          |
| GET    | /generations                    | Yes  | List user's generations       |
| GET    | /generations/:id                | Yes  | Get generation by ID           |
| DELETE | /generations/:id                | Yes  | Delete generation              |
| GET    | /generations/:id/status         | Yes  | Poll generation status         |

### Presets (`/presets`)

| Method | Path                            | Auth | Description                    |
|--------|---------------------------------|------|--------------------------------|
| GET    | /presets                        | No   | List available presets         |
| GET    | /presets/:id                    | No   | Get preset details             |
| POST   | /presets                        | Yes  | Create custom preset           |
| PUT    | /presets/:id                    | Yes  | Update custom preset           |
| DELETE | /presets/:id                    | Yes  | Delete custom preset           |

### Reference Images (`/reference`)

| Method | Path                            | Auth | Description                    |
|--------|---------------------------------|------|--------------------------------|
| POST   | /reference/upload-url            | Yes  | Get S3 presigned upload URL    |
| GET    | /reference/:id                   | No   | Get reference image URL        |

### Social (`/social`)

| Method | Path                            | Auth | Description                    |
|--------|---------------------------------|------|--------------------------------|
| POST   | /social/follow/:userId          | Yes  | Follow a user                  |
| DELETE | /social/follow/:userId          | Yes  | Unfollow a user               |
| GET    | /social/followers/:userId        | No   | Get user's followers           |
| GET    | /social/following/:userId        | No   | Get users someone follows      |

### Sharing (`/sharing`)

| Method | Path                            | Auth | Description                    |
|--------|---------------------------------|------|--------------------------------|
| POST   | /sharing/generations/:id/link   | Yes  | Create share link              |
| DELETE | /sharing/generations/:id/link    | Yes  | Delete share link             |
| GET    | /sharing/shared/:token          | No   | View shared generation         |
| GET    | /sharing/profile/:username      | No   | View public profile            |
| PUT    | /sharing/generations/:id/visibility | Yes | Update visibility             |

### Billing (`/subscriptions`, `/credits`, `/webhooks`)

| Method | Path                            | Auth | Description                    |
|--------|---------------------------------|------|--------------------------------|
| GET    | /subscriptions                  | Yes  | List user subscriptions        |
| POST   | /subscriptions/subscribe        | Yes  | Subscribe to a plan            |
| DELETE | /subscriptions/:id              | Yes  | Cancel subscription            |
| GET    | /credits/balance                | Yes  | Get credit balance             |
| POST   | /credits/purchase               | Yes  | Purchase credits               |
| GET    | /credits/history                | Yes  | Get credit transaction history |
| POST   | /webhooks/stripe                | No   | Stripe webhook handler         |

### Admin (`/admin`)

| Method | Path                            | Auth | Description                    |
|--------|---------------------------------|------|--------------------------------|
| GET    | /admin/users                    | Admin| List all users                 |
| GET    | /admin/users/:id                | Admin| Get user details               |
| PUT    | /admin/users/:id/role           | Admin| Update user role               |
| DELETE | /admin/users/:id                | Admin| Delete user                    |
| GET    | /admin/generations              | Admin| List all generations           |
| GET    | /admin/analytics/overview       | Admin| Platform analytics             |

### Health

| Method | Path                    | Auth | Description                    |
|--------|-------------------------|------|--------------------------------|
| GET    | /health                 | No   | Health check                   |
| GET    | /api/v1/health          | No   | Health check (v1)              |

### WebSocket

| Path                      | Auth | Description                    |
|---------------------------|------|--------------------------------|
| /ws/generations           | Yes  | Real-time generation updates   |

---

## Request/Response Formats

### Create Generation

**Request:**
```json
{
  "prompt": "A serene mountain landscape at sunset",
  "provider": "openai",
  "model": "dall-e-3",
  "style_preset_id": "uuid-of-preset",
  "reference_images": ["https://..."],
  "aspect_ratio": "16:9",
  "num_images": 1,
  "idempotency_key": "optional-uuid"
}
```

**Response (202 Accepted):**
```json
{
  "generation_id": "uuid",
  "status": "pending",
  "message": "Generation queued"
}
```

### Generation Response
```json
{
  "id": "uuid",
  "user_id": "uuid",
  "prompt": "A serene mountain landscape at sunset",
  "enhanced_prompt": "A serene mountain landscape at sunset with golden hour lighting...",
  "provider": "openai",
  "model": "dall-e-3",
  "style_preset_id": "uuid",
  "reference_images": [],
  "output_urls": ["https://s3.../0.png"],
  "status": "completed",
  "error_message": null,
  "credits_used": 1,
  "visibility": "private",
  "view_count": 0,
  "created_at": "2026-03-31T10:00:00Z"
}
```

### Paginated List Response
```json
{
  "items": [],
  "total": 100,
  "limit": 20,
  "offset": 0
}
```

### Public Profile Response
```json
{
  "user_id": "uuid",
  "username": "artist1",
  "name": "Artist One",
  "avatar_url": "https://...",
  "bio": "Digital artist",
  "follower_count": 150,
  "following_count": 42,
  "generation_count": 30,
  "public_generations": [
    {
      "id": "uuid",
      "share_token": "abc123def456",
      "prompt": "...",
      "thumbnail_url": "https://...",
      "view_count": 50,
      "created_at": "2026-03-31T10:00:00Z"
    }
  ]
}
```

---

## Generation Pipeline

### Flow

```
1. User submits generation request via POST /api/v1/generations
2. API validates request, checks credits/quota
3. Generation record created in PostgreSQL (status: 'pending')
4. Job enqueued to DragonflyDB: RPUSH "generation:queue" { job_json }
5. Response returned to user (202 Accepted)
6. Worker dequeues job: BLPOP "generation:queue" (5s timeout)
7. Worker updates status to 'processing', publishes WebSocket update
8. Worker routes to appropriate provider (OpenAI/Gemini)
9. Provider generates image, returns bytes
10. Worker uploads to S3, gets presigned URL
11. Worker updates DB (status: 'completed', output_urls)
12. Worker publishes completion update via Redis pub/sub
13. User receives real-time update via WebSocket
```

### Retry Logic

- Max retries: 3
- Backoff: exponential (2s, 4s, 8s)
- After all retries exhausted: status → 'failed', error_message stored

### Provider Routing

The `GenerationRouter` selects provider based on the `model` parameter:

| Model Pattern | Provider    | Implementation        |
|---------------|-------------|----------------------|
| `dall-e*`     | OpenAI      | `providers/openai.rs` |
| `imagen*`     | Gemini      | `providers/gemini.rs` |
| `claude*`     | Anthropic   | `providers/claude.rs` (stub) |

Default provider: OpenAI (`dall-e-3`)

### Prompt Enhancement

Before generation, prompts can be enhanced via Claude API:
- Adds detailed subject description
- Appends lighting, composition, style modifiers
- Includes mood and quality descriptors
- Enhancement is optional per-request

---

## Authentication & Security

### JWT Tokens

**Access Token:** Short-lived (15 min), sent in `Authorization: Bearer` header
**Refresh Token:** Long-lived (7 days), stored in HTTP-only cookie

JWT Claims:
```json
{
  "sub": "user-uuid",
  "email": "user@example.com",
  "role": "user",
  "exp": 1743420000,
  "iat": 1743416100
}
```

### OAuth (PKCE)

Supported providers:
- **Google**: OAuth 2.0 with PKCE
- **GitHub**: OAuth 2.0 with PKCE

OAuth flow:
1. User clicks "Login with Google/GitHub"
2. Server generates PKCE code_verifier + code_challenge
3. Redirect to provider's authorization URL
4. Provider redirects back with authorization code
5. Server exchanges code for access token
6. Server creates/links user account
7. Server issues JWT access + refresh tokens

### Password Hashing

- Algorithm: Argon2
- Parameters: Memory cost 19456 KB, time cost 2, parallelism 1

---

## Configuration

All configuration via environment variables (`.env`):

| Variable              | Description                        | Example                          |
|-----------------------|------------------------------------|----------------------------------|
| `DATABASE_URL`        | PostgreSQL connection string       | `postgres://postgres:...@postgres:5432` |
| `REDIS_URL`           | DragonflyDB connection string     | `redis://dragonfly:6379`         |
| `JWT_SECRET`          | JWT signing secret (256-bit)      | `openssl rand -hex 32`          |
| `S3_ENDPOINT`         | S3/MinIO endpoint                  | `http://minio:9000`             |
| `S3_ACCESS_KEY`       | MinIO access key                  | `minioadmin`                    |
| `S3_SECRET_KEY`       | MinIO secret key                  | `minioadmin`                    |
| `S3_BUCKET`           | S3 bucket name                    | `creative-ai-studio`            |
| `STRIPE_SECRET_KEY`   | Stripe secret key                 | `sk_live_...`                   |
| `STRIPE_WEBHOOK_SECRET`| Stripe webhook secret             | `whsec_...`                     |
| `GOOGLE_CLIENT_ID`    | Google OAuth client ID             |                                  |
| `GOOGLE_CLIENT_SECRET`| Google OAuth client secret        |                                  |
| `GITHUB_CLIENT_ID`    | GitHub OAuth app client ID        |                                  |
| `GITHUB_CLIENT_SECRET`| GitHub OAuth app client secret    |                                  |
| `OPENAI_API_KEY`      | OpenAI API key                    |                                  |
| `GEMINI_API_KEY`      | Google Gemini API key             |                                  |
| `ANTHROPIC_API_KEY`   | Anthropic API key                 |                                  |
| `FRONTEND_URL`        | Frontend URL for redirects        | `http://localhost:5173`         |
| `BACKEND_URL`         | Backend URL                       | `http://localhost:8080`         |
| `CORS_ORIGINS`        | Allowed CORS origins              | `http://localhost:5173`         |

---

## Deployment

### Docker Compose Services

| Service      | Image                          | Ports        | Purpose                    |
|--------------|--------------------------------|--------------|----------------------------|
| postgres     | postgres:16-alpine             | 5432         | Database                   |
| dragonfly    | dragonflylab/dragonfly:v1.3.0 | 6379         | Job queue + pub/sub        |
| minio        | minio/minio:latest             | 9000, 9001   | Object storage             |
| minio-init   | minio/mc:latest                | -            | Bucket initialization      |
| backend      | Built from Dockerfile.backend  | 8080         | API server                 |
| worker       | Built from Dockerfile.backend  | -            | Generation worker          |
| frontend     | node:20-alpine                 | 5173         | SvelteKit dev server       |

### Resource Limits

| Service   | CPU Limit | Memory Limit |
|-----------|-----------|--------------|
| postgres  | 1 core    | 512 MB       |
| dragonfly | 1 core    | 256 MB       |
| minio     | 0.5 core  | 256 MB       |
| backend   | 2 cores   | 512 MB       |
| worker    | 2 cores   | 512 MB       |
| frontend  | 0.5 core  | 256 MB       |

---

## File Structure

```
.
├── backend/
│   ├── Cargo.toml
│   ├── Cargo.lock
│   ├── migrations/
│   │   └── 001_initial_schema.sql
│   └── crates/
│       ├── api/          # HTTP handlers
│       │   └── src/
│       │       ├── lib.rs
│       │       ├── auth.rs
│       │       ├── users.rs
│       │       ├── generations.rs
│       │       ├── presets.rs
│       │       ├── billing.rs
│       │       ├── reference.rs
│       │       ├── social.rs
│       │       ├── sharing.rs
│       │       ├── admin.rs
│       │       ├── ws_generations.rs
│       │       └── extractors.rs
│       ├── auth/         # JWT, OAuth, password
│       ├── billing/      # Stripe integration
│       ├── common/       # Config, errors
│       ├── db/           # Database layer
│       │   └── src/queries/
│       ├── generation/   # AI providers, queue, worker
│       │   └── src/providers/
│       ├── models/       # Domain models
│       ├── sharing/      # Share links
│       ├── storage/      # S3 operations
│       ├── app/          # API server binary
│       └── worker/       # Background worker binary
├── frontend/
│   ├── package.json
│   ├── svelte.config.js
│   ├── tailwind.config.js
│   ├── vite.config.ts
│   └── src/
│       ├── app.html
│       ├── app.css
│       ├── app.d.ts
│       └── lib/
│           ├── api/
│           │   ├── client.ts
│           │   └── ws-client.ts
│           ├── components/
│           │   ├── ui/   # Button, Input, Modal, etc.
│           │   ├── ShareModal.svelte
│           │   ├── FollowButton.svelte
│           │   ├── FollowerList.svelte
│           │   └── Toast.svelte
│           ├── stores/   # Svelte stores
│           ├── types/
│           └── utils/
├── docker/
│   ├── Dockerfile.backend
│   └── entrypoint.sh
├── docker-compose.yml
├── .env.example
├── README.md
└── docs/superpowers/specs/
    └── SPEC.md
```

---

## Phase 2 Preview

Phase 2 will add sharing & collaboration features:
- Share generations via URL
- Public galleries
- Comments and likes
- Social feed

See `README.md` for full phase roadmap.
