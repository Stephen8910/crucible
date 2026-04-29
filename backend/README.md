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

## Tech Stack
- **Web Framework**: Axum (async Rust)
- **Runtime**: Tokio
- **Database**: PostgreSQL (via SQLx 0.8)
- **Caching & Jobs**: Redis (via Apalis)
- **Serialization**: Serde
- **Observability**: Tracing + OpenTelemetry (OTLP)
- **API Documentation**: Utoipa (Swagger UI)

## Structure
- `src/api/` – API handlers and routing
- `src/config/` – Environment configuration and hot-reload
- `src/db/` – Database utilities and seed data
- `src/jobs/` – Background job definitions (Apalis)
- `src/services/` – Business logic and external integrations
- `src/telemetry/` – Observability and logging setup
- `src/utils/` – Serialization, validation, XDR helpers
- `src/test_utils/` – Mock traits for unit testing

### API Handlers (`src/api/handlers/`)

| Module | Description |
|---|---|
| `profiling` | System status, metrics, health, and profiling trigger endpoints |
| `dashboard` | Aggregated dashboard data endpoint with Redis caching |
| `stellar` | Stellar SEP-1 `.well-known/stellar.toml` endpoint |

### Services (`src/services/`)

| Module | Description |
|---|---|
| `sys_metrics` | Collects and exposes system metrics (CPU, memory, uptime) |
| `error_recovery` | Tracks retry state for failing tasks with configurable max retries |
| `log_aggregator` | Async MPSC-based log pipeline; persists entries via a background worker |
| `log_alerts` | Threshold-based alerting over the log pipeline with sliding-window evaluation |
| `feature_flags` | Feature flag management backed by PostgreSQL with Redis caching |
| `alerts` | Critical-error notification dispatcher — deduplication, in-memory queue, Redis pub/sub |
| `tracing` | OpenTelemetry tracing initialisation — wires `tracing` spans to an OTLP HTTP exporter |

### Database (`src/db/`)

| Module | Description |
|---|---|
| `seeds` | Idempotent seed data for development and test environments |

## API Endpoints

| Method | Path | Description |
|---|---|---|
| `GET` | `/` | Base API greeting |
| `GET` | `/.well-known/stellar.toml` | Stellar network metadata (SEP-1) |
| `GET` | `/api/v1/profiling/metrics` | Detailed performance metrics (OpenAPI) |
| `GET` | `/api/v1/profiling/health` | Service health check (OpenAPI) |
| `GET` | `/api/v1/profiling/prometheus` | Prometheus-compatible metrics |
| `GET` | `/api/status` | System health summary and recovery status |
| `POST` | `/api/profile` | Trigger a manual profiling collection run |
| `GET` | `/api/dashboard` | Aggregated dashboard data: metrics, recovery tasks, and active alerts (Redis-cached, 30 s TTL) |
| `GET` | `/swagger-ui` | Interactive API documentation |

## Running
```bash
cargo run -p backend
```

## Testing
```bash
# All tests (unit + integration + load)
cargo test -p backend

# Load tests only
cargo test -p backend --test load_tests -- --nocapture
```

## Configuration Hot-Reload

`ConfigWatcher` holds the live `AppConfig` behind an `Arc<RwLock<_>>`. Any part of the application that holds a `ConfigHandle` sees new values immediately after a reload — no restart required.

```rust
use std::sync::Arc;
use backend::config::reload::{AppConfig, ConfigWatcher};

let watcher = Arc::new(ConfigWatcher::new(AppConfig::default()));
let handle = watcher.handle(); // cheap to clone, share across handlers

// Manual reload
watcher.reload(AppConfig { maintenance_mode: true, ..AppConfig::default() }).await;

// Reload from Redis key `config:current`
watcher.reload_from_redis(&redis_client).await?;

// Background watcher — subscribes to `config:reload` pub/sub channel
watcher.watch(redis_client); // returns a JoinHandle
```

Trigger a reload from the Redis CLI:

```bash
redis-cli SET config:current '{"log_level":"info","max_connections":50,"request_timeout_secs":30,"maintenance_mode":false,"redis_config_key":"config:current"}'
redis-cli PUBLISH config:reload reload
```

## Critical Error Alerting

`AlertDispatcher` sits on top of `log_alerts` and dispatches notifications when a critical condition fires. It deduplicates within a configurable cooldown window and publishes to Redis pub/sub.

```rust
use std::sync::Arc;
use backend::services::alerts::{AlertDispatcher, AlertNotification, NotificationLevel};

let dispatcher = Arc::new(AlertDispatcher::new(Some(redis_client), 60));

// Dispatch directly
dispatcher.dispatch(AlertNotification {
    alert_key: "db_down".to_string(),
    level: NotificationLevel::Critical,
    title: "Database unreachable".to_string(),
    message: "Pool exhausted after 3 retries".to_string(),
    metadata: Default::default(),
}).await?;

// Or derive from a fired log_alerts::Alert (only Critical severity is dispatched)
dispatcher.dispatch_alert(&fired_alert).await?;

// Drain the in-memory queue
let pending = dispatcher.drain_notifications().await;
```

Redis pub/sub channel defaults to `alerts:critical`; override with `.with_channel("my-channel")`.

## OpenTelemetry Tracing

Spans from every `#[tracing::instrument]`-annotated function are exported to an OTLP-compatible collector over HTTP/protobuf.

```rust
use backend::services::tracing::{init, TracingConfig};

let _guard = init(TracingConfig::from_env())?;
// spans are now exported; _guard flushes them on drop
```

| Environment variable | Default | Description |
|---|---|---|
| `OTEL_EXPORTER_OTLP_ENDPOINT` | `http://localhost:4318` | OTLP HTTP collector URL |
| `OTEL_SERVICE_NAME` | `backend` | Service name on every span |
| `RUST_LOG` | `backend=debug` | Log/span filter directive |

Run a local collector with Docker:

```bash
docker run -d -p4317:4317 -p4318:4318 -p16686:16686 jaegertracing/all-in-one:latest
# View traces at http://localhost:16686
```

## Feature Flags

Feature flags are stored in PostgreSQL and cached in Redis with a 5-minute TTL.

```rust
let service = FeatureFlagService::new(pool, redis_client);

// Check a flag
if service.is_enabled("new_dashboard").await? {
    // render new UI
}

// Create / update a flag
service.set("new_dashboard", true, "Enable redesigned dashboard").await?;
```

## Database Seeds

Seeds are idempotent and safe to run multiple times:

```bash
# In application code
run_all(&pool).await?;
```

Seeds populate:
- `users` table with two default accounts (`admin`, `dev`)
- `feature_flags` table with baseline flags (`new_dashboard`, `beta_api`)
