# Crucible Backend

This is the backend service layer for the Crucible project.

## Technologies
- **Axum**: Web framework
- **Tokio**: Async runtime
- **SQLx**: PostgreSQL driver (with `uuid` and `chrono` support)
- **Redis**: Caching and job queues
- **Tracing**: Observability

## Structure
- `src/api/` – API handlers and routing
- `src/db/` – Database utilities and seed data
- `src/services/` – Business logic and external integrations

### Services

| Module | Description |
|---|---|
| `sys_metrics` | Collects and exposes system metrics (CPU, memory, uptime) |
| `error_recovery` | Tracks retry state for failing tasks with configurable max retries |
| `log_aggregator` | Async MPSC-based log pipeline; persists entries via a background worker |
| `log_alerts` | Threshold-based alerting over the log pipeline with sliding-window evaluation |
| `feature_flags` | Feature flag management backed by PostgreSQL with Redis caching |

### Database (`src/db/`)

| Module | Description |
|---|---|
| `seeds` | Idempotent seed data for development and test environments |

## API Endpoints

| Method | Path | Description |
|---|---|---|
| `GET` | `/api/status` | System health, metrics, and active recovery tasks |
| `POST` | `/api/profile` | Trigger a profiling collection run |

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

## Log Alerts

Alert rules evaluate incoming log entries against a pattern within a sliding time window.

```rust
let manager = AlertManager::new();
manager.add_rule(AlertRule {
    id: Uuid::new_v4(),
    name: "High error rate".to_string(),
    pattern: "ERROR".to_string(),
    severity: AlertSeverity::Critical,
    threshold: 5,
    window_secs: 60,
}).await?;

// Evaluate a log entry
manager.evaluate(&log_entry).await;

// Retrieve fired alerts
let alerts = manager.get_active_alerts().await;
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

## Configuration Hot-Reload

The backend supports hot-reloading configuration from `config.json` without restarting the server.

### Configuration Structure

```rust
pub struct AppConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub log_level: String,
}
```

### Endpoints

| Method | Path | Description |
|---|---|---|
| `GET` | `/api/config` | Retrieve current configuration (sanitized) |
| `POST` | `/api/config/reload` | Trigger a reload from `config.json` |

### Usage

1. Create a `config.json` in the root directory.
2. Update values in the file.
3. Call `POST /api/config/reload` to apply changes.
