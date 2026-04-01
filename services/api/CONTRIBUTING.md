# Rust API Service Development Guide

Monobase API service built with Rust (Axum + SeaORM).

## Table of Contents

1. [Service Overview](#service-overview)
2. [Code Generation](#code-generation)
3. [Error Handling](#error-handling)
4. [Handler Implementation](#handler-implementation)
5. [Database Operations](#database-operations)
6. [Authentication & Authorization](#authentication--authorization)
7. [External Integrations](#external-integrations)
8. [Transport Layer](#transport-layer)
9. [Best Practices](#best-practices)
10. [Quick Reference](#quick-reference)

---

## Service Overview

**Purpose**: Native Rust port of the Monobase API — drop-in replacement for `services/api` (TypeScript).

**Key Technologies:**
- **Axum** 0.8: Web framework (tower ecosystem) — [docs](https://docs.rs/axum)
- **SeaORM** 1.1: Async ORM (PostgreSQL + SQLite) — [docs](https://www.sea-ql.org/SeaORM/docs)
- **tokio**: Async runtime — [docs](https://tokio.rs)
- **tracing**: Structured logging — [docs](https://docs.rs/tracing)
- **async-stripe**: Stripe SDK — [docs](https://docs.rs/async-stripe)
- **aws-sdk-s3**: S3/MinIO storage — [docs](https://docs.rs/aws-sdk-s3)
- **lettre**: SMTP email — [docs](https://docs.rs/lettre)

**For the full rewrite specification**, see [API_REWRITE.md](./API_REWRITE.md).

---

## Code Generation

**Never edit generated files!** They are overwritten by the generator.

### Generated Files (Never Edit)

- `src/generated/types.rs` — Request/response structs from OpenAPI
- `src/generated/enums.rs` — All API/DB enums
- `src/generated/routes.rs` — Route metadata and documentation
- `src/generated/mod.rs` — Module declarations

### Running the Generator

```bash
npx tsx generator-rs.ts
# or
bun run generate
```

Reads `specs/api/dist/openapi/openapi.json` and emits Rust code. Run after updating TypeSpec definitions.

### What to Edit Manually

- `src/handlers/{module}/mod.rs` — Route handlers
- `src/handlers/{module}/repo.rs` — SQL queries
- `src/service/*.rs` — External integrations
- `src/auth/*.rs` — Authentication logic
- `src/config.rs` — Configuration

---

## Error Handling

### Let errors propagate with `?`

The API uses `Result<T, ApiError>` everywhere. The `ApiError` enum implements `IntoResponse`, so errors automatically become proper HTTP responses. No manual try/catch needed.

### Correct: Use `?` operator

```rust
pub async fn get_person(
    State(state): State<Arc<AppState>>,
    Path(person_id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let person = repo::get_person(&state.db, &person_id)
        .await?
        .ok_or_else(|| ApiError::not_found("Person not found"))?;

    Ok(Json(person))
}
```

### Wrong: Manual error handling

```rust
// DON'T DO THIS
match repo::get_person(&state.db, &person_id).await {
    Ok(Some(person)) => Ok(Json(person)),
    Ok(None) => Err(ApiError::not_found("Person not found")),
    Err(e) => Err(ApiError::internal(format!("DB error: {}", e))),
}
```

### Error Types

| Error | Status | Code | Use for |
|-------|--------|------|---------|
| `ApiError::not_found(msg)` | 404 | NOT_FOUND | Missing resources |
| `ApiError::unauthorized(msg)` | 401 | UNAUTHORIZED | Missing/invalid auth |
| `ApiError::forbidden(msg)` | 403 | FORBIDDEN | Insufficient permissions |
| `ApiError::validation(msg, fields)` | 400 | VALIDATION_ERROR | Invalid input |
| `ApiError::conflict(msg)` | 409 | CONFLICT | Duplicate resources |
| `ApiError::BusinessLogic { msg, code }` | 422 | (custom) | Domain rule violations |
| `ApiError::Database(e)` | 500 | INTERNAL_ERROR | Auto-converted from SeaORM |

---

## Handler Implementation

### Module Structure

```
src/handlers/{module}/
├── mod.rs      # All handlers + request/response types
└── repo.rs     # SQL queries via SeaORM
```

### Handler Pattern

```rust
use std::sync::Arc;
use axum::{Json, extract::{Path, State}, http::HeaderMap};
use crate::error::ApiError;
use crate::handlers::AppState;
use crate::middleware::auth as auth_mw;

/// POST /patients
pub async fn create_patient(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<CreatePatientBody>,
) -> Result<(axum::http::StatusCode, Json<Value>), ApiError> {
    let user = auth_mw::require_roles(
        &state.db, &headers, &state.config.auth_secret, &["user"]
    ).await?;

    let patient = repo::create_patient(&state.db, &user.id, &body).await?;

    Ok((axum::http::StatusCode::CREATED, Json(patient)))
}
```

### Repository Pattern

```rust
use sea_orm::{DatabaseConnection, FromQueryResult, JsonValue, Statement};
use crate::error::ApiError;

pub async fn get_patient(
    db: &DatabaseConnection,
    id: &str,
) -> Result<Option<Value>, ApiError> {
    let sql = r#"SELECT * FROM "patient" WHERE id = $1"#;
    let result = JsonValue::find_by_statement(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        sql,
        vec![id.into()],
    ))
    .one(db)
    .await
    .map_err(ApiError::Database)?;

    Ok(result)
}
```

Key patterns:
- Use `JsonValue::find_by_statement` for flexible results (no entity codegen)
- Use `Statement::from_sql_and_values` with `DatabaseBackend::Postgres`
- Convert DB errors with `.map_err(ApiError::Database)?`
- Return `Option<Value>` for single results, `Vec<Value>` for lists

---

## Database Operations

### Base Entity Fields

All tables include:
```sql
id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
created_at  TIMESTAMP NOT NULL DEFAULT NOW(),
updated_at  TIMESTAMP NOT NULL DEFAULT NOW(),
version     INT NOT NULL DEFAULT 1,  -- optimistic locking
created_by  UUID,
updated_by  UUID
```

### Dialect Abstraction

The `Dialect` enum in `src/model/dialect.rs` encapsulates PostgreSQL vs SQLite differences:

```rust
let now = state.dialect.now_expr();        // "NOW()" or "datetime('now')"
let cast = state.dialect.json_cast();       // "::jsonb" or ""
let op = state.dialect.regex_op(true);      // "~*" or "LIKE"
```

### Dynamic Updates

Use the `maybe_set!` macro pattern for partial updates:

```rust
let mut set_clauses = vec!["updated_at = NOW()".to_string()];
let mut params: Vec<sea_orm::Value> = vec![];
let mut idx = 1u32;

macro_rules! maybe_set {
    ($field:ident, $col:expr) => {
        if let Some(ref val) = body.$field {
            set_clauses.push(format!("{} = ${}", $col, idx));
            params.push(val.clone().into());
            idx += 1;
        }
    };
}

maybe_set!(first_name, "first_name");
maybe_set!(last_name, "last_name");
// ... more fields

set_clauses.push("version = version + 1".to_string());
```

### Pagination

Return the standard `PaginatedResult<T>` for all list endpoints:

```rust
use crate::model::{PaginatedResult, PaginationMeta};

Ok(Json(PaginatedResult {
    data,
    pagination: PaginationMeta::new(offset, limit, data.len() as u64, total_count),
}))
```

### Migrations

SQL migration files live at `migrations/`. They were generated by Drizzle from the TypeScript service and are compatible with any PostgreSQL migration tool.

---

## Authentication & Authorization

### Session Validation

Auth is handled per-handler using middleware helpers:

```rust
// Require any authenticated user
let user = auth_mw::require_auth(&state.db, &headers, &state.config.auth_secret).await?;

// Require specific roles (OR logic)
let user = auth_mw::require_roles(
    &state.db, &headers, &state.config.auth_secret, &["admin", "support"]
).await?;

// Optional auth (public endpoints)
let user = auth_mw::extract_auth(&state.db, &headers, &state.config.auth_secret).await?;
// user is Option<AuthUser>
```

### How Auth Works

1. Extract token from `Authorization: Bearer {token}` or `better-auth.session_token` cookie
2. Verify HMAC-SHA256 signature using `AUTH_SECRET`
3. Query `session` + `user` tables
4. Check expiry and banned status
5. Parse comma-separated roles from `user.role`

### Owner Checks

```rust
let user = auth_mw::require_auth(&state.db, &headers, &state.config.auth_secret).await?;

let resource = repo::get_resource(&state.db, &id).await?
    .ok_or_else(|| ApiError::not_found("Resource not found"))?;

let created_by = resource.get("createdBy").and_then(|v| v.as_str()).unwrap_or("");
if created_by != user.id && !user.has_any_role(&["admin"]) {
    return Err(ApiError::forbidden("Access denied"));
}
```

---

## External Integrations

### Stripe (`service/billing.rs`)
- Uses `async-stripe` 0.39 with lazy client initialization
- Payment intents, Connect accounts, refunds, webhook verification
- Custom Stripe URL support for testing (`STRIPE_URL` env var)

### S3/MinIO (`service/storage.rs`)
- Uses `aws-sdk-s3` with presigned URL generation
- `force_path_style(true)` for MinIO compatibility
- Upload/download URL expiry configurable

### Email (`service/email.rs`)
- SMTP via `lettre` (async transport)
- Postmark via `reqwest` HTTP API
- OneSignal email via `reqwest` HTTP API
- Provider selected by `EMAIL_PROVIDER` env var

### Notifications (`service/notifs.rs`)
- OneSignal push via `reqwest` HTTP API
- Channel routing: push, email, in-app

### WebSocket (`service/ws.rs` + `transport/http.rs`)
- `/ws/user` — personal notifications
- `/ws/comms/chat-rooms/:room` — chat room messaging
- Token auth via `?token=` query parameter

### Background Jobs (`service/jobs.rs`)
- Polls `pgboss.job` table with `FOR UPDATE SKIP LOCKED`
- Supports cron, interval, delayed, manual trigger
- Compatible with existing pg-boss schema

---

## Transport Layer

### HTTP (server mode)
Routes are registered in `src/transport/http.rs` using Axum. Middleware stack:
1. `CompressionLayer` — gzip responses
2. `TimeoutLayer` — 30s request timeout
3. `TraceLayer` — request/response tracing
4. `CorsLayer` — CORS headers

### IPC (embedded mode)
`src/transport/ipc.rs` defines `IpcRequest`/`IpcResponse` structs for direct function calls in Tauri/iOS mode. No HTTP overhead.

---

## Best Practices

1. **Use `?` operator** — let errors propagate to the global handler
2. **One handler per route** — keep handlers focused
3. **SQL in repos** — handlers orchestrate, repos query
4. **`JsonValue` for flexibility** — no entity codegen needed
5. **Validate in handlers** — before calling repos
6. **Optimistic locking** — `version = version + 1` on updates
7. **Audit fields** — set `created_by`/`updated_by` from auth user
8. **Business logic errors** — use `ApiError::BusinessLogic` with a stable code
9. **State transitions** — validate current status before changing
10. **Standard pagination** — always return `PaginatedResult`

---

## Quick Reference

### Commands

```bash
cargo run                    # Start dev server (port 7213)
cargo test                   # Unit tests
cargo check                  # Fast type check
cargo build --release        # Release binary

bun install                  # Install E2E test deps
./run-tests.sh               # Run E2E tests
bun run deps:up              # Start test infra (PG, MinIO, Mailpit)
bun run deps:down            # Stop test infra
bun run generate             # Regenerate Rust types from OpenAPI
```

### Adding a New Module

1. Create `src/handlers/{module}/mod.rs` — handlers + types
2. Create `src/handlers/{module}/repo.rs` — SQL queries
3. Add `pub mod {module};` to `src/handlers/mod.rs`
4. Add routes to `src/transport/http.rs`
5. Add E2E tests at `tests/e2e/{module}/{module}.test.ts`

### Before Implementing Features

Follow API-First workflow:
1. Define API in TypeSpec (`specs/api/src/modules/`)
2. Generate OpenAPI + types (`cd specs/api && bun run build`)
3. Regenerate Rust types (`bun run generate`)
4. Implement handler + repo
5. Add E2E tests

**Never edit generated files!**

---

## External Documentation

- **Axum**: https://docs.rs/axum
- **SeaORM**: https://www.sea-ql.org/SeaORM/docs
- **tokio**: https://tokio.rs
- **tracing**: https://docs.rs/tracing
- **async-stripe**: https://docs.rs/async-stripe
- **aws-sdk-s3**: https://docs.rs/aws-sdk-s3
- **lettre**: https://docs.rs/lettre
