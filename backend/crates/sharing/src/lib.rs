//! # Sharing Crate
//!
//! Share link generation, public profile views, and visibility management
//! for the Creative AI Studio backend.

use chrono::{DateTime, Utc};
use common::{AppError, AppResult};
use nanoid::nanoid;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// View types
// ---------------------------------------------------------------------------

/// Shared generation view returned when accessing via share token.
/// Includes generation details plus creator info.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedGenerationView {
    pub generation_id: Uuid,
    pub share_token: String,
    pub prompt: String,
    pub enhanced_prompt: Option<String>,
    pub output_urls: Vec<String>,
    pub provider: String,
    pub model: String,
    pub style_preset_name: Option<String>,
    pub view_count: i32,
    pub created_at: DateTime<Utc>,
    pub creator_username: String,
    pub creator_avatar_url: Option<String>,
}

/// Public profile view with user summary and public generations list.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicProfileView {
    pub user_id: Uuid,
    pub username: String,
    pub name: String,
    pub avatar_url: Option<String>,
    pub bio: Option<String>,
    pub follower_count: i64,
    pub following_count: i64,
    pub generation_count: i64,
    pub public_generations: Vec<PublicGenerationSummary>,
}

/// Summary of a single generation for public profile display.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicGenerationSummary {
    pub id: Uuid,
    pub share_token: String,
    pub prompt: String,
    pub thumbnail_url: Option<String>,
    pub view_count: i32,
    pub created_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Share link creation
// ---------------------------------------------------------------------------

/// Create a share link for a generation.
///
/// Creates or updates a SharedGeneration record with a unique nanoid share token.
/// Also updates the generation's visibility field.
/// Returns the share token.
pub async fn create_share_link(
    pool: &PgPool,
    generation_id: Uuid,
    is_public: bool,
) -> AppResult<String> {
    // Generate a unique URL-safe nanoid token (12 chars).
    let share_token = nanoid!(12);

    let visibility = if is_public { "public" } else { "shared" };

    // Use upsert to create or update the shared_generation record.
    sqlx::query(
        r#"
        INSERT INTO shared_generations (id, generation_id, share_token, is_public)
        VALUES (uuid_generate_v4(), $1, $2, $3)
        ON CONFLICT (generation_id) DO UPDATE SET
            share_token = EXCLUDED.share_token,
            is_public = EXCLUDED.is_public
        "#,
    )
    .bind(generation_id)
    .bind(&share_token)
    .bind(is_public)
    .execute(pool)
    .await
    .map_err(AppError::from)?;

    // Update the generation's visibility.
    sqlx::query(
        r#"
        UPDATE generations SET visibility = $2 WHERE id = $1
        "#,
    )
    .bind(generation_id)
    .bind(visibility)
    .execute(pool)
    .await
    .map_err(AppError::from)?;

    Ok(share_token)
}

// ---------------------------------------------------------------------------
// Share link deletion
// ---------------------------------------------------------------------------

/// Delete a share link for a generation.
///
/// Removes the SharedGeneration record and sets the generation visibility to private.
pub async fn delete_share_link(pool: &PgPool, generation_id: Uuid) -> AppResult<()> {
    sqlx::query(
        r#"
        DELETE FROM shared_generations WHERE generation_id = $1
        "#,
    )
    .bind(generation_id)
    .execute(pool)
    .await
    .map_err(AppError::from)?;

    sqlx::query(
        r#"
        UPDATE generations SET visibility = 'private' WHERE id = $1
        "#,
    )
    .bind(generation_id)
    .execute(pool)
    .await
    .map_err(AppError::from)?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Shared generation view
// ---------------------------------------------------------------------------

/// Get a shared generation by its share token.
///
/// Returns generation details plus creator username and avatar.
/// Only returns results for generations that have a share token record.
pub async fn get_shared_generation(
    pool: &PgPool,
    token: &str,
) -> AppResult<Option<SharedGenerationView>> {
    let row = sqlx::query_as::<_, SharedGenerationJoinedRow>(
        r#"
        SELECT
            g.id                            AS generation_id,
            sg.share_token,
            g.prompt,
            g.enhanced_prompt,
            g.output_urls,
            g.provider,
            g.model,
            sp.name                         AS style_preset_name,
            g.view_count,
            g.created_at,
            u.username                      AS creator_username,
            u.avatar_url                    AS creator_avatar_url
        FROM shared_generations sg
        JOIN generations g ON g.id = sg.generation_id
        JOIN users u ON u.id = g.user_id
        LEFT JOIN style_presets sp ON sp.id = g.style_preset_id
        WHERE sg.share_token = $1
          AND g.status = 'completed'
        "#,
    )
    .bind(token)
    .fetch_optional(pool)
    .await
    .map_err(AppError::from)?;

    Ok(row.map(|r| SharedGenerationView {
        generation_id: r.generation_id,
        share_token: r.share_token,
        prompt: r.prompt,
        enhanced_prompt: r.enhanced_prompt,
        output_urls: r.output_urls,
        provider: r.provider,
        model: r.model,
        style_preset_name: r.style_preset_name,
        view_count: r.view_count,
        created_at: r.created_at,
        creator_username: r.creator_username,
        creator_avatar_url: r.creator_avatar_url,
    }))
}

// ---------------------------------------------------------------------------
// Public profile view
// ---------------------------------------------------------------------------

/// Get a user's public profile with their public generations.
///
/// Returns user summary (follower/following counts, bio) and a list of
/// public generations (those with visibility = 'public').
pub async fn get_public_profile(pool: &PgPool, username: &str) -> AppResult<Option<PublicProfileView>> {
    // First look up the user and their profile.
    let user_row = sqlx::query_as::<_, UserBasicRow>(
        r#"
        SELECT u.id, u.username, u.name, u.avatar_url, up.bio
        FROM users u
        LEFT JOIN user_profiles up ON up.user_id = u.id
        WHERE u.username = $1
        "#,
    )
    .bind(username)
    .fetch_optional(pool)
    .await
    .map_err(AppError::from)?;

    let user_row = match user_row {
        Some(r) => r,
        None => return Ok(None),
    };

    // Check if profile is public. If the user has no profile record,
    // treat them as having a private profile (don't expose).
    let is_public_profile = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT COALESCE(is_public_profile, FALSE)
        FROM user_profiles
        WHERE user_id = $1
        "#,
    )
    .bind(user_row.id)
    .fetch_optional(pool)
    .await
    .map_err(AppError::from)?
    .unwrap_or(false);

    if !is_public_profile {
        return Ok(None);
    }

    // Get follower count.
    let follower_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) FROM follows WHERE following_id = $1
        "#,
    )
    .bind(user_row.id)
    .fetch_one(pool)
    .await
    .map_err(AppError::from)?;

    // Get following count.
    let following_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) FROM follows WHERE follower_id = $1
        "#,
    )
    .bind(user_row.id)
    .fetch_one(pool)
    .await
    .map_err(AppError::from)?;

    // Get public generation count.
    let generation_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM generations g
        JOIN shared_generations sg ON sg.generation_id = g.id
        WHERE g.user_id = $1
          AND g.status = 'completed'
          AND sg.is_public = TRUE
        "#,
    )
    .bind(user_row.id)
    .fetch_one(pool)
    .await
    .map_err(AppError::from)?;

    // Get public generations list.
    let pub_gens = sqlx::query_as::<_, PublicGenerationRow>(
        r#"
        SELECT
            g.id,
            sg.share_token,
            g.prompt,
            (g.output_urls::text[])[1]     AS thumbnail_url,
            g.view_count,
            g.created_at
        FROM generations g
        JOIN shared_generations sg ON sg.generation_id = g.id
        WHERE g.user_id = $1
          AND g.status = 'completed'
          AND sg.is_public = TRUE
        ORDER BY g.created_at DESC
        LIMIT 50
        "#,
    )
    .bind(user_row.id)
    .fetch_all(pool)
    .await
    .map_err(AppError::from)?;

    let public_generations: Vec<PublicGenerationSummary> = pub_gens
        .into_iter()
        .map(|r| PublicGenerationSummary {
            id: r.id,
            share_token: r.share_token,
            prompt: r.prompt,
            thumbnail_url: r.thumbnail_url,
            view_count: r.view_count,
            created_at: r.created_at,
        })
        .collect();

    Ok(Some(PublicProfileView {
        user_id: user_row.id,
        username: user_row.username,
        name: user_row.name.unwrap_or_default(),
        avatar_url: user_row.avatar_url,
        bio: user_row.bio,
        follower_count,
        following_count,
        generation_count,
        public_generations,
    }))
}

// ---------------------------------------------------------------------------
// View count increment
// ---------------------------------------------------------------------------

/// Increment the view count of a generation.
pub async fn increment_view_count(pool: &PgPool, generation_id: Uuid) -> AppResult<()> {
    sqlx::query(
        r#"
        UPDATE generations SET view_count = view_count + 1 WHERE id = $1
        "#,
    )
    .bind(generation_id)
    .execute(pool)
    .await
    .map_err(AppError::from)?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Visibility toggle
// ---------------------------------------------------------------------------

/// Toggle the visibility of a generation and update its share link accordingly.
///
/// - `visibility = "private"`  -> removes SharedGeneration record, sets visibility = 'private'
/// - `visibility = "shared"`   -> upserts SharedGeneration with is_public = false
/// - `visibility = "public"`    -> upserts SharedGeneration with is_public = true
pub async fn toggle_visibility(
    pool: &PgPool,
    generation_id: Uuid,
    visibility: &str,
) -> AppResult<()> {
    match visibility {
        "private" => {
            sqlx::query(
                r#"
                DELETE FROM shared_generations WHERE generation_id = $1
                "#,
            )
            .bind(generation_id)
            .execute(pool)
            .await
            .map_err(AppError::from)?;

            sqlx::query(
                r#"
                UPDATE generations SET visibility = 'private' WHERE id = $1
                "#,
            )
            .bind(generation_id)
            .execute(pool)
            .await
            .map_err(AppError::from)?;
        }
        "shared" | "public" => {
            let is_public = visibility == "public";
            let share_token = nanoid!(12);

            sqlx::query(
                r#"
                INSERT INTO shared_generations (id, generation_id, share_token, is_public)
                VALUES (uuid_generate_v4(), $1, $2, $3)
                ON CONFLICT (generation_id) DO UPDATE SET
                    share_token = EXCLUDED.share_token,
                    is_public = EXCLUDED.is_public
                "#,
            )
            .bind(generation_id)
            .bind(&share_token)
            .bind(is_public)
            .execute(pool)
            .await
            .map_err(AppError::from)?;

            sqlx::query(
                r#"
                UPDATE generations SET visibility = $2 WHERE id = $1
                "#,
            )
            .bind(generation_id)
            .bind(visibility)
            .execute(pool)
            .await
            .map_err(AppError::from)?;
        }
        _ => {
            return Err(AppError::Validation(
                "visibility must be 'private', 'shared', or 'public'".to_string(),
            ));
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Internal row types (sqlx fetched rows)
// ---------------------------------------------------------------------------

/// Row type for the shared generation + creator join query.
#[derive(Debug, Clone, sqlx::FromRow)]
struct SharedGenerationJoinedRow {
    generation_id: Uuid,
    share_token: String,
    prompt: String,
    enhanced_prompt: Option<String>,
    output_urls: Vec<String>,
    provider: String,
    model: String,
    style_preset_name: Option<String>,
    view_count: i32,
    created_at: DateTime<Utc>,
    creator_username: String,
    creator_avatar_url: Option<String>,
}

/// Basic user row for profile lookups.
#[derive(Debug, Clone, sqlx::FromRow)]
struct UserBasicRow {
    id: Uuid,
    username: String,
    name: Option<String>,
    avatar_url: Option<String>,
    bio: Option<String>,
}

/// Row type for public generation summaries.
#[derive(Debug, Clone, sqlx::FromRow)]
struct PublicGenerationRow {
    id: Uuid,
    share_token: String,
    prompt: String,
    thumbnail_url: Option<String>,
    view_count: i32,
    created_at: DateTime<Utc>,
}
