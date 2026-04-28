use axum::{
    routing::get,
    Router,
};
use std::sync::Arc;
use tower::ServiceExt;
use hyper::{Request, StatusCode};
use backend::api::handlers::profiling::{AppState, get_system_status};
use backend::services::{
    sys_metrics::MetricsExporter,
    error_recovery::ErrorManager,
};

// We need to make modules public or use a common library for this to work perfectly in integration tests.
// For the sake of this task, I'll assume the structure allows for this or adjust as needed.

#[tokio::test]
async fn test_get_status_endpoint() {
    let metrics_exporter = Arc::new(MetricsExporter::new());
    let error_manager = Arc::new(ErrorManager::new());

    let config_manager = Arc::new(backend::config::reload::ConfigManager::new(backend::config::AppConfig::default()));
    let state = Arc::new(AppState {
        metrics_exporter,
        error_manager,
        config_manager,
    });

    let app = Router::new()
        .route("/api/status", get(get_system_status))
        .with_state(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/status")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}
