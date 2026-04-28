use axum::extract::State;
use std::sync::Arc;
use crate::services::{
    sys_metrics::MetricsExporter,
    error_recovery::ErrorManager,
};
use crate::config::reload::ConfigManager;
use crate::api::contracts::{ApiResponse, SystemStatus, ProfileTriggerRequest, ProfileTriggerResponse, ValidatedJson};

pub struct AppState {
    pub metrics_exporter: Arc<MetricsExporter>,
    pub error_manager: Arc<ErrorManager>,
    pub config_manager: Arc<ConfigManager>,
}

pub async fn get_system_status(
    State(state): State<Arc<AppState>>,
) -> ApiResponse<SystemStatus> {
    let metrics = state.metrics_exporter.get_metrics().await;
    let recovery_tasks = state.error_manager.get_active_tasks().await;

    ApiResponse::new(SystemStatus {
        status: "healthy".to_string(),
        uptime_secs: metrics.uptime,
        memory_used_bytes: metrics.memory_usage,
        active_recovery_tasks: recovery_tasks.len(),
    })
}

pub async fn trigger_profile_collection(
    State(_state): State<Arc<AppState>>,
    ValidatedJson(payload): ValidatedJson<ProfileTriggerRequest>,
) -> ApiResponse<ProfileTriggerResponse> {
    // In a real implementation, this would trigger a CPU/Memory profile
    // using the provided payload (duration, sample rate, etc.)
    
    ApiResponse::new(ProfileTriggerResponse {
        profile_id: uuid::Uuid::new_v4(),
        message: format!("Profiling collection triggered for label: {}", payload.label),
        estimated_completion: chrono::Utc::now() + chrono::Duration::seconds(payload.duration_secs as i64),
    })
}
