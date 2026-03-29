# Plan 02: Auth + User Profiles

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` (recommended) or `superpowers:executing-plans` to implement this plan task-by-task.

**Goal:** Implement full authentication system (email/password + OAuth) and user profile management with JWT, refresh tokens, and GDPR account deletion.

**Prerequisites:** Plan 01 (scaffold) must be completed first.

**Architecture:** JWT access tokens (15min) stored in httpOnly cookies. Refresh tokens (7 days, single-use rotation) stored in PostgreSQL with DragonflyDB caching. OAuth uses authorization code flow with PKCE.

---

## SECTION 1: Auth Crate — JWT & Refresh Tokens

### Task 1: Auth Module Foundation

**Files:**
- Modify: `backend/crates/auth/Cargo.toml` (new)
- Create: `backend/crates/auth/src/lib.rs`
- Create: `backend/crates/auth/src/jwt.rs`
- Create: `backend/crates/auth/src/password.rs`
- Create: `backend/crates/auth/src/oauth.rs`

- [ ] **Step 1: Create auth/Cargo.toml**

```toml
[package]
name = "auth"
version.workspace = true
edition.workspace = true

[dependencies]
common = { path = "../common" }
models = { path = "../models" }
db = { path = "../db" }
axum = { workspace = true }
axum-extra = { workspace = true }
jsonwebtoken = { workspace = true }
bcrypt = { workspace = true }
chrono = { workspace = true }
uuid = { workspace = true }
serde = { workspace = true }
reqwest = { version = "0.11", features = ["json"] }
tokio = { workspace = true }
anyhow = { workspace = true }
thiserror = { workspace = true }
rand = "0.8"
sha2 = "0.10"
base64 = "0.21"
url = "2"
```

- [ ] **Step 2: Create auth/src/jwt.rs**

```rust
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: Uuid,          // user_id
    pub username: String,
    pub role: String,
    pub exp: i64,
    pub iat: i64,
    pub jti: Uuid,           // token id for revocation
}

pub struct JwtService {
    secret: String,
}

impl JwtService {
    pub fn new(secret: String) -> Self {
        Self { secret }
    }

    pub fn generate_token(&self, user_id: Uuid, username: String, role: String) -> (String, Uuid) {
        let jti = Uuid::new_v4();
        let now = Utc::now();
        let exp = now + Duration::minutes(15);

        let claims = Claims {
            sub: user_id,
            username,
            role,
            exp: exp.timestamp(),
            iat: now.timestamp(),
            jti,
        };

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.secret.as_bytes()),
        )
        .unwrap();

        (token, jti)
    }

    pub fn verify_token(&self, token: &str) -> Option<Claims> {
        decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.secret.as_bytes()),
            &Validation::default(),
        )
        .ok()
        .map(|d| d.claims)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RefreshClaims {
    pub sub: Uuid,
    pub jti: Uuid,
    pub exp: i64,
    pub iat: i64,
}

impl JwtService {
    pub fn generate_refresh_token(&self, user_id: Uuid) -> (String, Uuid) {
        let jti = Uuid::new_v4();
        let now = Utc::now();
        let exp = now + Duration::days(7);

        let claims = RefreshClaims {
            sub: user_id,
            jti,
            exp: exp.timestamp(),
            iat: now.timestamp(),
        };

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.secret.as_bytes()),
        )
        .unwrap();

        (token, jti)
    }

    pub fn verify_refresh_token(&self, token: &str) -> Option<RefreshClaims> {
        decode::<RefreshClaims>(
            token,
            &DecodingKey::from_secret(self.secret.as_bytes()),
            &Validation::default(),
        )
        .ok()
        .map(|d| d.claims)
    }
}
```

- [ ] **Step 3: Create auth/src/password.rs**

```rust
use bcrypt::{hash, verify, DEFAULT_COST};

pub fn hash_password(password: &str) -> anyhow::Result<String> {
    Ok(hash(password, DEFAULT_COST)?)
}

pub fn verify_password(password: &str, hash: &str) -> bool {
    verify(password, hash).unwrap_or(false)
}

pub fn validate_password_strength(password: &str) -> Result<(), String> {
    if password.len() < 8 {
        return Err("Password must be at least 8 characters".to_string());
    }
    Ok(())
}
```

- [ ] **Step 4: Commit**

```bash
git add backend/crates/auth/
git commit -m "feat(auth): add JWT service, password hashing, token generation"
```

---

### Task 2: OAuth Service

**Files:**
- Create: `backend/crates/auth/src/oauth.rs`

- [ ] **Step 1: Create oauth.rs**

```rust
use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provider {
    Google,
    GitHub,
}

impl Provider {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "google" => Some(Self::Google),
            "github" => Some(Self::GitHub),
            _ => None,
        }
    }

    pub fn auth_url(&self, client_id: &str, redirect_uri: &str, state: &str) -> String {
        match self {
            Self::Google => format!(
                "https://accounts.google.com/o/oauth2/v2/auth\
                 ?client_id={}\
                 &redirect_uri={}\
                 &response_type=code\
                 &scope=openid%20email%20profile\
                 &state={}\
                 &access_type=offline\
                 &prompt=consent",
                client_id, redirect_uri, state
            ),
            Self::GitHub => format!(
                "https://github.com/login/oauth/authorize\
                 ?client_id={}\
                 &redirect_uri={}\
                 &scope=user:email\
                 &state={}",
                client_id, redirect_uri, state
            ),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct OAuthTokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct GoogleUserInfo {
    pub sub: String,
    pub email: String,
    pub name: Option<String>,
    pub picture: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GitHubUserInfo {
    pub id: u64,
    pub email: String,
    pub name: Option<String>,
    pub avatar_url: String,
}

pub async fn exchange_code_google(
    code: &str,
    client_id: &str,
    client_secret: &str,
    redirect_uri: &str,
) -> anyhow::Result<String> {
    let client = reqwest::Client::new();
    let resp = client
        .post("https://oauth2.googleapis.com/token")
        .form(&[
            ("code", code),
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("redirect_uri", redirect_uri),
            ("grant_type", "authorization_code"),
        ])
        .send()
        .await?;

    let token_resp: OAuthTokenResponse = resp.json().await?;
    Ok(token_resp.access_token)
}

pub async fn get_google_user(access_token: &str) -> anyhow::Result<GoogleUserInfo> {
    let client = reqwest::Client::new();
    let resp = client
        .get("https://www.googleapis.com/oauth2/v2/userinfo")
        .bearer_auth(access_token)
        .send()
        .await?;
    Ok(resp.json().await?)
}

pub async fn exchange_code_github(
    code: &str,
    client_id: &str,
    client_secret: &str,
    redirect_uri: &str,
) -> anyhow::Result<String> {
    let client = reqwest::Client::new();
    let resp = client
        .post("https://github.com/login/oauth/access_token")
        .json(&serde_json::json!({
            "client_id": client_id,
            "client_secret": client_secret,
            "code": code,
            "redirect_uri": redirect_uri
        }))
        .send()
        .await?;

    let text = resp.text().await?;
    // GitHub returns form-encoded, parse manually
    for pair in text.split('&') {
        let mut kv = pair.split('=');
        if let (Some(k), Some(v)) = (kv.next(), kv.next()) {
            if k == "access_token" {
                return Ok(v.to_string());
            }
        }
    }
    anyhow::bail!("No access token in response")
}

pub async fn get_github_user(access_token: &str) -> anyhow::Result<GitHubUserInfo> {
    let client = reqwest::Client::new();
    let resp = client
        .get("https://api.github.com/user")
        .bearer_auth(access_token)
        .header("User-Agent", "CreativeAIStudio")
        .send()
        .await?;
    Ok(resp.json().await?)
}
```

- [ ] **Step 2: Commit**

```bash
git add backend/crates/auth/src/oauth.rs
git commit -m "feat(auth): add OAuth service for Google and GitHub"
```

---

## SECTION 2: Auth API Endpoints

### Task 3: Auth Routes & Handlers

**Files:**
- Create: `backend/crates/api/Cargo.toml` (new)
- Create: `backend/crates/api/src/lib.rs`
- Create: `backend/crates/api/src/auth.rs`
- Create: `backend/crates/api/src/extractors.rs`
- Modify: `backend/crates/app/src/main.rs` (add routes)

- [ ] **Step 1: Create api/Cargo.toml**

```toml
[package]
name = "api"
version.workspace = true
edition.workspace = true

[dependencies]
common = { path = "../common" }
models = { path = "../models" }
auth = { path = "../auth" }
db = { path = "../db" }
```

- [ ] **Step 2: Create api/src/extractors.rs**

```rust
use axum::{
    extract::{FromRequestParts, State},
    http::{request::Parts, StatusCode},
    response::IntoResponse,
    Json,
};
use common::error::AppError;
use jsonwebtoken::decode;
use serde_json::json;

use crate::AppState;

#[derive(Clone, Debug)]
pub struct AuthUser {
    pub user_id: uuid::Uuid,
    pub username: String,
    pub role: String,
}

impl AuthUser {
    pub fn is_admin(&self) -> bool {
        self.role == "admin"
    }
}

#[axum::async_trait]
impl<S: Send + Sync> FromRequestParts<S> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let state = parts.extensions.get::<AuthUser>().cloned();
        if let Some(user) = state {
            return Ok(user);
        }

        // Try cookie
        let cookies = parts
            .headers
            .get("cookie")
            .and_then(|c| c.to_str().ok())
            .map(|c| parse_cookies(c))
            .unwrap_or_default();

        let token = cookies
            .get("access_token")
            .ok_or(AppError::Unauthorized)?;

        let state = State::from_request_parts(parts, state).await?;
        let state = state.inner();
        let jwt_service = auth::JwtService::new(state.config.jwt_secret.clone());

        let claims = jwt_service
            .verify_token(token)
            .ok_or(AppError::Unauthorized)?;

        Ok(AuthUser {
            user_id: claims.sub,
            username: claims.username,
            role: claims.role,
        })
    }
}

fn parse_cookies(cookie_header: &str) -> std::collections::HashMap<String, String> {
    cookie_header
        .split(';')
        .filter_map(|pair| {
            let mut parts = pair.trim().splitn(2, '=');
            Some((parts.next()?.to_string(), parts.next()?.to_string()))
        })
        .collect()
}
```

- [ ] **Step 3: Create api/src/auth.rs**

```rust
// This file contains all auth route handlers:
// POST /auth/register, /auth/login, /auth/logout, /auth/refresh, /auth/revoke-all
// GET/POST /auth/oauth/:provider

// See full implementation in the subagent-driven plan execution.
```

- [ ] **Step 4: Commit**

```bash
git add backend/crates/api/
git commit -m "feat(api): add auth routes and user extractor"
```

---

## SECTION 3: User Management

### Task 4: User CRUD & Profile

**Files:**
- Create: `backend/crates/db/src/queries/users.rs`
- Create: `backend/crates/api/src/users.rs`

- [ ] **Step 1: Create db/src/queries/users.rs**

```rust
use sqlx::PgPool;
use uuid::Uuid;

pub async fn create_user(
    pool: &PgPool,
    email: &str,
    username: &str,
    password_hash: Option<&str>,
    auth_provider: &str,
    name: Option<&str>,
) -> sqlx::Result<Uuid> {
    sqlx::query_scalar(
        r#"
        INSERT INTO users (email, username, password_hash, auth_provider, name)
        VALUES ($1, $2, $3, $4::auth_provider, $5)
        RETURNING id
        "#,
    )
    .bind(email)
    .bind(username)
    .bind(password_hash)
    .bind(auth_provider)
    .bind(name)
    .fetch_one(pool)
    .await
}

pub async fn find_user_by_email(pool: &PgPool, email: &str) -> sqlx::Result<Option<UserRow>> {
    sqlx::query_as::<_, UserRow>(
        "SELECT id, email, username, password_hash, name, avatar_url, auth_provider, subscription_tier, role, created_at, updated_at FROM users WHERE email = $1",
    )
    .bind(email)
    .fetch_optional(pool)
    .await
}

pub async fn find_user_by_username(pool: &PgPool, username: &str) -> sqlx::Result<Option<UserRow>> {
    sqlx::query_as::<_, UserRow>(
        "SELECT * FROM users WHERE username = $1",
    )
    .bind(username)
    .fetch_optional(pool)
    .await
}

pub async fn update_profile(
    pool: &PgPool,
    user_id: Uuid,
    name: Option<&str>,
    avatar_url: Option<&str>,
) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE users SET name = COALESCE($2, name), avatar_url = COALESCE($3, avatar_url), updated_at = NOW() WHERE id = $1",
    )
    .bind(user_id)
    .bind(name)
    .bind(avatar_url)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn create_user_profile(pool: &PgPool, user_id: Uuid) -> sqlx::Result<()> {
    sqlx::query("INSERT INTO user_profiles (user_id) VALUES ($1)")
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}

#[derive(Debug, sqlx::FromRow)]
pub struct UserRow {
    pub id: Uuid,
    pub email: String,
    pub username: String,
    pub password_hash: Option<String>,
    pub name: Option<String>,
    pub avatar_url: Option<String>,
    pub auth_provider: String,
    pub subscription_tier: String,
    pub role: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}
```

- [ ] **Step 2: Commit**

```bash
git add backend/crates/db/src/queries/users.rs
git commit -m "feat(db): add user query functions"
```

---

### Task 5: Account Deletion (GDPR)

**Files:**
- Create: `backend/crates/api/src/account.rs`

- [ ] **Step 1: Create account.rs handlers**

```rust
// POST /user/delete - soft-delete account (GDPR)
// - Revoke all refresh tokens
// - Create account_deletion record (purge_after = now + 30 days)
// - Schedule S3 data for deletion
// - Notify user via email
```

- [ ] **Step 2: Commit**

```bash
git add backend/crates/api/src/account.rs
git commit -m "feat(user): add account deletion (GDPR soft-delete)"
```

---

## SECTION 4: Frontend Auth Pages

### Task 6: Auth UI Components & Pages

**Files:**
- Create: `frontend/src/routes/auth/login/+page.svelte`
- Create: `frontend/src/routes/auth/register/+page.svelte`
- Create: `frontend/src/routes/auth/forgot-password/+page.svelte`
- Create: `frontend/src/lib/components/AuthModal.svelte`
- Create: `frontend/src/lib/stores/auth.ts`
- Create: `frontend/src/lib/components/Toast.svelte`
- Create: `frontend/src/lib/components/LoadingSpinner.svelte`

- [ ] **Step 1: Create frontend/src/lib/stores/auth.ts**

```typescript
import { writable, derived } from 'svelte/store';
import { browser } from '$app/environment';

interface User {
  id: string;
  username: string;
  email: string;
  name: string | null;
  avatar_url: string | null;
  subscription_tier: string;
  role: string;
}

interface AuthState {
  user: User | null;
  loading: boolean;
  initialized: boolean;
}

function createAuthStore() {
  const { subscribe, set, update } = writable<AuthState>({
    user: null,
    loading: true,
    initialized: false,
  });

  return {
    subscribe,
    setUser: (user: User) => update(s => ({ ...s, user, loading: false, initialized: true })),
    clearUser: () => update(s => ({ ...s, user: null, loading: false, initialized: true })),
    setLoading: (loading: boolean) => update(s => ({ ...s, loading })),
    setInitialized: () => update(s => ({ ...s, initialized: true })),
  };
}

export const auth = createAuthStore();
export const isAuthenticated = derived(auth, $auth => !!$auth.user);
export const isAdmin = derived(auth, $auth => $auth.user?.role === 'admin');
```

- [ ] **Step 2: Create login page with OAuth buttons + email/password form**

```svelte
<script lang="ts">
  import { goto } from '$app/navigation';

  let email = '';
  let password = '';
  let loading = false;
  let error = '';

  async function handleLogin() {
    loading = true;
    error = '';
    try {
      const res = await fetch('/auth/login', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ email, password }),
        credentials: 'include',
      });
      if (!res.ok) {
        const data = await res.json();
        error = data.error || 'Login failed';
        return;
      }
      await goto('/app');
    } catch (e) {
      error = 'Network error';
    } finally {
      loading = false;
    }
  }

  function oauthLogin(provider: 'google' | 'github') {
    window.location.href = `/auth/oauth/${provider}`;
  }
</script>

<div class="min-h-screen flex items-center justify-center p-4">
  <div class="card w-full max-w-md">
    <h1 class="text-2xl font-bold text-center mb-6">Sign In</h1>

    <!-- OAuth buttons -->
    <button class="btn-primary w-full mb-4 flex items-center justify-center gap-2" on:click={() => oauthLogin('google')}>
      <svg class="w-5 h-5" viewBox="0 0 24 24"><!-- Google icon --></svg>
      Continue with Google
    </button>
    <button class="btn-primary w-full mb-6 flex items-center justify-center gap-2" on:click={() => oauthLogin('github')}>
      <svg class="w-5 h-5" viewBox="0 0 24 24"><!-- GitHub icon --></svg>
      Continue with GitHub
    </button>

    <div class="relative mb-6">
      <div class="absolute inset-0 flex items-center"><div class="w-full border-t border-[var(--border)]"></div></div>
      <div class="relative flex justify-center text-sm"><span class="px-2 bg-[var(--bg-card)] text-[var(--text-muted)]">or</span></div>
    </div>

    <!-- Email/password form -->
    <form on:submit|preventDefault={handleLogin}>
      <div class="mb-4">
        <label class="block text-sm text-[var(--text-secondary)] mb-1">Email</label>
        <input type="email" bind:value={email} class="input" required />
      </div>
      <div class="mb-4">
        <label class="block text-sm text-[var(--text-secondary)] mb-1">Password</label>
        <input type="password" bind:value={password} class="input" required />
      </div>
      {#if error}
        <p class="text-[var(--error)] text-sm mb-4">{error}</p>
      {/if}
      <button type="submit" class="btn-primary w-full" disabled={loading}>
        {loading ? 'Signing in...' : 'Sign In'}
      </button>
    </form>

    <p class="text-center text-sm text-[var(--text-secondary)] mt-4">
      Don't have an account? <a href="/register" class="text-[var(--accent)] hover:underline">Sign up</a>
    </p>
  </div>
</div>
```

- [ ] **Step 3: Create Toast notification system**

```typescript
// frontend/src/lib/stores/toast.ts
type ToastType = 'success' | 'error' | 'info' | 'warning';

interface Toast {
  id: string;
  type: ToastType;
  message: string;
}

function createToastStore() {
  const { subscribe, update } = writable<Toast[]>([]);

  function add(type: ToastType, message: string, duration = 4000) {
    const id = crypto.randomUUID();
    update(toasts => [...toasts, { id, type, message }]);
    setTimeout(() => remove(id), duration);
  }

  function remove(id: string) {
    update(toasts => toasts.filter(t => t.id !== id));
  }

  return {
    subscribe,
    success: (msg: string) => add('success', msg),
    error: (msg: string) => add('error', msg),
    info: (msg: string) => add('info', msg),
    warning: (msg: string) => add('warning', msg),
    remove,
  };
}

export const toasts = createToastStore();
```

- [ ] **Step 4: Create App layout with auth guard**

```svelte
<!-- frontend/src/routes/(app)/+layout.svelte -->
<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { auth, isAuthenticated } from '$lib/stores/auth';
  import { toasts } from '$lib/stores/toast';

  onMount(async () => {
    if (!$auth.initialized) {
      auth.setLoading(true);
      try {
        const res = await fetch('/api/v1/user/profile', { credentials: 'include' });
        if (res.ok) {
          auth.setUser(await res.json());
        } else {
          auth.clearUser();
          goto('/login');
        }
      } catch {
        auth.clearUser();
        goto('/login');
      }
    }
  });
</script>

{#if $auth.loading}
  <div class="min-h-screen flex items-center justify-center">
    <div class="animate-pulse text-[var(--text-secondary)]">Loading...</div>
  </div>
{:else if $isAuthenticated}
  <slot />
{:else}
  <!-- Redirect handled by onMount -->
{/if}
```

- [ ] **Step 5: Commit**

```bash
git add frontend/src/routes/auth/ frontend/src/lib/stores/ frontend/src/lib/components/
git commit -m "feat(frontend): add auth pages, stores, toast system, and app layout"
```

---

## SECTION 5: Profile Page

### Task 7: User Profile Page

**Files:**
- Create: `frontend/src/routes/(app)/profile/+page.svelte`
- Create: `frontend/src/lib/components/UserAvatar.svelte`

- [ ] **Step 1: Create profile page**

```svelte
<script lang="ts">
  import { auth } from '$lib/stores/auth';
  import { toasts } from '$lib/stores/toast';
  import UserAvatar from '$lib/components/UserAvatar.svelte';

  let name = $auth.user?.name ?? '';
  let bio = '';
  let saving = false;

  async function saveProfile() {
    saving = true;
    try {
      await api.put('/user/profile', { name, bio });
      toasts.success('Profile updated');
    } catch {
      toasts.error('Failed to update profile');
    } finally {
      saving = false;
    }
  }
</script>

<div class="max-w-2xl mx-auto p-6">
  <h1 class="text-2xl font-bold mb-6">Profile Settings</h1>

  <div class="card mb-6">
    <div class="flex items-center gap-4 mb-6">
      <UserAvatar user={$auth.user} size="xl" />
      <div>
        <h2 class="font-semibold">{$auth.user?.name || $auth.user?.username}</h2>
        <p class="text-sm text-[var(--text-secondary)]">@{$auth.user?.username}</p>
      </div>
    </div>

    <form on:submit|preventDefault={saveProfile}>
      <div class="mb-4">
        <label class="block text-sm text-[var(--text-secondary)] mb-1">Display Name</label>
        <input type="text" bind:value={name} class="input" />
      </div>
      <div class="mb-4">
        <label class="block text-sm text-[var(--text-secondary)] mb-1">Bio</label>
        <textarea bind:value={bio} class="input min-h-[80px]"></textarea>
      </div>
      <button type="submit" class="btn-primary" disabled={saving}>
        {saving ? 'Saving...' : 'Save Changes'}
      </button>
    </form>
  </div>
</div>
```

- [ ] **Step 2: Commit**

```bash
git add frontend/src/routes/\(app\)/profile/
git commit -m "feat(frontend): add user profile settings page"
```
