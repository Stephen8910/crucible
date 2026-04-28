# Crucible Backend

This is the backend service layer for the Crucible project.

## Technologies
- **Axum**: Web framework
- **Tokio**: Async runtime
- **SQLx**: PostgreSQL driver
- **Redis**: Caching and job queues
- **Tracing**: Observability

## Structure
- `src/api/`: API handlers and routing
- `src/services/`: Business logic and external integrations
- `src/models/`: Data structures and database schemas

## Running
```bash
cargo run -p backend
```

## Testing
```bash
cargo test -p backend
```
