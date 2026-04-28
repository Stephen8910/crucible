mod api;
mod services;
mod config;

use std::sync::Arc;
use axum::{
    routing::{get, post},
    Router,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use crate::api::handlers::profiling::{AppState, get_system_status, trigger_profile_collection};
use crate::services::{
    sys_metrics::MetricsExporter,
    error_recovery::ErrorManager,
    log_aggregator::LogAggregator,
};
use crate::config::{AppConfig, reload::{ConfigManager, handle_reload, handle_get_config}};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "backend=debug,tower_http=debug".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Initialize services
    let metrics_exporter = Arc::new(MetricsExporter::new());
    let error_manager = Arc::new(ErrorManager::new());
    let (_log_aggregator, log_receiver) = LogAggregator::new();

    // Spawn background workers
    tokio::spawn(MetricsExporter::run_collector(metrics_exporter.clone()));
    tokio::spawn(LogAggregator::run_worker(log_receiver));
    
    // Initialize config manager
    let config = AppConfig::default();
    let config_manager = Arc::new(ConfigManager::new(config));

    let state = Arc::new(AppState {
        metrics_exporter,
        error_manager,
        config_manager: config_manager.clone(),
    });

    // Build router
    let app = Router::new()
        .route("/api/status", get(get_system_status))
        .route("/api/profile", post(trigger_profile_collection))
        .route("/api/config", get(handle_get_config))
        .route("/api/config/reload", post(handle_reload))
        .with_state(state);

    // Run server
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    tracing::info!("Server listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await?;

    Ok(())
}
