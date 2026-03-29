# Plan 05: Sharing, Social & Admin Features

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` (recommended) or `superpowers:executing-plans` to implement this plan task-by-task.
>
> **⚠️ CRITICAL:** Every `// ...` stub comment must be fully implemented. Stubs are NOT final code.

> **Known issues from review:** (1) All crate files (`lib.rs`, API handlers) must be fully implemented — no empty stubs. (2) Share token generation: use `nanoid!` or `uuid` with URL-safe encoding. (3) Content moderation flow must be specified. (4) Coordinate `api/src/social.rs` — Plan 02 imports from it, so Plan 05 creates it first. (5) Create admin analytics DB queries. (6) Public profile page must include FollowerList component.

**Goal:** Implement sharing (public links, profiles), follow system, and admin dashboard (moderation, analytics, system API keys).

**Prerequisites:** Plan 01, Plan 02, and Plan 03 should be completed first.

---

## SECTION 1: Sharing System

### Task 1: Share & Public API

**Files:**
- Create: `backend/crates/sharing/Cargo.toml` (new)
- Create: `backend/crates/sharing/src/lib.rs`
- Create: `backend/crates/api/src/sharing.rs`
- Create: `backend/crates/api/src/social.rs`

- [ ] **Step 1: Create sharing crate**

```toml
[package]
name = "sharing"
version.workspace = true
edition.workspace = true
```

```rust
// sharing/src/lib.rs
// - create_share_link(generation_id, is_public) -> ShareToken
// - delete_share_link(generation_id)
// - get_shared_generation(token) -> SharedGenerationView
// - get_public_profile(username) -> PublicProfileView
// - increment_view_count(generation_id)
// - toggle_visibility(generation_id, visibility)
```

- [ ] **Step 2: Create API endpoints**

```rust
// POST /api/v1/generations/:id/share - Create/update share link
// DELETE /api/v1/generations/:id/share - Remove share
// GET /shared/:token - View shared generation (public, no auth)
// GET /public/:username - View public profile (public, no auth)
```

- [ ] **Step 3: Commit**

```bash
git add backend/crates/sharing/ backend/crates/api/src/sharing.rs
git commit -m "feat(sharing): add share links and public profile endpoints"
```

---

## SECTION 2: Follow System

### Task 2: Social API

**Files:**
- Create: `backend/crates/db/src/queries/social.rs`
- Create: `backend/crates/api/src/social.rs`

- [ ] **Step 1: Create social queries**

```rust
// follow_user(follower_id, following_id)
// unfollow_user(follower_id, following_id)
// get_followers(user_id, page, limit) -> Vec<UserSummary>
// get_following(user_id, page, limit) -> Vec<UserSummary>
// get_follower_count(user_id) -> count
// get_following_count(user_id) -> count
// is_following(follower_id, following_id) -> bool
```

- [ ] **Step 2: Create social API endpoints**

```rust
// POST /api/v1/users/:username/follow
// DELETE /api/v1/users/:username/follow
// GET /api/v1/users/:username/followers
// GET /api/v1/users/:username/following
```

- [ ] **Step 3: Commit**

```bash
git add backend/crates/db/src/queries/social.rs backend/crates/api/src/social.rs
git commit -m "feat(social): add follow system API endpoints"
```

---

## SECTION 3: Admin Dashboard Backend

### Task 3: Admin API

**Files:**
- Create: `backend/crates/api/src/admin.rs`

- [ ] **Step 1: Create admin handlers**

```rust
// Auth middleware: check role == "admin"

// GET /api/v1/admin/users - Paginated user list with search
// PUT /api/v1/admin/users/:id - Update user (role, subscription_tier)
// DELETE /api/v1/admin/users/:id - Suspend/delete user

// GET /api/v1/admin/api-keys - List system API keys
// POST /api/v1/admin/api-keys - Add system API key (encrypted)
// DELETE /api/v1/admin/api-keys/:id - Remove system API key

// GET /api/v1/admin/analytics - Usage analytics
//   Returns: { total_users, total_generations, generations_by_provider, user_growth[], cost_summary }

// GET /api/v1/admin/content - Browse all generations (moderation)
// DELETE /api/v1/admin/content/:id - Remove flagged content
```

- [ ] **Step 2: Commit**

```bash
git add backend/crates/api/src/admin.rs
git commit -m "feat(admin): add admin API endpoints for user management, analytics, and moderation"
```

---

## SECTION 4: Frontend Social Features

### Task 4: Sharing UI & Social Components

**Files:**
- Create: `frontend/src/lib/components/ShareModal.svelte`
- Create: `frontend/src/lib/components/FollowButton.svelte`
- Create: `frontend/src/lib/components/FollowerList.svelte`

- [ ] **Step 1: Create ShareModal.svelte**

```svelte
<!-- Toggle: Private / Shared / Public -->
<!-- Copy link button -->
<!-- QR code for shared link -->
<!-- Share to social platforms (optional) -->
```

- [ ] **Step 2: Create FollowButton.svelte**

```svelte
<!-- Follow / Following / Unfollow states -->
<!-- Loading state during API call -->
```

- [ ] **Step 3: Commit**

```bash
git add frontend/src/lib/components/ShareModal.svelte frontend/src/lib/components/FollowButton.svelte
git commit -m "feat(frontend): add share modal and follow button components"
```
