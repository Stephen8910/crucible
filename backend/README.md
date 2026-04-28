# Crucible Backend

This is the backend component of the Crucible toolkit, providing performance profiling, mock service layers, and specialized serialization utilities.

## Features

### 🚀 Performance Profiling API
High-performance endpoints for monitoring application health and system metrics.
- `/api/v1/profiling/metrics`: Real-time system metrics.
- `/api/v1/profiling/health`: System health status.

### 🧪 Mock Service Layer
A robust mock layer for testing services in isolation, supporting both database and cache operations.

### 🔢 Custom Serialization
Specialized Serde serializers for high-precision types and Stellar-specific formats.

## Tech Stack
- **Web Framework**: Axum (async Rust)
- **Database**: PostgreSQL (via SQLx)
- **Caching**: Redis
- **Serialization**: Serde
- **Observability**: Tracing

## Development

### Running Tests
```bash
cargo test -p backend
```

### CI/CD
The project uses GitHub Actions for continuous integration. Configuration can be found in `.github/workflows/backend-ci.yml`.
