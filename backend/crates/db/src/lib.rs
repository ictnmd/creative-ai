//! # DB Crate
//!
//! Database access layer providing PostgreSQL and Redis/DragonflyDB connections
//! and repositories for data access.

use common::AppResult;
use once_cell::sync::OnceCell;
use parking_lot::RwLock;
use redis::aio::ConnectionManager;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::sync::Arc;

/// Global PostgreSQL connection pool.
static PG_POOL: OnceCell<PgPool> = OnceCell::new();

/// Global Redis connection manager.
static REDIS_MANAGER: OnceCell<Arc<RwLock<ConnectionManager>>> = OnceCell::new();

/// Initialize the PostgreSQL connection pool.
pub async fn init_pg_pool(database_url: &str) -> AppResult<PgPool> {
    let pool = PgPoolOptions::new()
        .max_connections(20)
        .connect(database_url)
        .await?;

    PG_POOL.set(pool.clone()).map_err(|_| {
        common::AppError::Config("PostgreSQL pool already initialized".to_string())
    })?;

    Ok(pool)
}

/// Get the global PostgreSQL pool.
pub fn pg_pool() -> AppResult<PgPool> {
    PG_POOL.get().cloned().ok_or_else(|| {
        common::AppError::Config("PostgreSQL pool not initialized".to_string())
    })
}

/// Initialize the Redis/DragonflyDB connection manager.
pub async fn init_redis(redis_url: &str) -> AppResult<()> {
    let client = redis::Client::open(redis_url)?;
    let manager = ConnectionManager::new(client).await?;

    REDIS_MANAGER.set(Arc::new(RwLock::new(manager))).map_err(|_| {
        common::AppError::Config("Redis manager already initialized".to_string())
    })?;

    Ok(())
}

/// Get the global Redis connection manager.
pub fn redis_manager() -> AppResult<Arc<RwLock<ConnectionManager>>> {
    REDIS_MANAGER.get().cloned().ok_or_else(|| {
        common::AppError::Config("Redis manager not initialized".to_string())
    })
}

/// Repository trait for CRUD operations.
#[async_trait::async_trait]
pub trait Repository<T, ID> {
    async fn find_by_id(&self, id: ID) -> AppResult<Option<T>>;
    async fn find_all(&self) -> AppResult<Vec<T>>;
    async fn create(&self, entity: &T) -> AppResult<T>;
    async fn update(&self, entity: &T) -> AppResult<T>;
    async fn delete(&self, id: ID) -> AppResult<()>;
}
