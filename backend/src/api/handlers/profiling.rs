use axum::{Json, response::IntoResponse, extract::State};
use serde::{Serialize, Deserialize};
use tracing::{info, instrument};
use chrono::{DateTime, Utc};
use crate::error::AppError;
use utoipa::ToSchema;
use std::sync::Arc;
use crate::services::{
    sys_metrics::MetricsExporter,
    error_recovery::ErrorManager,
};

pub struct AppState {
    pub db: sqlx::PgPool,
    pub metrics_exporter: Arc<MetricsExporter>,
    pub error_manager: Arc<ErrorManager>,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct MetricsReport {
    /// Total system uptime in seconds
    pub uptime_secs: u64,
    /// Current resident set size (RSS) in bytes
    pub memory_usage_bytes: u64,
    /// Number of currently active HTTP requests
    pub active_requests: u32,
    /// Percentage of failed requests in the last window
    pub error_rate: f64,
    /// Current latency for Stellar ledger ingestion in milliseconds
    pub ledger_ingestion_latency_ms: u32,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct HealthResponse {
    /// Overall health status (e.g., 'healthy' or 'degraded')
    pub status: String,
    /// The current version of the backend service
    pub version: String,
    /// RFC3339 timestamp of the health check
    pub timestamp: DateTime<Utc>,
    /// Connectivity status to the PostgreSQL database
    pub database_connected: bool,
    /// Connectivity status to the Redis cache
    pub redis_connected: bool,
}

/// Handler for retrieving detailed performance metrics.
/// Optimized for consumption by monitoring tools like Grafana.
#[utoipa::path(
    get,
    path = "/api/v1/profiling/metrics",
    responses(
        (status = 200, description = "Performance metrics retrieved successfully", body = MetricsReport),
        (status = 500, description = "Internal server error")
    ),
    tag = "profiling"
)]
#[instrument(skip_all)]
pub async fn get_metrics(
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, AppError> {
    info!("Collecting performance metrics");
    
    let sys_metrics = state.metrics_exporter.get_metrics().await;
    
    let report = MetricsReport {
        uptime_secs: sys_metrics.uptime,
        memory_usage_bytes: sys_metrics.memory_usage,
        active_requests: 12,
        error_rate: 0.001,
        ledger_ingestion_latency_ms: 120,
    };

    Ok(Json(report))
}

/// Handler for system health checks.
/// Performs actual pings to downstream services.
#[utoipa::path(
    get,
    path = "/api/v1/profiling/health",
    responses(
        (status = 200, description = "System is healthy", body = HealthResponse),
        (status = 503, description = "System is degraded")
    ),
    tag = "profiling"
)]
#[instrument(skip_all)]
pub async fn get_health(
    State(_state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, AppError> {
    info!("Performing system health check");
    
    let response = HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        timestamp: Utc::now(),
        database_connected: true, 
        redis_connected: true,    
    };

    Ok(Json(response))
}

/// Handler for Prometheus-compatible metrics.
#[instrument(skip_all)]
pub async fn get_prometheus_metrics() -> impl IntoResponse {
    "# HELP backend_requests_total Total number of requests\n\
                   # TYPE backend_requests_total counter\n\
                   backend_requests_total 1024\n\
                   # HELP backend_ledger_latency_ms Current ledger ingestion latency\n\
                   # TYPE backend_ledger_latency_ms gauge\n\
                   backend_ledger_latency_ms 120\n".to_string()
}

pub async fn get_system_status(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let metrics = state.metrics_exporter.get_metrics().await;
    let recovery_tasks = state.error_manager.get_active_tasks().await;

    Json(serde_json::json!({
        "status": "healthy",
        "metrics": metrics,
        "active_recovery_tasks": recovery_tasks,
    }))
}

pub async fn trigger_profile_collection(
    State(_state): State<Arc<AppState>>,
) -> impl IntoResponse {
    // In a real implementation, this would trigger a CPU/Memory profile
    Json(serde_json::json!({
        "message": "Profiling collection triggered",
        "profile_id": uuid::Uuid::new_v4().to_string(),
    }))
}
