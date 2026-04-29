//! Feature flag service with Redis caching and PostgreSQL persistence.
//!
//! This module provides a production-ready feature flag system that:
//! - Stores flag state in PostgreSQL for durability
//! - Caches flag values in Redis for low-latency reads
//! - Supports cache invalidation on updates
//! - Provides async API for flag evaluation
//!
//! # Example
//! ```rust,no_run
//! use backend::services::feature_flags::FeatureFlagService;
//! use sqlx::PgPool;
//! use redis::Client;
//!
//! # async fn example(pool: PgPool, redis: Client) -> anyhow::Result<()> {
//! let service = FeatureFlagService::new(pool, redis);
//! let enabled = service.is_enabled("new_dashboard").await?;
//! if enabled {
//!     // render new UI
//! }
//! # Ok(())
//! # }
//! ```

#![allow(dead_code)]

use chrono::{DateTime, Utc};
use redis::{AsyncCommands, Client as RedisClient};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use thiserror::Error;
use tracing::{debug, info, warn};

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors that can occur in the feature flag service.
#[derive(Debug, Error)]
pub enum FlagError {
    /// A database error occurred.
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    /// A Redis error occurred.
    #[error("Redis error: {0}")]
    Redis(#[from] redis::RedisError),

    /// The requested flag was not found.
    #[error("Feature flag not found: {0}")]
    NotFound(String),

    /// An internal error occurred.
    #[error("Internal error: {0}")]
    Internal(String),
}

// ---------------------------------------------------------------------------
// Domain types
// ---------------------------------------------------------------------------

/// A feature flag record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureFlag {
    /// Unique key identifying the flag.
    pub key: String,
    /// Whether the flag is enabled.
    pub enabled: bool,
    /// Human-readable description.
    pub description: String,
    /// Last update timestamp.
    pub updated_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// FeatureFlagService
// ---------------------------------------------------------------------------

/// Service for managing feature flags with Redis caching and PostgreSQL persistence.
pub struct FeatureFlagService {
    db: PgPool,
    redis: RedisClient,
}

impl FeatureFlagService {
    /// Create a new feature flag service.
    ///
    /// # Arguments
    /// - `db`: PostgreSQL connection pool
    /// - `redis`: Redis client
    pub fn new(db: PgPool, redis: RedisClient) -> Self {
        Self { db, redis }
    }

    /// Check if a feature flag is enabled.
    ///
    /// This method first checks Redis cache. On cache miss, it queries
    /// PostgreSQL and populates the cache with a 5-minute TTL.
    ///
    /// # Errors
    /// Returns [`FlagError::NotFound`] if the flag doesn't exist.
    pub async fn is_enabled(&self, key: &str) -> Result<bool, FlagError> {
        let cache_key = format!("flag:{key}");

        // Try cache first
        let mut conn = self.redis.get_multiplexed_async_connection().await?;
        let cached: Option<String> = conn.get(&cache_key).await?;

        if let Some(val) = cached {
            debug!(key = %key, cached = %val, "Feature flag cache hit");
            return Ok(val == "1");
        }

        // Cache miss – query database
        debug!(key = %key, "Feature flag cache miss – querying database");
        let row: Option<(bool,)> =
            sqlx::query_as("SELECT enabled FROM feature_flags WHERE key = $1")
                .bind(key)
                .fetch_optional(&self.db)
                .await?;

        match row {
            Some((enabled,)) => {
                // Populate cache with 5-minute TTL
                let val = if enabled { "1" } else { "0" };
                let _: () = conn.set_ex(&cache_key, val, 300).await?;
                debug!(key = %key, enabled = enabled, "Cached feature flag");
                Ok(enabled)
            }
            None => Err(FlagError::NotFound(key.to_string())),
        }
    }

    /// Get the full feature flag record.
    ///
    /// # Errors
    /// Returns [`FlagError::NotFound`] if the flag doesn't exist.
    pub async fn get(&self, key: &str) -> Result<FeatureFlag, FlagError> {
        let row: Option<(String, bool, String, DateTime<Utc>)> = sqlx::query_as(
            "SELECT key, enabled, description, updated_at FROM feature_flags WHERE key = $1",
        )
        .bind(key)
        .fetch_optional(&self.db)
        .await?;

        match row {
            Some((key, enabled, description, updated_at)) => Ok(FeatureFlag {
                key,
                enabled,
                description,
                updated_at,
            }),
            None => Err(FlagError::NotFound(key.to_string())),
        }
    }

    /// List all feature flags.
    pub async fn list(&self) -> Result<Vec<FeatureFlag>, FlagError> {
        let rows: Vec<(String, bool, String, DateTime<Utc>)> = sqlx::query_as(
            "SELECT key, enabled, description, updated_at FROM feature_flags ORDER BY key",
        )
        .fetch_all(&self.db)
        .await?;

        Ok(rows
            .into_iter()
            .map(|(key, enabled, description, updated_at)| FeatureFlag {
                key,
                enabled,
                description,
                updated_at,
            })
            .collect())
    }

    /// Create or update a feature flag.
    ///
    /// This method upserts the flag in PostgreSQL and invalidates the cache.
    pub async fn set(&self, key: &str, enabled: bool, description: &str) -> Result<(), FlagError> {
        sqlx::query(
            r#"
            INSERT INTO feature_flags (key, enabled, description, updated_at)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (key) DO UPDATE
            SET enabled = EXCLUDED.enabled,
                description = EXCLUDED.description,
                updated_at = EXCLUDED.updated_at
            "#,
        )
        .bind(key)
        .bind(enabled)
        .bind(description)
        .bind(Utc::now())
        .execute(&self.db)
        .await?;

        // Invalidate cache
        self.invalidate_cache(key).await?;

        info!(key = %key, enabled = enabled, "Feature flag updated");
        Ok(())
    }

    /// Delete a feature flag.
    ///
    /// # Errors
    /// Returns [`FlagError::NotFound`] if the flag doesn't exist.
    pub async fn delete(&self, key: &str) -> Result<(), FlagError> {
        let result = sqlx::query("DELETE FROM feature_flags WHERE key = $1")
            .bind(key)
            .execute(&self.db)
            .await?;

        if result.rows_affected() == 0 {
            return Err(FlagError::NotFound(key.to_string()));
        }

        self.invalidate_cache(key).await?;
        info!(key = %key, "Feature flag deleted");
        Ok(())
    }

    /// Invalidate the Redis cache for a specific flag.
    async fn invalidate_cache(&self, key: &str) -> Result<(), FlagError> {
        let cache_key = format!("flag:{key}");
        let mut conn = self.redis.get_multiplexed_async_connection().await?;
        let deleted: i32 = conn.del(&cache_key).await?;
        if deleted > 0 {
            debug!(key = %key, "Invalidated feature flag cache");
        } else {
            warn!(key = %key, "Cache key not found during invalidation");
        }
        Ok(())
    }

    /// Flush all feature flag cache entries (useful for testing / maintenance).
    ///
    /// This uses a Redis SCAN to find all keys matching `flag:*` and deletes them.
    pub async fn flush_cache(&self) -> Result<usize, FlagError> {
        let mut conn = self.redis.get_multiplexed_async_connection().await?;
        let keys: Vec<String> = redis::cmd("KEYS")
            .arg("flag:*")
            .query_async(&mut conn)
            .await?;

        if keys.is_empty() {
            debug!("No feature flag cache entries to flush");
            return Ok(0);
        }

        let count = keys.len();
        for key in keys {
            let _: () = conn.del(&key).await?;
        }

        info!(count = count, "Flushed feature flag cache");
        Ok(count)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // Unit tests that do not require live database/Redis connections.

    #[test]
    fn test_flag_error_display() {
        let err = FlagError::NotFound("test_flag".to_string());
        assert!(err.to_string().contains("test_flag"));
    }

    #[test]
    fn test_feature_flag_serialization() {
        let flag = FeatureFlag {
            key: "test".to_string(),
            enabled: true,
            description: "Test flag".to_string(),
            updated_at: Utc::now(),
        };
        let json = serde_json::to_string(&flag).unwrap();
        assert!(json.contains("test"));
        assert!(json.contains("true"));
    }

    #[test]
    fn test_feature_flag_deserialization() {
        let json = r#"{
            "key": "beta",
            "enabled": false,
            "description": "Beta features",
            "updated_at": "2024-01-01T00:00:00Z"
        }"#;
        let flag: FeatureFlag = serde_json::from_str(json).unwrap();
        assert_eq!(flag.key, "beta");
        assert!(!flag.enabled);
    }
}
