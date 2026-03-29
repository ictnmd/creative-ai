# Creative AI Studio — Design Document

> **Phase 1:** Core Image Generation
> **Date:** 2026-03-29
> **Status:** Approved (revised after review)

---

## 1. Overview

A multi-user creative AI platform for generating images via multiple providers (OpenAI DALL-E, Google Gemini Imagen, Claude Image). Phase 1 focuses on image generation only; video, chat, and audio will be added in future phases.

**Guiding principles:**
- Creator-focused workspace as primary interface
- Hybrid auth: OAuth + Email/Password
- Hybrid billing: Subscription quotas + Credits
- Premium models require credits (not quota)
- Modern, dark minimal UI with gradient accents

---

## 2. Tech Stack

| Layer | Technology |
|---|---|
| Frontend | SvelteKit + Tailwind CSS |
| Backend | Rust (Axum or Actix) |
| Database | PostgreSQL |
| Cache | DragonflyDB (Redis-compatible) |
| Storage | S3-compatible (reference images + generated outputs) |
| Containerization | Docker |
| Auth | OAuth (Google, GitHub) + Email/Password |
| Billing | Stripe (Subscriptions + Credits) |
| API Keys | Hybrid: Admin-managed + User bring-your-own |

---

## 3. Architecture

### 3.1 System Overview

```
┌─────────────────────────────────────────────────────────────┐
│                    SvelteKit Frontend                       │
│  (Dark UI / Tailwind / Component-based / Client routing)   │
└─────────────────────┬───────────────────────────────────────┘
                      │ HTTPS + JWT
┌─────────────────────▼───────────────────────────────────────┐
│                  Rust Backend (Axum)                         │
│  ┌─────────────┐ ┌──────────────┐ ┌──────────────────────┐  │
│  │ Auth Module │ │ Gen Module   │ │ Billing Module        │  │
│  │ - JWT       │ │ - Multi-prov │ │ - Quota tracking      │  │
│  │ - OAuth     │ │ - Queue      │ │ - Credit deduction    │  │
│  │ - Sessions  │ │ - Rate limit │ │ - Stripe webhooks     │  │
│  └─────────────┘ └──────────────┘ └──────────────────────┘  │
│  ┌─────────────┐ ┌──────────────┐ ┌──────────────────────┐  │
│  │ Preset Mgr  │ │ Share Module │ │ Admin Module          │  │
│  │ - Built-in  │ │ - Links      │ │ - User management     │  │
│  │ - Custom    │ │ - Public prof│ │ - API key management  │  │
│  └─────────────┘ └──────────────┘ │ - Content moderation  │  │
│                                    │ - Analytics           │  │
│                                    └──────────────────────┘  │
└───────┬───────────────┬───────────────────┬──────────────────┘
        │               │                   │
┌───────▼────┐  ┌──────▼────┐  ┌──────────▼──────────┐
│ PostgreSQL │  │ DragonflyDB│  │ S3 Storage           │
│ - Users    │  │ - Sessions │  │ - Reference images   │
│ - Gens     │  │ - Cache    │  │ - Generated outputs  │
│ - Presets  │  │ - Queue    │  │                      │
│ - Billing  │  │ - Rate lmt │  └──────────────────────┘
└───────────┘  └────────────┘
```

### 3.2 Generation Flow

1. User submits prompt + options via `/app/create`
2. Frontend validates input, checks quota/credits
3. Backend creates generation record (status: `pending`)
4. Generation queued in DragonflyDB
5. Worker picks from queue → calls provider API (OpenAI/Gemini/Claude)
6. Download generated image(s) to S3
7. Update generation record (status: `completed`, store S3 URLs)
8. Frontend polls `/generations/:id/status` or receives via WebSocket
9. Results displayed in results area

### 3.4 Worker Architecture

The generation worker is a **separate Rust process** (same binary, `--worker` flag) that:
- Connects to DragonflyDB for the job queue (BRPOPLPUSH pattern)
- Connects to PostgreSQL for generation records and user API keys
- Connects to S3 for upload/download
- Scales **horizontally**: run N worker instances behind a load balancer
- Workers are **stateless**: no shared state between instances
- Each worker processes **one generation at a time** with configurable concurrency
- On startup, workers register with DragonflyDB (health check key with TTL)
- Failed jobs are re-queued with a delay (max 3 retries, then marked `failed`)
- Workers use exponential backoff between retries when provider APIs are rate-limited

### 3.3 Credit/Quota Priority

- **Order:** Quota → Credits
- **Premium models:** Only payable with credits (quota does not cover)
- **On quota exhaustion:** System checks credit balance, auto-switches
- **On credit exhaustion:** Generate button disabled, prompt to purchase

---

## 4. Data Model

### User
| Field | Type | Notes |
|---|---|---|
| id | UUID | Primary key |
| username | VARCHAR(50) | Unique, URL-safe, used in `/public/:username` |
| email | VARCHAR(255) | Unique |
| password_hash | VARCHAR(255) | bcrypt, nullable for OAuth-only |
| name | VARCHAR(100) | Display name |
| avatar_url | TEXT | Nullable |
| auth_provider | ENUM | google, github, email |
| subscription_tier | ENUM | free, basic, pro, unlimited | Derived from active Subscription. Updated within same DB transaction as any subscription change. |
| role | ENUM | user, admin |
| created_at | TIMESTAMP | |
| updated_at | TIMESTAMP | |

> **Note:** `credit_balance` is NOT stored on User. Balance is computed from `SUM(amount)` in `CreditTransaction` table (cached in DragonflyDB for performance). If stored as a convenience field, it MUST be updated within the same DB transaction as every credit transaction to avoid drift.

### Generation
| Field | Type | Notes |
|---|---|---|
| id | UUID | Primary key |
| user_id | UUID | FK → User |
| prompt | TEXT | Original prompt |
| enhanced_prompt | TEXT | After AI enhancement |
| provider | VARCHAR(50) | openai, gemini, claude |
| model | VARCHAR(100) | e.g. dalle-3, imagen-3 |
| style_preset_id | UUID | FK → StylePreset, nullable |
| reference_images | TEXT[] | S3 URLs of reference images |
| sketch_data | TEXT | JSON sketch data, nullable |
| output_urls | TEXT[] | S3 URLs of generated outputs |
| status | ENUM | pending, processing, completed, failed |
| error_message | TEXT | Nullable |
| credits_used | INTEGER | |
| quota_used | INTEGER | |
| visibility | ENUM | private, shared, public |
| view_count | INTEGER | Default 0 |
| created_at | TIMESTAMP | |

### StylePreset
| Field | Type | Notes |
|---|---|---|
| id | UUID | Primary key |
| name | VARCHAR(100) | |
| description | TEXT | |
| prompt_suffix | TEXT | Added to user prompt |
| thumbnail_url | TEXT | |
| is_public | BOOLEAN | |
| is_builtin | BOOLEAN | |
| creator_id | UUID | FK → User, nullable for builtin |
| tags | TEXT[] | |
| usage_count | INTEGER | Default 0 |
| created_at | TIMESTAMP | |

### Subscription
| Field | Type | Notes |
|---|---|---|
| id | UUID | Primary key |
| user_id | UUID | FK → User |
| tier | ENUM | free, basic, pro, unlimited |
| stripe_subscription_id | VARCHAR(255) | Nullable |
| stripe_customer_id | VARCHAR(255) | |
| quota_monthly | INTEGER | Generations per month |
| starts_at | TIMESTAMP | |
| expires_at | TIMESTAMP | |
| status | ENUM | active, cancelled, past_due |
| created_at | TIMESTAMP | |

> **Note:** `quota_used` is NOT stored. It is computed at query time as `COUNT(*) FROM generation WHERE user_id = ? AND created_at >= starts_at`. This avoids the need for a reset mechanism and guarantees accuracy. Cache computed quota in DragonflyDB with TTL = 5 minutes.
>
> **Expired/cancelled subscription:** When `Subscription.status = cancelled` or `expires_at < NOW()`, the user's effective tier is `free` and `quota_monthly` is treated as 0. The system always uses the active subscription with `status = active` and `expires_at > NOW()`. If no active subscription exists, tier falls back to `free`.

### CreditTransaction
| Field | Type | Notes |
|---|---|---|
| id | UUID | Primary key |
| user_id | UUID | FK → User |
| amount | INTEGER | Positive = purchase, negative = usage |
| transaction_type | ENUM | purchase, usage, refund, bonus |
| description | TEXT | |
| stripe_payment_id | VARCHAR(255) | Nullable |
| created_at | TIMESTAMP | |

### SharedGeneration
| Field | Type | Notes |
|---|---|---|
| id | UUID | Primary key |
| generation_id | UUID | FK → Generation, UNIQUE |
| share_token | VARCHAR(64) | Unique, URL-safe |
| is_public | BOOLEAN | Appears in public browse/search |
| created_at | TIMESTAMP | |

> **Note:** `visibility` on `Generation` and `is_public` on `SharedGeneration` are linked:
> - `visibility = private` → no `SharedGeneration` record exists
> - `visibility = shared` → `SharedGeneration` record exists, `is_public = false`
> - `visibility = public` → `SharedGeneration` record exists, `is_public = true`
>
> `view_count` is only on `Generation` (not duplicated).

### UserProfile (public)
| Field | Type | Notes |
|---|---|---|
| user_id | UUID | PK, FK → User |
| bio | TEXT | |
| showcase_ids | UUID[] | Featured generation IDs |
| is_public_profile | BOOLEAN | Default true |

> **Note:** `follower_count` and `following_count` are derived from `Follow` table, not stored here.

### Follow
| Field | Type | Notes |
|---|---|---|
| follower_id | UUID | FK → User (the one who follows) |
| following_id | UUID | FK → User (the one being followed) |
| created_at | TIMESTAMP | |
| | | PRIMARY KEY (follower_id, following_id) |

### AccountDeletion
| Field | Type | Notes |
|---|---|---|
| user_id | UUID | FK → User |
| deleted_at | TIMESTAMP | Soft-delete timestamp |
| purge_after | TIMESTAMP | When data should be permanently erased (30 days) |

> **Account Deletion Flow:** When a user deletes their account, the system performs a soft-delete (sets `deleted_at`). All personally identifiable data is purged after 30 days (GDPR compliance). During the 30-day grace period, the user can contact support to restore. Generated images stored in S3 are scheduled for deletion at `purge_after`. All refresh tokens are revoked immediately.

---

### UserAPIKey (user bring-your-own)
| Field | Type | Notes |
|---|---|---|
| id | UUID | Primary key |
| user_id | UUID | FK → User |
| provider | VARCHAR(50) | openai, gemini, claude |
| encrypted_key | TEXT | Encrypted API key |
| label | VARCHAR(100) | User-defined label |
| is_active | BOOLEAN | |
| created_at | TIMESTAMP | |

### SystemAPIKey (admin-managed)
| Field | Type | Notes |
|---|---|---|
| id | UUID | Primary key |
| provider | VARCHAR(50) | openai, gemini, claude |
| encrypted_key | TEXT | |
| is_active | BOOLEAN | |
| rate_limit | INTEGER | Requests per minute |
| created_at | TIMESTAMP | |

---

## 5. Page Routes

```
/                         → Landing page (public)
/login                    → Login page
/register                 → Registration page
/forgot-password          → Password reset

/app                      → Redirects to /app/create
/app/create               → Main creator workspace
/app/gallery              → User's generations grid
/app/profile              → Profile & settings
/app/credits              → Credit purchase & history
/app/admin                → Admin dashboard (role-gated)

/public/:username         → Public profile (unauthenticated)
/shared/:token           → Shared generation view (unauthenticated)
```

### 5.1 /app/create — Creator Workspace

**Layout:** Two-column (left sidebar + right main area)

**Left Sidebar:**
- Tab navigation: Basic | Advanced | Models
- Style presets gallery (scrollable)
- Model selector dropdown
- Aspect ratio selector
- Number of images slider
- Reference panel toggle (Upload / URL / Sketch)
- Generate button

**Right Main Area:**
- Large prompt textarea (auto-resize)
- Action bar: Enhance button | Upload | URL | Sketch
- Results grid (streamed display)
- Generation status indicators

### 5.2 /app/gallery — Gallery View

**Layout:** Full-width grid + slide-in detail panel

**Grid:** Masonry or uniform grid of generation thumbnails
**Filters:** Provider, date range, status
**Detail Panel (slide-in from right):**
- Large image preview
- Prompt, model, style, timestamp
- Actions: Download, Remix, Share, Delete
- Visibility toggle

### 5.3 /app/profile — Profile & Settings

- Avatar & name editor
- Bio (for public profile)
- Subscription status & plan management
- Credit balance & purchase
- Custom style presets management
- User API keys (bring-your-own)
- Account settings (password, OAuth accounts)

### 5.4 /app/credits — Credits

- Current balance display
- Credit packs for purchase (Stripe)
- Transaction history table
- Auto-top-up toggle

### 5.5 /app/admin — Admin Dashboard

- **Users:** List, search, suspend, adjust quota
- **API Keys:** Manage system API keys per provider
- **Subscriptions:** View/manage subscriptions
- **Billing:** Stripe dashboard integration
- **Analytics:** Generation counts, provider usage, user growth
- **Moderation:** Browse & moderate user content

---

## 6. API Design

**Base URL:** `/api/v1`

### Auth
> **Base path:** `/auth` (these endpoints live outside `/api/v1` for OAuth redirect compatibility). All state-changing endpoints also validate `Sec-Fetch-Site` and `Origin` headers.

| Method | Endpoint | Description |
|---|---|---|
| POST | /auth/register | Email/password registration |
| POST | /auth/login | Login, returns JWT |
| POST | /auth/oauth/:provider | Initiate OAuth (Google/GitHub). Requires PKCE + state param for CSRF protection. State param stored in DragonflyDB with 10-min TTL. |
| GET | /auth/oauth/:provider/callback | OAuth callback, validates state param, exchanges code for token |
| POST | /auth/refresh | Refresh JWT (refresh token rotation). Body: `{ refresh_token: string }`. Returns new access + refresh token. Old refresh token invalidated. |
| POST | /auth/logout | Invalidate current session. Body: `{ refresh_token?: string }` (optional — if provided, revokes that specific refresh token) |
| POST | /auth/revoke-all | Revoke all refresh tokens for user (logout everywhere) |

### Generations
| Method | Endpoint | Description |
|---|---|---|
| POST | /generations | Create new generation. Body: `{ prompt, enhanced_prompt?, provider?, model?, style_preset_id?, reference_images?, sketch_data?, aspect_ratio?, num_images?, visibility? }` |
| GET | /generations | List user's generations (paginated). Query params: `?provider=`, `?status=`, `?from=`, `?to=`, `?page=`, `?limit=` |
| GET | /generations/:id | Get generation details |
| DELETE | /generations/:id | Delete generation |
| GET | /generations/:id/status | Poll generation status |
| WS | /generations/stream | WebSocket for real-time status |

#### WebSocket Protocol — `/generations/stream`

- **Connection:** `wss://api.example.com/ws/generations`
- **Auth:** JWT passed via `Authorization: Bearer <jwt>` header during WebSocket upgrade handshake. Not as query param (query params leak in logs/referrers).
- **Client → Server (subscribe):**
  ```json
  { "type": "subscribe", "generation_id": "uuid-here" }
  ```
- **Server → Client (status update):**
  ```json
  { "type": "status", "generation_id": "uuid-here", "status": "processing", "progress": 50 }
  { "type": "status", "generation_id": "uuid-here", "status": "completed", "output_urls": ["..."] }
  { "type": "error", "generation_id": "uuid-here", "message": "Provider API error" }
  ```
- **Reconnection:** Client auto-reconnects with exponential backoff (max 5 retries). Falls back to polling `/generations/:id/status` if WebSocket unavailable.

### Presets
| Method | Endpoint | Description |
|---|---|---|
| GET | /presets | List available presets |
| POST | /presets | Create custom preset |
| PUT | /presets/:id | Update preset |
| DELETE | /presets/:id | Delete preset |

### User
| Method | Endpoint | Description |
|---|---|---|
| GET | /user/profile | Get profile |
| PUT | /user/profile | Update profile |
| GET | /user/usage | Quota & credit usage |
| GET | /user/api-keys | List user API keys |
| POST | /user/api-keys | Add user API key |
| DELETE | /user/api-keys/:id | Remove user API key |

### Billing
| Method | Endpoint | Description |
|---|---|---|
| GET | /subscriptions | List subscription tiers |
| POST | /subscriptions/subscribe | Subscribe (Stripe) |
| DELETE | /subscriptions/:id | Cancel subscription |
| POST | /credits/purchase | Buy credits (Stripe) |
| GET | /credits/history | Credit transaction history |
| POST | /webhooks/stripe | Stripe webhook handler |

### Sharing
| Method | Endpoint | Description |
|---|---|---|
| POST | /generations/:id/share | Create share link |
| DELETE | /generations/:id/share | Remove share |
| GET | /shared/:token | View shared generation (public) |
| GET | /public/:username | Public profile (public) |

### Social (Follow)
| Method | Endpoint | Description |
|---|---|---|
| POST | /users/:username/follow | Follow a user |
| DELETE | /users/:username/follow | Unfollow a user |
| GET | /users/:username/followers | List followers. Query params: `?page=`, `?limit=` |
| GET | /users/:username/following | List following. Query params: `?page=`, `?limit=` |

### Admin
| Method | Endpoint | Description |
|---|---|---|
| GET | /admin/users | List all users (paginated) |
| PUT | /admin/users/:id | Update user (role, quota) |
| DELETE | /admin/users/:id | Suspend/delete user |
| GET | /admin/api-keys | List system API keys |
| POST | /admin/api-keys | Add system API key |
| DELETE | /admin/api-keys/:id | Remove system API key |
| GET | /admin/analytics | Usage analytics |
| GET | /admin/content | Browse all content (moderation) |
| DELETE | /admin/content/:id | Remove flagged content |

---

## 7. UI Design System

### 7.1 Color Palette (Dark Minimal + Accent Gradient)

> **Note:** Final color system will be refined using ui-ux-pro-max skill before implementation.

| Token | Value | Usage |
|---|---|---|
| bg-primary | `#0a0a0f` | Main background |
| bg-secondary | `#111118` | Sidebar, panels |
| bg-card | `#16161f` | Cards, inputs |
| bg-glass | `rgba(22,22,31,0.7)` | Glassmorphism panels |
| bg-hover | `#1a1a25` | Hover states |
| border | `rgba(255,255,255,0.06)` | Subtle borders |
| border-active | `rgba(139,92,246,0.5)` | Active/focus borders |
| accent | `#8b5cf6` | Primary accent (violet) |
| accent-secondary | `#6366f1` | Secondary accent (indigo) |
| accent-gradient | `linear-gradient(135deg,#6366f1,#8b5cf6,#a855f7)` | Gradient accents |
| accent-glow | `rgba(139,92,246,0.3)` | Glow/shadow effects |
| text-primary | `#f1f1f3` | Main text |
| text-secondary | `#8b8b9a` | Secondary text |
| text-muted | `#4a4a5a` | Muted/disabled text |
| success | `#22c55e` | Success states |
| warning | `#f59e0b` | Warning states |
| error | `#ef4444` | Error states |
| info | `#3b82f6` | Info states |

### 7.2 Typography

- **Font Family:** Inter (primary), system-ui fallback
- **Headings:** Inter, weight 600-700
- **Body:** Inter, weight 400
- **Monospace:** JetBrains Mono (for prompts, code)

### 7.3 Spacing

- Base unit: 4px
- Scale: 4, 8, 12, 16, 24, 32, 48, 64, 96
- Border radius: 6px (small), 8px (medium), 12px (large), 16px (xl)

### 7.4 Motion

- **Transitions:** 150ms ease-out (micro), 250ms ease-out (standard), 400ms ease-out (dramatic)
- **Entrance:** Fade + slight upward translate
- **Loading:** Pulse animation on skeleton cards
- **Generate button:** Gradient animation while processing

---

## 8. Component Inventory

| Component | States | Description |
|---|---|---|
| PromptInput | default, focused, loading, error | Auto-resize textarea, char count |
| EnhanceButton | idle, loading, success | AI prompt enhancement with animation |
| TabGroup | — | Basic / Advanced / Models tabs |
| StylePresetCard | default, selected, hover | Thumbnail + label, selectable |
| CustomPresetModal | — | Form to create custom preset |
| ReferencePanel | upload, url, sketch | 3 sub-tabs for reference input |
| SketchCanvas | drawing, empty | Canvas drawing tool for outlines |
| ModelSelect | open, closed | Dropdown with provider/model |
| AspectRatioSelect | — | 1:1, 16:9, 9:16, 4:3, 3:4 |
| NumImagesSlider | — | 1-4 images |
| GenerateButton | idle, loading, disabled | Gradient button, spinner |
| GenerationCard | default, hover, selected | Grid thumbnail |
| DetailPanel | open, closed | Slide-in right panel |
| GenerationActions | — | Download, Remix, Share, Delete |
| ShareModal | public, private, link | Share link creation + toggle |
| CreditBadge | — | Credit balance display |
| QuotaIndicator | — | Monthly quota progress bar |
| AuthModal | login, register, forgot | OAuth + email/password tabs |
| SubscriptionCard | free, basic, pro, unlimited | Pricing tier display |
| CreditPurchaseModal | — | Credit pack selection + Stripe |
| Toast | success, error, info, warning, loading | Notification system |
| LoadingSkeleton | — | Placeholder during loading |
| EmptyState | — | Empty gallery state |
| UserAvatar | — | Avatar with fallback initials |
| Sidebar | collapsed, expanded | Main navigation sidebar |
| FilterBar | — | Gallery filters: provider, date, status |
| Pagination | — | Page navigation for gallery and history |
| SubscriptionManagementModal | — | Upgrade/downgrade subscription tiers |
| DeleteConfirmationModal | — | Confirm before destructive actions |
| AdminUserTable | — | User list with search, suspend, quota edit |
| AdminContentModerationTable | — | Browse and moderate flagged content |
| AnalyticsDashboard | — | Charts and stats for admin analytics |
| AdminAPIKeyTable | — | System API key management |

---

## 9. Error Handling

| Scenario | Handling |
|---|---|
| API key quota exhausted | Toast error + redirect to settings |
| Generation failed | Error card in results + retry option |
| **Provider API failure** (timeout, error, rate limit) | Retry with exponential backoff (max 3 attempts). If all fail, mark generation as `failed` with error message, notify user. Do NOT silently fall back to another provider without user consent. |
| **Stripe webhook delivery failure** | Stripe retries webhooks automatically. Log failed deliveries; process on retry. |
| **DragonflyDB connection failure** | Return 503 Service Unavailable; sessions become unavailable, fall back to PostgreSQL for session storage |
| **PostgreSQL connection failure** | Return 503; generation queue pauses; admin alert triggered |
| Image upload too large | Client-side validation (max 10MB) |
| Rate limit exceeded | Toast warning + countdown timer |
| Credits exhausted | Disable generate, prompt purchase |
| Quota exhausted (credits remain) | Auto-switch to credit usage |
| WebSocket disconnect | Auto-reconnect + polling fallback |
| Session expired | Auto-redirect to login, preserve form |
| S3 upload failed | Retry 3x, then error + notification |
| Duplicate generation request | Idempotency key (UUID) on POST; deduplicate on backend |
| OAuth provider unavailable | Show error, suggest email/password login |

---

## 10. Security Considerations

- **API keys encrypted at rest (AES-256-GCM)** — Using AES-GCM provides both confidentiality and authenticity (authenticated encryption). Each key has a unique IV/nonce.
- **JWT with short expiry + refresh token rotation** — Access token: 15 min expiry. Refresh token: 7 day expiry, single-use (rotation on each refresh).
- **OAuth CSRF protection** — Authorization code flow with PKCE. State parameter stored in DragonflyDB with 10-minute TTL, validated on callback.
- **SvelteKit SPA: JWT stored in httpOnly cookie** — Not localStorage. Cookie has `SameSite=Strict` + `Secure` flag. CSRF protection via SameSite cookie policy (no separate CSRF token needed when using SameSite=Strict).
- **Rate limiting per user and per IP** — Implemented via DragonflyDB. Limits: 60 req/min per IP for auth endpoints, 100 req/min per user for generation endpoints.
- **Input sanitization** — All user prompts sanitized server-side. Reference image URLs validated and fetched server-side (not passed through to provider directly).
- **Content moderation** — On shared/public content before publication. Admin moderation queue for flagged content.
- **Admin routes** — Protected by role middleware. Separate admin JWT claims.
- **S3 presigned URLs** — With short expiry (15 minutes) for downloads.
- **Stripe webhook verification** — Verify `Stripe-Signature` header using webhook secret. Reject any request with invalid or missing signature.
- **BYOK fallback logic** — If user's own API key is invalid/rejected, do NOT silently fall back to system keys. Return clear error to user.
- **CSP headers** — Content-Security-Policy configured to restrict inline scripts and external domains.
- **CORS** — Backend configured with explicit allowed origins list (env var). No wildcard `*`.
- **Sketch data** — Max size limit enforced (1MB JSON payload).

---

## 11. Future Phases

| Phase | Features |
|---|---|
| Phase 2 | Video generation (Runway, Pika, Sora) |
| Phase 3 | AI Chat (Claude API, GPT) + Canvas editor |
| Phase 4 | Audio (TTS, music, voice cloning) |
| Phase 5 | Team collaboration, advanced subscriptions, analytics |
| **Phase 6** | **Creative Workflows (Pipeline Builder + Project/Team + Preset Chains)** |

## 11.1 Phase 6 — Creative Workflows

Phase 6 adds three interconnected workflow systems to transform the platform from a generation tool into a **creative operations platform**.

---

### A. Pipeline Builder (Node-based Automation)

A visual node editor for chaining multiple AI operations into automated pipelines.

**Core concept:** Each pipeline is a directed graph of nodes. Data flows from node to node. Users drag nodes onto a canvas, connect them, and execute the pipeline end-to-end.

**Node types:**

| Category | Node Types |
|---|---|
| Input | Prompt node, Image upload node, Reference image node, Sketch node |
| Generation | Image generation node (configurable provider/model), Video generation node, Audio generation node |
| Processing | Upscale node, Inpaint node, Remove background node, Enhance prompt node |
| Logic | Condition node (if/else based on output quality), Loop node (iterate over images), Merge node (combine outputs) |
| Output | Download node, Share node, Save to project node, Export node |
| Integration | API webhook node (trigger external services), Schedule node (cron-based triggers) |

**Pipeline features:**
- **Sub-pipelines:** Save a group of nodes as a reusable sub-pipeline (like a function)
- **Variables:** Define variables at pipeline level, reference them in any node
- **Loop/iteration:** Process multiple inputs through the same pipeline (batch processing)
- **Conditional branches:** Route execution based on output (e.g., if image quality < threshold, retry with different params)
- **Version history:** Every pipeline save creates a version; rollback supported
- **Execution modes:** Run synchronously (wait for result) or asynchronously (queue and notify)
- **Cost estimation:** Show estimated credit/quota cost before execution
- **Execution log:** Per-node status, timing, cost, and output for debugging

**Pipeline data model:**
```
Pipeline
├── id, user_id, name, description
├── graph_json (serialized node graph)
├── variables_json
├── is_public, is_builtin
├── last_run_at, run_count
├── created_at, updated_at

PipelineRun
├── id, pipeline_id, user_id
├── status (queued, running, completed, failed)
├── started_at, completed_at
├── total_cost_credits, total_cost_quota
├── trigger_type (manual, webhook, schedule)
├── execution_log_json (per-node results)
├── parent_pipeline_run_id (for loop iterations)
├── created_at
```

---

### B. Project & Team Workflow

Organize work into projects with team members, roles, permissions, and approval flows.

**Project concept:** A project is a container for related generations, pipelines, and assets. Teams collaborate within projects.

**Project features:**
- **Project spaces:** Create projects (e.g., "Marketing Campaign Q2", "Product Photoshoot")
- **Team roles:** Owner, Admin, Editor, Viewer. Role-based permissions on project resources
- **Asset library:** Shared reference images, uploaded assets, style guides per project
- **Task/approval flow:** Assign generation tasks to team members; reviewer approves/rejects outputs before publishing
- **Project-wide style presets:** Team admins can publish presets shared across all project members
- **Project analytics:** Per-project generation count, cost, and usage stats
- **Invitation system:** Invite members via email, generate invite links with expiry

**Project data model:**
```
Team
├── id, name, owner_id
├── plan_tier (team_small, team_large, enterprise)
├── settings_json (permissions defaults)
├── created_at

TeamMember
├── team_id, user_id
├── role (owner, admin, editor, viewer)
├── invited_by, joined_at

Project
├── id, team_id, name, description
├── is_archived, settings_json
├── created_at, updated_at

ProjectInvitation
├── id, project_id, email
├── token, role, expires_at
├── accepted_at
```

---

### C. Preset Chains (Reusable Generation Templates)

Extend the existing preset system into **chains** — parameterized templates that combine prompt structure, style, model, reference images, and post-processing steps.

**Chain features:**
- **Chain steps:** A chain contains ordered steps. Each step = (prompt segment + style + model + reference)
- **Parameters:** Mark any part of the prompt as a parameter `{subject}`, `{background}`, `{mood}`. When executing, user fills in parameters.
- **Batch execution:** Run the same chain across multiple parameter sets (e.g., generate the same style for 20 different products)
- **Template gallery:** Public chain templates shared by community; curated by admin
- **Chain linking:** Output of one chain becomes input of another (connects to Pipeline Builder)
- **Export/import:** Export chains as JSON for sharing or backup

**Chain data model:**
```
GenerationChain
├── id, user_id, name, description
├── steps_json (array of chain steps)
├── parameters_json (defined parameters with types/defaults)
├── is_public, is_builtin, category
├── usage_count, created_at, updated_at

ChainRun
├── id, chain_id, user_id
├── parameters_json (filled parameters)
├── results (array of generation_ids)
├── status, started_at, completed_at
├── cost_credits, cost_quota
```

---

### Phase 6 — New Routes

```
/app/projects                 → Project list
/app/projects/:id            → Project workspace
/app/projects/:id/settings   → Project settings & members
/app/pipelines               → Pipeline builder
/app/pipelines/:id           → Pipeline editor
/app/pipelines/:id/run       → Pipeline execution view
/app/chains                  → Chain template gallery
/app/chains/:id              → Chain editor
/app/team                    → Team management
/app/team/members            → Team members & invitations
```

---

### Phase 6 — New API Endpoints

**Projects:**
- `GET /api/v1/projects` — List user's projects
- `POST /api/v1/projects` — Create project
- `GET /api/v1/projects/:id` — Get project details
- `PUT /api/v1/projects/:id` — Update project
- `DELETE /api/v1/projects/:id` — Archive project
- `GET /api/v1/projects/:id/members` — List project members
- `POST /api/v1/projects/:id/invite` — Invite member
- `DELETE /api/v1/projects/:id/members/:userId` — Remove member
- `PUT /api/v1/projects/:id/members/:userId` — Update member role

**Pipelines:**
- `GET /api/v1/pipelines` — List pipelines
- `POST /api/v1/pipelines` — Create pipeline
- `GET /api/v1/pipelines/:id` — Get pipeline
- `PUT /api/v1/pipelines/:id` — Update pipeline (creates new version)
- `DELETE /api/v1/pipelines/:id` — Delete pipeline
- `POST /api/v1/pipelines/:id/run` — Execute pipeline
- `GET /api/v1/pipelines/:id/runs` — List pipeline runs
- `GET /api/v1/pipelines/:id/runs/:runId` — Get run details & log
- `POST /api/v1/pipelines/:id/export` — Export pipeline as JSON
- `POST /api/v1/pipelines/import` — Import pipeline from JSON

**Chains:**
- `GET /api/v1/chains` — List chains (public + user's)
- `POST /api/v1/chains` — Create chain
- `GET /api/v1/chains/:id` — Get chain
- `PUT /api/v1/chains/:id` — Update chain
- `DELETE /api/v1/chains/:id` — Delete chain
- `POST /api/v1/chains/:id/run` — Run chain with parameters
- `GET /api/v1/chains/:id/runs` — List chain runs

**Teams:**
- `GET /api/v1/teams` — List user's teams
- `POST /api/v1/teams` — Create team
- `GET /api/v1/teams/:id` — Get team
- `PUT /api/v1/teams/:id` — Update team
- `GET /api/v1/teams/:id/members` — List team members
- `POST /api/v1/teams/:id/invite` — Invite member to team
- `DELETE /api/v1/teams/:id/members/:userId` — Remove member

---

### Phase 6 — New Components

| Component | Description |
|---|---|
| PipelineCanvas | Visual node editor with drag-drop, connect, zoom, pan |
| PipelineNode | Individual node in the canvas (rendered per type) |
| PipelineSidebar | Node palette, search, drag nodes from palette to canvas |
| PipelineConfigPanel | Configure selected node's parameters |
| PipelineRunModal | Execute pipeline with variable input |
| PipelineExecutionLog | Per-node status, timing, output preview |
| ProjectCard | Project thumbnail with stats |
| ProjectHeader | Project nav: assets, members, settings, analytics |
| AssetLibrary | Uploaded assets grid with project-wide reference images |
| TeamMemberTable | Member list with role badges and actions |
| InvitationModal | Invite via email with role selection |
| ApprovalCard | Task card for review/approve/reject workflow |
| ChainEditor | Edit chain steps, define parameters, preview |
| ChainParameterForm | Fill in chain parameters before execution |
| ChainBatchRunner | Run chain across multiple parameter sets (CSV upload) |
| TemplateGallery | Browse public chain templates |

---

## 12. Reference Sites Analyzed

- **Leonardo AI** — Dark mode, neon gradient, feature-rich dashboard
- **Ideogram** — Light mode, minimal, typography-focused
- **Runway** — Dark editorial, cinematic, video-heavy
- **Midjourney** — Dark minimal, prompt-centric
