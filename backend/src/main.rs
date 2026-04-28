use backend::{
    telemetry::init_telemetry, 
    config::Config,
    jobs::{monitor_transaction, TransactionMonitorJob}
};
use axum::{routing::get, Router};
use std::net::SocketAddr;
use tower_http::{
    trace::TraceLayer,
    cors::{CorsLayer, Any},
};
use tokio::signal;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;
use backend::api::handlers::{profiling, stellar};
use apalis::prelude::*;
use apalis_redis::RedisStorage;
use sqlx::postgres::PgPoolOptions;
use redis::aio::ConnectionManager;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Load configuration
    let config = Config::from_env()?;

    // Initialize observability
    init_telemetry();

    // Database setup & migrations
    let db_pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&config.database_url)
        .await?;
    
    tracing::info!("Database migrations synchronized");

    // Redis Job Queue setup
    // Apalis 0.6.4 RedisStorage requires a ConnectionManager
    let redis_client = redis::Client::open(config.redis_url)?;
    let conn = ConnectionManager::new(redis_client).await?;
    let storage: RedisStorage<TransactionMonitorJob> = RedisStorage::new(conn);
    
    let worker = WorkerBuilder::new("monitor-worker")
        .backend(storage)
        .build_fn(monitor_transaction);

    // Define OpenAPI documentation
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

    // Configure CORS
    let cors = CorsLayer::new()
        .allow_origin(Any) // In production, replace with specific origins
        .allow_methods(Any)
        .allow_headers(Any);

    // Build our application with routes
    let app = Router::new()
        .route("/", get(|| async { "Crucible Backend API" }))
        .route("/.well-known/stellar.toml", get(stellar::get_stellar_toml))
        .nest("/api/v1/profiling", Router::new()
            .route("/metrics", get(profiling::get_metrics))
            .route("/health", get(profiling::get_health))
            .route("/prometheus", get(profiling::get_prometheus_metrics))
        )
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(db_pool);

    // Run it with graceful shutdown and background workers
    let addr = SocketAddr::from(([0, 0, 0, 0], config.server_port));
    tracing::info!("listening on {}", addr);
    
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    
    tracing::info!("Starting services...");

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
