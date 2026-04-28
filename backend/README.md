# Crucible Backend

This is the backend service layer for the Crucible project.

## Technologies
- **Axum**: Web framework
- **Tokio**: Async runtime
- **SQLx**: PostgreSQL driver
- **Redis**: Caching and job queues
- **Tracing**: Observability

## Structure
- `src/api/` — API handlers and routing
- `src/services/` — Business logic and external integrations
- `src/models/` — Data structures and database schemas
- `tests/` — Integration and API tests

## Running
```bash
cargo run -p backend
```

## Testing

### All tests
```bash
cargo test -p backend
```

### Unit tests only
```bash
cargo test -p backend --lib
```

### Integration tests only
```bash
cargo test -p backend --test integration_tests
```

## Integration Test Framework

Integration tests live under `tests/integration/` and are compiled as a single
test crate via the `tests/integration_tests.rs` entry point.

### Layout

```
tests/
├── integration_tests.rs        # Cargo entry point — declares the integration module
├── api_tests.rs                # Legacy API smoke test
└── integration/
    ├── mod.rs                  # Shared helpers (test_app builder)
    ├── api_status_test.rs      # Tests for GET /api/status
    ├── api_profile_test.rs     # Tests for POST /api/profile
    └── services_test.rs        # Tests for MetricsExporter, ErrorManager, LogAggregator
```

### Shared helpers

`integration::test_app()` returns a fully-configured [`axum::Router`] backed by
fresh in-memory service instances. Use [`tower::ServiceExt::oneshot`] to send a
single request without binding a TCP socket:

```rust
use tower::ServiceExt;
use hyper::{Request, StatusCode};
use axum::body::Body;

#[tokio::test]
async fn my_test() {
    let response = test_app()
        .oneshot(Request::builder().uri("/api/status").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}
```

### Adding new tests

1. Create a new file under `tests/integration/`, e.g. `my_feature_test.rs`.
2. Declare it in `tests/integration/mod.rs`:
   ```rust
   pub mod my_feature_test;
   ```
3. Write `#[tokio::test]` functions — no extra setup required.
