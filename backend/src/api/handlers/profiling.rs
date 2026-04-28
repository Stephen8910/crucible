use axum::{
    extract::State,
    Json,
    response::IntoResponse,
};
use serde_json::json;
use std::sync::Arc;
use crate::services::{
    sys_metrics::MetricsExporter,
    error_recovery::ErrorManager,
};
use crate::config::reload::ConfigManager;

pub struct AppState {
    pub metrics_exporter: Arc<MetricsExporter>,
    pub error_manager: Arc<ErrorManager>,
    pub config_manager: Arc<ConfigManager>,
}

pub async fn get_system_status(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let metrics = state.metrics_exporter.get_metrics().await;
    let recovery_tasks = state.error_manager.get_active_tasks().await;

    Json(json!({
        "status": "healthy",
        "metrics": metrics,
        "active_recovery_tasks": recovery_tasks,
    }))
}

pub async fn trigger_profile_collection(
    State(_state): State<Arc<AppState>>,
) -> impl IntoResponse {
    // In a real implementation, this would trigger a CPU/Memory profile
    Json(json!({
        "message": "Profiling collection triggered",
        "profile_id": uuid::Uuid::new_v4().to_string(),
    }))
}
