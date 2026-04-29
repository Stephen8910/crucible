use apalis::prelude::*;
use apalis_redis::RedisStorage;
use axum::{
    routing::{get, post},
    Router,
};
use backend::api::handlers::dashboard::{get_dashboard, DashboardState};
use backend::{
    api::handlers::{profiling, stellar},
    config::Config,
    jobs::{monitor_transaction, TransactionMonitorJob},
    services::{
        error_recovery::ErrorManager, log_aggregator::LogAggregator, log_alerts::AlertManager,
        sys_metrics::MetricsExporter,
    },
    telemetry::init_telemetry,
};
use profiling::AppState;
use redis::aio::ConnectionManager;
use sqlx::postgres::PgPoolOptions;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::signal;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Load configuration
    let config = Config::from_env()?;

    // Initialize observability (fmt tracing subscriber)
    init_telemetry();

    // Database setup
    let db_pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&config.database_url)
        .await?;

    tracing::info!("Database connection established");

    // Initialize services
    let metrics_exporter = Arc::new(MetricsExporter::new());
    let error_manager = Arc::new(ErrorManager::new());
    let alert_manager = Arc::new(AlertManager::new());
    let (_log_aggregator, log_receiver) = LogAggregator::new();

    tokio::spawn(MetricsExporter::run_collector(metrics_exporter.clone()));
    tokio::spawn(LogAggregator::run_worker(log_receiver));

    // Redis + job queue setup
    let redis_client = redis::Client::open(config.redis_url.clone())?;
    let conn = ConnectionManager::new(redis_client.clone()).await?;
    let storage: RedisStorage<TransactionMonitorJob> = RedisStorage::new(conn);

    let worker = WorkerBuilder::new("monitor-worker")
        .backend(storage)
        .build_fn(monitor_transaction);

    // Shared state for profiling/status routes
    let profiling_state = Arc::new(AppState {
        db: Some(db_pool),
        metrics_exporter: metrics_exporter.clone(),
        error_manager: error_manager.clone(),
    });

    // Shared state for dashboard route
    let dashboard_state = Arc::new(DashboardState {
        metrics_exporter,
        error_manager,
        alert_manager,
        redis: redis_client,
    });

    // OpenAPI docs
    #[derive(OpenApi)]
    #[openapi(
        paths(
            profiling::get_metrics,
            profiling::get_health,
        ),
        components(
            schemas(profiling::MetricsReport, profiling::HealthResponse)
        ),
        tags(
            (name = "profiling", description = "Performance and health monitoring endpoints")
        )
    )]
    struct ApiDoc;

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/", get(|| async { "Crucible Backend API" }))
        .route("/.well-known/stellar.toml", get(stellar::get_stellar_toml))
        .nest(
            "/api/v1/profiling",
            Router::new()
                .route("/metrics", get(profiling::get_metrics))
                .route("/health", get(profiling::get_health))
                .route("/prometheus", get(profiling::get_prometheus_metrics)),
        )
        .route("/api/status", get(profiling::get_system_status))
        .route("/api/profile", post(profiling::trigger_profile_collection))
        .with_state(profiling_state)
        .route("/api/dashboard", get(get_dashboard))
        .with_state(dashboard_state)
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .layer(TraceLayer::new_for_http())
        .layer(cors);

    let addr = SocketAddr::from(([0, 0, 0, 0], config.server_port));
    tracing::info!("listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;

    tokio::select! {
        res = axum::serve(listener, app).with_graceful_shutdown(shutdown_signal()) => {
            res?;
        },
        _ = worker.run() => {
            tracing::info!("Worker stopped");
        }
    }

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            tracing::info!("Received Ctrl+C, starting graceful shutdown");
        },
        _ = terminate => {
            tracing::info!("Received SIGTERM, starting graceful shutdown");
        },
    }
}
