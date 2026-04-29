# Crucible Backend

This is the backend service layer for the Crucible toolkit, providing performance profiling, mock service layers, specialized serialization utilities, and robust background monitoring.

## Features

### 🚀 Performance Profiling API
High-performance endpoints for monitoring application health and system metrics.
- `/api/v1/profiling/metrics`: Real-time system metrics.
- `/api/v1/profiling/health`: System health status.
- `/api/status`: Unified health, metrics, and active recovery tasks.

### 🧪 Mock Service Layer
A robust mock layer for testing services in isolation, supporting both database and cache operations.

### 🔢 Custom Serialization
Specialized Serde serializers for high-precision types and Stellar-specific formats.

### 🛠️ Background Services
The backend runs several background workers for system health and data consistency.

| Module | Description |
|---|---|
| `sys_metrics` | Build system metrics exporter with PostgreSQL persistence and Redis caching (compilation times, dependency counts, cache hit rates) |
| `error_recovery` | Tracks retry state for failing tasks with configurable max retries |
| `log_aggregator` | Async MPSC-based log pipeline; persists entries via a background worker |
| `log_alerts` | Threshold-based alerting over the log pipeline with sliding-window evaluation |
| `feature_flags` | Feature flag management backed by PostgreSQL with Redis caching |

## Tech Stack
- **Web Framework**: Axum (async Rust)
- **Runtime**: Tokio
- **Database**: PostgreSQL (via SQLx 0.8)
- **Caching & Jobs**: Redis (via Apalis)
- **Serialization**: Serde
- **Observability**: Tracing
- **API Documentation**: Utoipa (Swagger UI)

## API Endpoints

| Method | Path | Description |
|---|---|---|
| `GET` | `/` | Base API greeting |
| `GET` | `/.well-known/stellar.toml` | Stellar network metadata |
| `GET` | `/api/v1/profiling/metrics` | Detailed performance metrics (OpenAPI) |
| `GET` | `/api/v1/profiling/health` | Service health check (OpenAPI) |
| `GET` | `/api/status` | System health summary and recovery status |
| `POST` | `/api/profile` | Trigger a manual profiling collection run |
| `GET` | `/swagger-ui` | Interactive API documentation |

## Development

### Running the App
```bash
cargo run -p backend
```

### Running Tests
```bash
# All tests (unit + integration)
cargo test -p backend

# Load tests specifically
cargo test -p backend --test load_tests -- --nocapture

# Build metrics integration tests (requires PostgreSQL and Redis)
cargo test -p backend --test build_metrics_tests -- --ignored
```

## Build System Metrics Exporter

The `sys_metrics` module provides a production-ready build system metrics exporter that tracks and analyzes build performance across projects.

### Features

- **Build Tracking**: Record compilation times, dependency counts, and resource usage
- **Status Monitoring**: Track build success/failure/cancellation rates
- **Cache Analytics**: Monitor cache hit rates to optimize build performance
- **Resource Metrics**: Track CPU and memory usage during builds
- **PostgreSQL Persistence**: Durable storage for historical metrics
- **Redis Caching**: High-performance caching with automatic invalidation
- **Aggregated Summaries**: Get project-level statistics and success rates

### Usage Example

```rust
use backend::services::sys_metrics::{BuildMetricsService, BuildMetric, BuildStatus};
use sqlx::PgPool;
use redis::Client;

let service = BuildMetricsService::new(pool, redis);

// Record a build metric
let metric = BuildMetric {
    id: None,
    project_name: "crucible".to_string(),
    build_id: "build-123".to_string(),
    build_status: BuildStatus::Success,
    compilation_time_ms: 5000,
    dependency_count: 42,
    cache_hit_rate: Some(85.5),
    cpu_usage: Some(75.2),
    memory_usage_mb: Some(1024),
    build_timestamp: Utc::now(),
};
service.record_build(metric).await?;

// Get project metrics with caching
let metrics = service.get_project_metrics("crucible", 10).await?;

// Get aggregated summary
let summary = service.get_project_summary("crucible").await?;
println!("Success rate: {}%", summary.success_rate);
```

### API Reference

#### BuildMetricsService

- `new(db, redis)` - Create a new metrics service
- `record_build(metric)` - Record a build metric (invalidates cache)
- `get_project_metrics(project_name, limit)` - Get metrics for a project (with caching)
- `get_project_summary(project_name)` - Get aggregated statistics
- `get_recent_metrics(limit)` - Get recent builds across all projects
- `delete_project_metrics(project_name)` - Delete all metrics for a project

## Structure
- `src/api/` – API handlers and routing
- `src/config/` – Environment configuration
- `src/db/` – Database utilities and seed data
- `src/jobs/` – Background job definitions (Apalis)
- `src/services/` – Business logic and external integrations
- `src/telemetry/` – Observability and logging setup

