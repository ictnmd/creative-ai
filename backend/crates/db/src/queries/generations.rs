//! Generation query operations
//!
//! Database operations for the generations table.

use chrono::{DateTime, Utc};
use common::AppResult;
use sqlx::PgPool;
use uuid::Uuid;

/// Generation row returned from the generations table.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct GenerationRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub prompt: String,
    pub enhanced_prompt: Option<String>,
    pub provider: String,
    pub model: String,
    pub style_preset_id: Option<Uuid>,
    pub reference_images: Vec<String>,
    pub sketch_data: Option<String>,
    pub output_urls: Vec<String>,
    pub status: String,
    pub error_message: Option<String>,
    pub credits_used: i32,
    pub quota_used: i32,
    pub visibility: String,
    pub view_count: i32,
    pub created_at: DateTime<Utc>,
}

/// Parameters for creating a new generation record.
#[derive(Debug)]
pub struct CreateGenerationParams {
    pub user_id: Uuid,
    pub prompt: String,
    pub enhanced_prompt: Option<String>,
    pub provider: String,
    pub model: String,
    pub style_preset_id: Option<Uuid>,
    pub reference_images: Vec<String>,
    pub sketch_data: Option<String>,
    pub aspect_ratio: Option<String>,
    pub num_images: i32,
    pub idempotency_key: Uuid,
}

/// Create a new generation record and return its ID.
pub async fn create(
    pool: &PgPool,
    params: &CreateGenerationParams,
) -> AppResult<Uuid> {
    let id: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO generations (
            id, user_id, prompt, enhanced_prompt, provider, model,
            style_preset_id, reference_images, sketch_data,
            credits_used, idempotency_key
        )
        VALUES (
            uuid_generate_v4(), $1, $2, $3, $4, $5,
            $6, $7, $8, $9, $10
        )
        RETURNING id
        "#,
    )
    .bind(params.user_id)
    .bind(&params.prompt)
    .bind(&params.enhanced_prompt)
    .bind(&params.provider)
    .bind(&params.model)
    .bind(params.style_preset_id)
    .bind(&params.reference_images)
    .bind(&params.sketch_data)
    .bind(params.num_images)
    .bind(params.idempotency_key)
    .fetch_one(pool)
    .await
    .map_err(common::AppError::from)?;

    Ok(id)
}

/// Get a generation by ID, ensuring it belongs to the specified user.
pub async fn get(pool: &PgPool, id: Uuid, user_id: Uuid) -> AppResult<Option<GenerationRow>> {
    sqlx::query_as::<_, GenerationRow>(
        r#"
        SELECT id, user_id, prompt, enhanced_prompt, provider, model,
               style_preset_id, reference_images, sketch_data, output_urls,
               status, error_message, credits_used, quota_used,
               visibility, view_count, created_at
        FROM generations
        WHERE id = $1 AND user_id = $2
        "#,
    )
    .bind(id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(common::AppError::from)
}

/// Get a generation by ID without user filter (for workers).
pub async fn get_by_id(pool: &PgPool, id: Uuid) -> AppResult<Option<GenerationRow>> {
    sqlx::query_as::<_, GenerationRow>(
        r#"
        SELECT id, user_id, prompt, enhanced_prompt, provider, model,
               style_preset_id, reference_images, sketch_data, output_urls,
               status, error_message, credits_used, quota_used,
               visibility, view_count, created_at
        FROM generations
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(common::AppError::from)
}

/// List generations for a user with optional filtering and pagination.
pub async fn list(
    pool: &PgPool,
    user_id: Uuid,
    status_filter: Option<&str>,
    provider_filter: Option<&str>,
    limit: i64,
    offset: i64,
) -> AppResult<Vec<GenerationRow>> {
    let rows = sqlx::query_as::<_, GenerationRow>(
        r#"
        SELECT id, user_id, prompt, enhanced_prompt, provider, model,
               style_preset_id, reference_images, sketch_data, output_urls,
               status, error_message, credits_used, quota_used,
               visibility, view_count, created_at
        FROM generations
        WHERE user_id = $1
          AND ($2::text IS NULL OR status = $2)
          AND ($3::text IS NULL OR provider = $3)
        ORDER BY created_at DESC
        LIMIT $4 OFFSET $5
        "#,
    )
    .bind(user_id)
    .bind(status_filter)
    .bind(provider_filter)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
    .map_err(common::AppError::from)?;

    Ok(rows)
}

/// Count total generations for a user with optional filtering.
pub async fn count(
    pool: &PgPool,
    user_id: Uuid,
    status_filter: Option<&str>,
    provider_filter: Option<&str>,
) -> AppResult<i64> {
    let count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM generations
        WHERE user_id = $1
          AND ($2::text IS NULL OR status = $2)
          AND ($3::text IS NULL OR provider = $3)
        "#,
    )
    .bind(user_id)
    .bind(status_filter)
    .bind(provider_filter)
    .fetch_one(pool)
    .await
    .map_err(common::AppError::from)?;

    Ok(count)
}

/// Update the status and output URLs of a generation.
pub async fn update_status(
    pool: &PgPool,
    id: Uuid,
    status: &str,
    output_urls: Option<&[String]>,
) -> AppResult<()> {
    sqlx::query(
        r#"
        UPDATE generations
        SET status = $2,
            output_urls = COALESCE($3, output_urls)
        WHERE id = $1
        "#,
    )
    .bind(id)
    .bind(status)
    .bind(output_urls)
    .execute(pool)
    .await
    .map_err(common::AppError::from)?;

    Ok(())
}

/// Set the error message on a generation (used when a job fails).
pub async fn set_error(pool: &PgPool, id: Uuid, error_message: &str) -> AppResult<()> {
    sqlx::query(
        r#"
        UPDATE generations
        SET status = 'failed', error_message = $2
        WHERE id = $1
        "#,
    )
    .bind(id)
    .bind(error_message)
    .execute(pool)
    .await
    .map_err(common::AppError::from)?;

    Ok(())
}

/// Delete a generation, ensuring it belongs to the user.
pub async fn delete(pool: &PgPool, id: Uuid, user_id: Uuid) -> AppResult<bool> {
    let result = sqlx::query(
        r#"
        DELETE FROM generations WHERE id = $1 AND user_id = $2
        "#,
    )
    .bind(id)
    .bind(user_id)
    .execute(pool)
    .await
    .map_err(common::AppError::from)?;

    Ok(result.rows_affected() > 0)
}
