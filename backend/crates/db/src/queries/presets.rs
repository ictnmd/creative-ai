//! Style preset query operations (renamed to presets for clarity)

use chrono::{DateTime, Utc};
use common::AppResult;
use sqlx::PgPool;
use uuid::Uuid;

/// Preset row returned from the style_presets table.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PresetRow {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub prompt_suffix: String,
    pub thumbnail_url: Option<String>,
    pub is_public: bool,
    pub is_builtin: bool,
    pub creator_id: Option<Uuid>,
    pub tags: Vec<String>,
    pub usage_count: i32,
    pub created_at: DateTime<Utc>,
}

/// List presets accessible to a user.
/// Includes builtin presets (is_builtin = true) and the user's own custom presets.
pub async fn list_presets(pool: &PgPool, user_id: Uuid) -> AppResult<Vec<PresetRow>> {
    let rows = sqlx::query_as::<_, PresetRow>(
        r#"
        SELECT id, name, description, prompt_suffix, thumbnail_url,
               is_public, is_builtin, creator_id, tags, usage_count, created_at
        FROM style_presets
        WHERE is_builtin = TRUE
           OR (is_builtin = FALSE AND creator_id = $1)
           OR (is_public = TRUE AND creator_id = $1)
        ORDER BY is_builtin DESC, usage_count DESC, name ASC
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(common::AppError::from)?;

    Ok(rows)
}

/// Get a preset by its ID.
pub async fn get_preset(pool: &PgPool, preset_id: Uuid) -> AppResult<Option<PresetRow>> {
    sqlx::query_as::<_, PresetRow>(
        r#"
        SELECT id, name, description, prompt_suffix, thumbnail_url,
               is_public, is_builtin, creator_id, tags, usage_count, created_at
        FROM style_presets
        WHERE id = $1
        "#,
    )
    .bind(preset_id)
    .fetch_optional(pool)
    .await
    .map_err(common::AppError::from)
}

/// Create a custom preset.
pub async fn create_preset(
    pool: &PgPool,
    user_id: Uuid,
    name: &str,
    description: Option<&str>,
    prompt_suffix: &str,
    thumbnail_url: Option<&str>,
    tags: &[String],
) -> AppResult<Uuid> {
    let id: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO style_presets (id, name, description, prompt_suffix, thumbnail_url, is_public, is_builtin, creator_id, tags)
        VALUES (uuid_generate_v4(), $1, $2, $3, $4, FALSE, FALSE, $5, $6)
        RETURNING id
        "#,
    )
    .bind(name)
    .bind(description)
    .bind(prompt_suffix)
    .bind(thumbnail_url)
    .bind(user_id)
    .bind(tags)
    .fetch_one(pool)
    .await
    .map_err(common::AppError::from)?;

    Ok(id)
}

/// Update a preset. Only the owner can update it (and not builtin presets).
/// Returns true if the update was successful.
pub async fn update_preset(
    pool: &PgPool,
    preset_id: Uuid,
    user_id: Uuid,
    name: Option<&str>,
    description: Option<&str>,
    prompt_suffix: Option<&str>,
    thumbnail_url: Option<&str>,
    tags: Option<&[String]>,
) -> AppResult<bool> {
    let result = sqlx::query(
        r#"
        UPDATE style_presets
        SET name = COALESCE($3, name),
            description = COALESCE($4, description),
            prompt_suffix = COALESCE($5, prompt_suffix),
            thumbnail_url = COALESCE($6, thumbnail_url),
            tags = COALESCE($7, tags)
        WHERE id = $1 AND creator_id = $2 AND is_builtin = FALSE
        "#,
    )
    .bind(preset_id)
    .bind(user_id)
    .bind(name)
    .bind(description)
    .bind(prompt_suffix)
    .bind(thumbnail_url)
    .bind(tags)
    .execute(pool)
    .await
    .map_err(common::AppError::from)?;

    Ok(result.rows_affected() > 0)
}

/// Delete a preset. Only the owner can delete it (and not builtin presets).
/// Returns true if the deletion was successful.
pub async fn delete_preset(pool: &PgPool, preset_id: Uuid, user_id: Uuid) -> AppResult<bool> {
    let result = sqlx::query(
        r#"
        DELETE FROM style_presets
        WHERE id = $1 AND creator_id = $2 AND is_builtin = FALSE
        "#,
    )
    .bind(preset_id)
    .bind(user_id)
    .execute(pool)
    .await
    .map_err(common::AppError::from)?;

    Ok(result.rows_affected() > 0)
}

/// Increment the usage count of a preset.
pub async fn increment_usage_count(pool: &PgPool, preset_id: Uuid) -> AppResult<()> {
    sqlx::query(
        r#"
        UPDATE style_presets SET usage_count = usage_count + 1 WHERE id = $1
        "#,
    )
    .bind(preset_id)
    .execute(pool)
    .await
    .map_err(common::AppError::from)?;

    Ok(())
}
