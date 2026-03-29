//! User query operations

use chrono::{DateTime, Utc};
use common::AppResult;
use sqlx::PgPool;
use uuid::Uuid;

/// User row returned from the users table.
#[derive(Debug, Clone, sqlx::FromRow)]
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
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// User profile row.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct UserProfileRow {
    pub user_id: Uuid,
    pub bio: Option<String>,
    pub is_public_profile: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Find a user by their email address.
pub async fn find_by_email(pool: &PgPool, email: &str) -> AppResult<Option<UserRow>> {
    sqlx::query_as::<_, UserRow>(
        r#"
        SELECT id, email, username, password_hash, name, avatar_url,
               auth_provider, subscription_tier, role, created_at, updated_at
        FROM users
        WHERE email = $1
        "#,
    )
    .bind(email)
    .fetch_optional(pool)
    .await
    .map_err(common::AppError::from)
}

/// Find a user by their username.
pub async fn find_by_username(pool: &PgPool, username: &str) -> AppResult<Option<UserRow>> {
    sqlx::query_as::<_, UserRow>(
        r#"
        SELECT id, email, username, password_hash, name, avatar_url,
               auth_provider, subscription_tier, role, created_at, updated_at
        FROM users
        WHERE username = $1
        "#,
    )
    .bind(username)
    .fetch_optional(pool)
    .await
    .map_err(common::AppError::from)
}

/// Get a user by their ID.
pub async fn get_user_by_id(pool: &PgPool, user_id: Uuid) -> AppResult<Option<UserRow>> {
    sqlx::query_as::<_, UserRow>(
        r#"
        SELECT id, email, username, password_hash, name, avatar_url,
               auth_provider, subscription_tier, role, created_at, updated_at
        FROM users
        WHERE id = $1
        "#,
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(common::AppError::from)
}

/// Create a new email/password user.
pub async fn create_user(
    pool: &PgPool,
    email: &str,
    username: &str,
    password_hash: &str,
) -> AppResult<Uuid> {
    let id: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO users (id, email, username, password_hash, auth_provider, role)
        VALUES (uuid_generate_v4(), $1, $2, $3, 'email', 'user')
        RETURNING id
        "#,
    )
    .bind(email)
    .bind(username)
    .bind(password_hash)
    .fetch_one(pool)
    .await
    .map_err(common::AppError::from)?;

    Ok(id)
}

/// Create a new OAuth user and return their ID.
pub async fn create_oauth_user(
    pool: &PgPool,
    email: &str,
    username: &str,
    provider: &str,
    name: Option<&str>,
    avatar_url: Option<&str>,
) -> AppResult<Uuid> {
    let id: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO users (id, email, username, name, avatar_url, auth_provider, role, subscription_tier)
        VALUES (uuid_generate_v4(), $1, $2, $3, $4, $5, 'user', 'free')
        RETURNING id
        "#,
    )
    .bind(email)
    .bind(username)
    .bind(name)
    .bind(avatar_url)
    .bind(provider)
    .fetch_one(pool)
    .await
    .map_err(common::AppError::from)?;

    Ok(id)
}

/// Find an OAuth user by email and provider.
pub async fn find_oauth_user_by_email(
    pool: &PgPool,
    email: &str,
    provider: &str,
) -> AppResult<Option<UserRow>> {
    sqlx::query_as::<_, UserRow>(
        r#"
        SELECT id, email, username, password_hash, name, avatar_url,
               auth_provider, subscription_tier, role, created_at, updated_at
        FROM users
        WHERE email = $1 AND auth_provider = $2
        "#,
    )
    .bind(email)
    .bind(provider)
    .fetch_optional(pool)
    .await
    .map_err(common::AppError::from)
}

/// Update a user's profile fields.
pub async fn update_user_profile(
    pool: &PgPool,
    user_id: Uuid,
    name: Option<&str>,
    avatar_url: Option<&str>,
) -> AppResult<()> {
    sqlx::query(
        r#"
        UPDATE users
        SET name = COALESCE($2, name),
            avatar_url = COALESCE($3, avatar_url),
            updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(user_id)
    .bind(name)
    .bind(avatar_url)
    .execute(pool)
    .await
    .map_err(common::AppError::from)?;

    Ok(())
}

/// Create or ensure a user profile exists for the given user.
pub async fn create_profile(pool: &PgPool, user_id: Uuid) -> AppResult<()> {
    sqlx::query(
        r#"
        INSERT INTO user_profiles (user_id)
        VALUES ($1)
        ON CONFLICT (user_id) DO NOTHING
        "#,
    )
    .bind(user_id)
    .execute(pool)
    .await
    .map_err(common::AppError::from)?;

    Ok(())
}

/// Get user profile by user ID.
pub async fn get_profile(pool: &PgPool, user_id: Uuid) -> AppResult<Option<UserProfileRow>> {
    sqlx::query_as::<_, UserProfileRow>(
        r#"
        SELECT user_id, bio, is_public_profile, created_at, updated_at
        FROM user_profiles
        WHERE user_id = $1
        "#,
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(common::AppError::from)
}

/// Update user profile (bio, is_public_profile).
pub async fn update_profile_settings(
    pool: &PgPool,
    user_id: Uuid,
    bio: Option<&str>,
    is_public_profile: bool,
) -> AppResult<()> {
    sqlx::query(
        r#"
        INSERT INTO user_profiles (user_id, bio, is_public_profile)
        VALUES ($1, $2, $3)
        ON CONFLICT (user_id) DO UPDATE SET
            bio = EXCLUDED.bio,
            is_public_profile = EXCLUDED.is_public_profile
        "#,
    )
    .bind(user_id)
    .bind(bio)
    .bind(is_public_profile)
    .execute(pool)
    .await
    .map_err(common::AppError::from)?;

    Ok(())
}

/// Update a user's subscription tier.
pub async fn update_subscription_tier(
    pool: &PgPool,
    user_id: Uuid,
    tier: &str,
) -> AppResult<()> {
    sqlx::query(
        r#"
        UPDATE users SET subscription_tier = $2, updated_at = NOW() WHERE id = $1
        "#,
    )
    .bind(user_id)
    .bind(tier)
    .execute(pool)
    .await
    .map_err(common::AppError::from)?;

    Ok(())
}
