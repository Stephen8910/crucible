# Backend Service

A production-ready backend service built with Rust, Axum, PostgreSQL, and Redis.

## Overview

This is the backend API service for the application, providing:
- RESTful HTTP APIs using Axum
- PostgreSQL database operations via SQLx
- Redis caching and job queues
- Comprehensive error handling
- Observability with tracing

## Tech Stack

- **Runtime**: Tokio (async Rust)
- **HTTP Framework**: Axum
- **Database**: PostgreSQL with SQLx
- **Cache/Queue**: Redis
- **Observability**: Tracing

## Project Structure

```
backend/
├── src/
│   ├── main.rs           # Application entry point
│   ├── lib.rs            # Library root
│   ├── error.rs          # Error types
│   └── test_utils/
│       ├── factories.rs  # Factory module
│       ├── user.rs       # User factory
│       ├── order.rs      # Order factory
│       ├── product.rs    # Product factory
│       └── session.rs    # Session factory
├── Cargo.toml
└── README.md
```

## Getting Started

### Prerequisites

- Rust 1.70+
- PostgreSQL 14+
- Redis 7+

### Build

```bash
cd backend
cargo build --release
```

### Run

```bash
cargo run --release
```

The server starts on `http://localhost:3000`.

## API Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/` | API version info |
| GET | `/health` | Health check |
| GET | `/api/users` | List users |
| POST | `/api/users` | Create user |

## Test Utilities

The `test_utils` module provides factory functions for creating domain objects in tests:

### User Factory

```rust
use backend::test_utils::factories::{create_user, create_user_with, create_users_with, build_user, UserFactory};

// Create with defaults
let user = create_user();

// Create with customizations
let user = create_user_with(|u| {
    u.email = "test@example.com".to_string();
});

// Create many with incremental changes
let users = create_users_with(5, |u, i| {
    u.email = format!("user{}@example.com", i);
});

// Builder pattern
let user = build_user()
    .email("user@example.com")
    .is_admin(true)
    .finish();
```

### Order Factory

```rust
use backend::test_utils::factories::{create_order, build_order, OrderItem};
use uuid::Uuid;

let order = build_order()
    .user_id(user_id)
    .add_item(OrderItem::new(
        Uuid::new_v4(),
        "Product".to_string(),
        2,
        1999,
    ))
    .finish();
```

### Product Factory

```rust
use backend::test_utils::factories::{create_product, build_product, ProductCategory};

let product = build_product()
    .name("New Product")
    .price_cents(2999)
    .category(ProductCategory::Electronics)
    .finish();
```

### Session Factory

```rust
use backend::test_utils::factories::{create_session, build_session};

let session = build_session()
    .user_id(user_id)
    .expires_in_days(30)
    .finish();
```

## Error Handling

The module provides custom error types with HTTP status code mapping:

```rust
use backend::error::{Error, Result};

fn example() -> Result<User> {
    Err(Error::NotFound("User not found".to_string()))
}
```

## Testing

Run all tests:

```bash
cargo test
```

Run with output:

```bash
cargo test -- --nocapture
```

## Configuration

Environment variables:
- `DATABASE_URL` - PostgreSQL connection string
- `REDIS_URL` - Redis connection string
- `LOG_LEVEL` - Logging level (default: debug)

## License

MIT