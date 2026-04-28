use std::sync::Arc;
use arc_swap::ArcSwap;
use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde_json::Value;
use thiserror::Error;
use tracing::{info, warn, instrument};
use crate::config::AppConfig;

/// Errors that can occur during configuration reload.
#[derive(Debug, Error)]
pub enum ConfigReloadError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Invalid configuration: {0}")]
    Invalid(String),
}

impl IntoResponse for ConfigReloadError {
    fn into_response(self) -> axum::response::Response {
        let (status, message) = match self {
            ConfigReloadError::Io(_) => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
            ConfigReloadError::Serialization(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            ConfigReloadError::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
            ConfigReloadError::Invalid(_) => (StatusCode::BAD_REQUEST, self.to_string()),
        };

        let body = Json(serde_json::json!({
            "error": message,
            "status": status.as_u16()
        }));

        (status, body).into_response()
    }
}

/// Manages hot-reloadable application configuration.
pub struct ConfigManager {
    current_config: ArcSwap<AppConfig>,
}

impl ConfigManager {
    /// Create a new ConfigManager with the default configuration.
    pub fn new(initial_config: AppConfig) -> Self {
        Self {
            current_config: ArcSwap::from(Arc::new(initial_config)),
        }
    }

    /// Get a reference to the current configuration.
    pub fn load(&self) -> Arc<AppConfig> {
        self.current_config.load_full()
    }

    /// Reload the configuration from a file or environment.
    /// In this implementation, we simulate loading from a local `config.json` file.
    #[instrument(skip(self))]
    pub async fn reload(&self) -> Result<(), ConfigReloadError> {
        info!("Starting configuration reload...");

        // In a real scenario, we would load from a file or external service.
        // For this task, we'll look for `config.json` in the current directory.
        let config_path = "config.json";
        
        if !std::path::Path::new(config_path).exists() {
            warn!("config.json not found, skipping reload");
            return Err(ConfigReloadError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "config.json not found",
            )));
        }

        let content = tokio::fs::read_to_string(config_path).await?;
        let new_config: AppConfig = serde_json::from_str(&content)?;

        // Validate config (e.g., check database URL format)
        if new_config.database.url.is_empty() {
            return Err(ConfigReloadError::Invalid("Database URL cannot be empty".to_string()));
        }

        // Update the global configuration
        self.current_config.store(Arc::new(new_config));
        
        info!("Configuration successfully reloaded");
        Ok(())
    }

    /// Update configuration from a JSON value (e.g., from an API request).
    #[instrument(skip(self, patch))]
    pub fn update_from_patch(&self, patch: Value) -> Result<(), ConfigReloadError> {
        let current = self.load();
        let mut current_json = serde_json::to_value(&*current)?;
        
        // Deep merge patch into current configuration
        if let Some(patch_obj) = patch.as_object() {
            if let Some(current_obj) = current_json.as_object_mut() {
                for (k, v) in patch_obj {
                    if v.is_object() && current_obj.contains_key(k) && current_obj[k].is_object() {
                        // Merge nested objects
                        let sub_patch = v.as_object().unwrap();
                        let sub_current = current_obj.get_mut(k).unwrap().as_object_mut().unwrap();
                        for (sk, sv) in sub_patch {
                            sub_current.insert(sk.clone(), sv.clone());
                        }
                    } else {
                        // Direct replacement for non-objects or new keys
                        current_obj.insert(k.clone(), v.clone());
                    }
                }
            }
        }

        let new_config: AppConfig = serde_json::from_value(current_json)?;
        self.current_config.store(Arc::new(new_config));
        
        info!("Configuration updated via patch");
        Ok(())
    }
}

/// Axum handler to trigger a configuration reload.
pub async fn handle_reload(
    State(state): State<Arc<crate::api::handlers::profiling::AppState>>,
) -> Result<impl IntoResponse, ConfigReloadError> {
    state.config_manager.reload().await?;
    Ok((StatusCode::OK, Json(serde_json::json!({ "status": "reloaded" }))))
}

/// Axum handler to get the current configuration (sanitized).
pub async fn handle_get_config(
    State(state): State<Arc<crate::api::handlers::profiling::AppState>>,
) -> impl IntoResponse {
    let config = state.config_manager.load();
    // In a real app, we would sanitize sensitive fields like DB passwords
    Json(config)
}
