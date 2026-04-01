# API Development Standards

This document outlines the API development standards and best practices for the Monobase Healthcare Platform. These standards ensure consistency, maintainability, and developer experience across all API modules.

## Table of Contents

1. [Overview](#overview)
2. [Field Naming Conventions](#field-naming-conventions)
3. [Response Patterns](#response-patterns)
4. [Request Body Standards](#request-body-standards)
5. [Entity Reference Patterns](#entity-reference-patterns)
6. [Security Pattern Standards](#security-pattern-standards)
7. [Error Handling Standards](#error-handling-standards)
8. [Module Organization](#module-organization)
9. [Examples and Templates](#examples-and-templates)

## Overview

The Monobase API (Rust) is a native backend built on **Axum** with **SeaORM** for database access and **utoipa** for OpenAPI generation. Handlers are organized as plain Rust async functions registered on an Axum router.

### Core Design Principles

- **Consistency**: Uniform naming, structure, and behavior across all endpoints
- **Type Safety**: Rust's type system enforced at compile time; serde for JSON
- **Healthcare Compliance**: HIPAA-compliant patterns with audit trails and consent management
- **Developer Experience**: Clear documentation, predictable patterns, and structured error responses
- **Zero-Cost Abstractions**: No runtime overhead from the framework layer

### Technology Stack

| Layer | Crate |
|-------|-------|
| HTTP framework | `axum 0.8` |
| ORM | `sea-orm 1.1` (Postgres + SQLite) |
| OpenAPI | `utoipa 5.3` + `utoipa-axum` |
| Validation | `garde 0.21` |
| Auth | `bcrypt`, `hmac`, `sha2`, `base64` |
| Billing | `async-stripe 0.39` |
| Email | `lettre 0.11` (SMTP) + `reqwest` (Postmark/OneSignal) |
| Logging | `tracing` + `tracing-subscriber` |

## Field Naming Conventions

### Primary Rules

**Use camelCase in JSON, snake_case in Rust**

Serde handles the mapping via `#[serde(rename_all = "camelCase")]` on request/response structs:

```rust
// ✅ Correct — Rust struct with camelCase JSON output
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PatientResponse {
    pub id: Uuid,
    pub first_name: String,
    pub last_name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

Wire format:
```json
{
  "id": "123e4567-e89b-12d3-a456-426614174000",
  "firstName": "John",
  "lastName": "Doe",
  "createdAt": "2023-12-01T10:00:00Z",
  "updatedAt": "2023-12-01T10:00:00Z"
}
```

**Avoid `entity_id` suffix pattern**

```rust
// ✅ Correct — direct entity reference
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BookingRequest {
    pub person: Uuid,
    pub provider: Uuid,
}

// ❌ Incorrect — don't use Id suffix in the JSON field name
pub struct BookingRequest {
    pub person_id: Uuid,
    pub provider_id: Uuid,
}
```

### Date and Time Fields

```rust
// ✅ Correct naming patterns
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InvoiceResponse {
    pub created_at: DateTime<Utc>,   // Timestamps use 'At' suffix
    pub updated_at: DateTime<Utc>,
    pub start_time: NaiveTime,       // Time values use 'Time'
    pub end_time: NaiveTime,
    pub invoice_date: NaiveDate,     // Dates use 'Date' suffix
    pub due_date: NaiveDate,
}
```

### Boolean Fields

```rust
// ✅ Use descriptive boolean names
#[derive(Serialize, Deserialize)]
pub struct PaginationMeta {
    pub is_active: bool,
    pub has_next_page: bool,
    pub is_recurring: bool,
}
```

## Response Patterns

### List Endpoints — `PaginatedResponse<T>`

All list endpoints return a paginated envelope:

```rust
#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PaginatedResponse<T: Serialize> {
    pub data: Vec<T>,
    pub pagination: PaginationMeta,
}

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PaginationMeta {
    pub offset: i64,
    pub limit: i64,
    pub count: i64,
    pub total_count: i64,
    pub total_pages: i64,
    pub current_page: i64,
    pub has_next_page: bool,
    pub has_previous_page: bool,
}
```

### Individual Endpoints — Direct Entity Types

Individual resource endpoints return the entity directly as JSON:

```rust
// Handler signature
pub async fn get_patient(
    State(ctx): State<AppContext>,
    AuthUser(user): AuthUser,
    Path(patient_id): Path<Uuid>,
) -> Result<Json<PatientResponse>, ApiError> {
    // ...
}
```

## Request Body Standards

### Required vs Optional Fields

Use `Option<T>` for optional fields in update requests:

```rust
// Create request — required fields are non-Option
#[derive(Debug, Deserialize, garde::Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreatePatientRequest {
    #[garde(skip)]
    pub person: Option<CreatePersonRequest>,
    #[garde(skip)]
    pub primary_provider: Option<ProviderInfo>,
}

// Update request — all fields optional
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePatientRequest {
    pub primary_provider: Option<ProviderInfo>,
    pub primary_pharmacy: Option<PharmacyInfo>,
}
```

### Action Endpoints — Reason Field

Action endpoints include a `reason` field for audit trails:

```rust
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppointmentActionRequest {
    #[garde(length(max = 500))]
    pub reason: Option<String>,
}
```

## Entity Reference Patterns

### Base Fields

All entities include standard audit fields via SeaORM column definitions:

```rust
// Every entity model includes:
pub id: Uuid,
pub created_at: DateTime<Utc>,
pub updated_at: DateTime<Utc>,
pub created_by: Option<Uuid>,
pub updated_by: Option<Uuid>,
```

### Expandable References

Use `serde_json::Value` or a custom enum for fields that may be returned as a UUID reference or a nested object:

```rust
#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PersonRef {
    Id(Uuid),
    Object(Box<PersonResponse>),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PatientResponse {
    pub id: Uuid,
    pub person: PersonRef,
}
```

## Security Pattern Standards

### Authentication and Authorization

All protected handlers extract the authenticated user via an `AuthUser` extractor:

```rust
use crate::middleware::auth::AuthUser;

pub async fn get_patient(
    State(ctx): State<AppContext>,
    AuthUser(user): AuthUser,          // Fails with 401 if token invalid
    Path(patient_id): Path<Uuid>,
) -> Result<Json<PatientResponse>, ApiError> {
    // user.roles contains the authenticated user's roles
    if !user.has_role("admin") && user.id != patient_id {
        return Err(ApiError::forbidden("Insufficient permissions"));
    }
    // ...
}
```

### Role Checking

```rust
// Single role
user.has_role("admin")

// Any of several roles
user.has_any_role(&["admin", "support"])

// Owner permission
user.id == resource.person_id
```

### Public Endpoints

Public endpoints omit the `AuthUser` extractor:

```rust
// No AuthUser — fully public
pub async fn search_providers(
    State(ctx): State<AppContext>,
    Query(params): Query<SearchProvidersParams>,
) -> Result<Json<PaginatedResponse<ProviderResponse>>, ApiError> {
    // ...
}
```

## Error Handling Standards

### `ApiError` Type

All handlers return `Result<_, ApiError>`. The `ApiError` type serializes to a consistent JSON envelope:

```rust
// src/error.rs
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Database error: {0}")]
    Database(#[from] sea_orm::DbErr),

    #[error("Internal error: {0}")]
    Internal(String),
}
```

Wire format on error:
```json
{
  "error": "Not found",
  "message": "Patient 123 not found",
  "statusCode": 404
}
```

### Handler Error Pattern

```rust
pub async fn get_patient(
    State(ctx): State<AppContext>,
    AuthUser(user): AuthUser,
    Path(patient_id): Path<Uuid>,
) -> Result<Json<PatientResponse>, ApiError> {
    let repo = PatientRepo::new(&ctx.db);

    let patient = repo
        .find_by_id(patient_id)
        .await?                          // DbErr → ApiError::Database (via From)
        .ok_or_else(|| ApiError::NotFound(format!("Patient {patient_id} not found")))?;

    Ok(Json(patient.into()))
}
```

## Module Organization

### File Structure

```
services/api-rs/src/
├── main.rs                        # Entry point, router construction
├── config.rs                      # Config struct (clap + env vars)
├── error.rs                       # ApiError type
├── context.rs                     # AppContext (db, services)
├── auth/
│   ├── mod.rs                     # AuthUser, role types
│   ├── session.rs                 # Session verification (HMAC)
│   ├── password.rs                # bcrypt helpers
│   └── backend.rs                 # Sign-up / sign-in logic
├── middleware/
│   └── auth.rs                    # Axum extractor for AuthUser
├── service/
│   ├── billing.rs                 # BillingService (async-stripe)
│   ├── email.rs                   # EmailService (lettre + reqwest)
│   ├── jobs.rs                    # JobScheduler (pg-boss poller)
│   ├── notifs.rs                  # NotificationService (OneSignal)
│   ├── storage.rs                 # StorageService (S3/MinIO)
│   └── ws.rs                      # WebSocketService (broadcast)
└── handlers/
    ├── mod.rs                     # Router assembly
    ├── auth.rs                    # Auth endpoints
    ├── person/
    │   ├── mod.rs                 # Router + handlers
    │   └── repo.rs                # SeaORM queries
    ├── patient/
    │   ├── mod.rs
    │   └── repo.rs
    ├── billing/
    │   ├── mod.rs
    │   └── repo.rs
    └── comms/
        ├── mod.rs
        └── repo.rs
```

### Handler Module Pattern

```rust
// src/handlers/patient/mod.rs

use axum::{Router, routing::{get, post, patch, delete}};
use crate::context::AppContext;

pub fn router() -> Router<AppContext> {
    Router::new()
        .route("/patients",          get(list_patients).post(create_patient))
        .route("/patients/:patient", get(get_patient).patch(update_patient).delete(delete_patient))
        .route("/patients/me",       get(get_me))
}

pub async fn get_patient(/* ... */) -> Result<Json<PatientResponse>, ApiError> { /* ... */ }
pub async fn list_patients(/* ... */) -> Result<Json<PaginatedResponse<PatientResponse>>, ApiError> { /* ... */ }
// ...
```

## Examples and Templates

### Complete CRUD Handler Template

```rust
// src/handlers/example/mod.rs

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{delete, get, patch, post},
};
use uuid::Uuid;

use crate::{
    context::AppContext,
    error::ApiError,
    middleware::auth::AuthUser,
};

pub fn router() -> Router<AppContext> {
    Router::new()
        .route("/examples",          get(list_examples).post(create_example))
        .route("/examples/:example", get(get_example).patch(update_example).delete(delete_example))
}

pub async fn list_examples(
    State(ctx): State<AppContext>,
    AuthUser(user): AuthUser,
    Query(params): Query<ListExamplesParams>,
) -> Result<Json<PaginatedResponse<ExampleResponse>>, ApiError> {
    // ...
    todo!()
}

pub async fn create_example(
    State(ctx): State<AppContext>,
    AuthUser(user): AuthUser,
    Json(body): Json<CreateExampleRequest>,
) -> Result<(StatusCode, Json<ExampleResponse>), ApiError> {
    // ...
    todo!()
}

pub async fn get_example(
    State(ctx): State<AppContext>,
    AuthUser(user): AuthUser,
    Path(example_id): Path<Uuid>,
) -> Result<Json<ExampleResponse>, ApiError> {
    // ...
    todo!()
}

pub async fn update_example(
    State(ctx): State<AppContext>,
    AuthUser(user): AuthUser,
    Path(example_id): Path<Uuid>,
    Json(body): Json<UpdateExampleRequest>,
) -> Result<Json<ExampleResponse>, ApiError> {
    // ...
    todo!()
}

pub async fn delete_example(
    State(ctx): State<AppContext>,
    AuthUser(user): AuthUser,
    Path(example_id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    // ...
    todo!()
}
```

### Action Endpoint Template

```rust
pub async fn cancel_appointment(
    State(ctx): State<AppContext>,
    AuthUser(user): AuthUser,
    Path(appointment_id): Path<Uuid>,
    Json(body): Json<AppointmentActionRequest>, // includes optional reason
) -> Result<Json<AppointmentResponse>, ApiError> {
    // ...
    todo!()
}
```

---

## Development Workflow

### 1. Development Commands

```bash
# Build the project
cargo build

# Run in development (reads .env automatically via dotenvy)
cargo run

# Run with a specific log level
RUST_LOG=debug cargo run

# Run tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Check for warnings (fast, no binary produced)
cargo check

# Run clippy lints
cargo clippy -- -D warnings

# Format code
cargo fmt
```

### 2. Adding a New Handler

1. Create `src/handlers/{module}/mod.rs` with a `router()` function and handler functions
2. Create `src/handlers/{module}/repo.rs` with SeaORM query functions
3. Register the router in `src/handlers/mod.rs`
4. Add request/response types with `#[derive(Serialize, Deserialize, utoipa::ToSchema)]`

### 3. Type Safety Verification

- Compile-time type safety via Rust's type system — no separate type generation step needed
- Run `cargo check` for fast feedback without producing a binary
- Run `cargo test` to execute all unit and integration tests

### 4. Database Migrations

Migrations live in `src/generated/migrations/` and are applied at startup via SeaORM's migrator:

```bash
# Generate a new migration (from the TypeScript service)
# The Rust service applies migrations from the generated/ directory automatically on startup

# To run migrations manually via Sea-ORM CLI:
sea-orm-cli migrate up
sea-orm-cli migrate down
```

---

This document serves as the definitive reference for API development standards in the Monobase Healthcare Platform Rust service. All new handlers and modifications to existing handlers must follow these patterns to ensure consistency and maintainability.
