# Monobase API — Rust

Native Rust port of the Monobase API service. Drop-in replacement for `services/api` (TypeScript/Bun/Hono).

## Quick Start

```bash
# Development
cargo run

# Run unit tests
cargo test

# Run E2E tests (requires running server + deps)
cd tests && bun install && ./scripts/run-tests.sh

# Release build (9.6 MB binary)
cargo build --release

# Embedded/iOS build (size-optimized)
cargo build --features embedded --profile release-embedded
```

The server starts on port 7213 by default (same as the TypeScript service).

## Configuration

All configuration is via environment variables (same as the TypeScript service):

```bash
# Required
DATABASE_URL=postgres://postgres:password@localhost:5432/monobase

# Server
SERVER_PORT=7213
SERVER_HOST=0.0.0.0

# Auth
AUTH_SECRET=change-me-in-production

# Storage (S3/MinIO)
STORAGE_PROVIDER=minio
STORAGE_ENDPOINT=http://localhost:9000
STORAGE_BUCKET=monobase-files
STORAGE_ACCESS_KEY_ID=minioadmin
STORAGE_SECRET_ACCESS_KEY=minioadmin

# Email
EMAIL_PROVIDER=smtp
SMTP_HOST=127.0.0.1
SMTP_PORT=1025

# Billing (optional)
STRIPE_SECRET_KEY=sk_test_...
STRIPE_WEBHOOK_SECRET=whsec_...

# Notifications (optional)
ONESIGNAL_APP_ID=...
ONESIGNAL_API_KEY=...
```

See `src/config.rs` for all options with defaults, or run `cargo run -- --help`.

## Architecture

```
src/
├── main.rs              # Entry point, CLI, server startup
├── config.rs            # Environment variable parsing (clap)
├── error.rs             # ApiError enum + HTTP response mapping
├── context.rs           # Transport-agnostic ServiceContext
│
├── db/                  # Database connection (PG + SQLite)
├── model/               # Dialect abstraction, base CRUD, pagination
├── auth/                # Session validation, bcrypt, sign-up/in
├── middleware/           # Auth extraction from headers/cookies
│
├── handlers/            # Route handlers (13 business modules)
│   ├── person/          # mod.rs (handlers) + repo.rs (SQL queries)
│   ├── patient/
│   ├── provider/
│   ├── booking/
│   ├── billing/
│   ├── comms/
│   ├── emr/
│   ├── email/
│   ├── storage/
│   ├── notifs/
│   ├── reviews/
│   └── audit/
│
├── service/             # External integrations
│   ├── billing.rs       # Stripe (async-stripe)
│   ├── storage.rs       # S3/MinIO (aws-sdk-s3)
│   ├── email.rs         # SMTP (lettre) + Postmark + OneSignal
│   ├── notifs.rs        # OneSignal push notifications
│   ├── ws.rs            # WebSocket connection management
│   └── jobs.rs          # pg-boss compatible job scheduler
│
├── transport/
│   ├── http.rs          # Axum router (99 routes + middleware)
│   └── ipc.rs           # IPC structs for embedded mode
│
├── embedded/            # Tauri/iOS embedded runtime
│   ├── runtime.rs       # MonobaseEmbedded (SQLite lifecycle)
│   └── commands.rs      # Tauri IPC command handlers
│
└── generated/           # Auto-generated from OpenAPI spec
    ├── types.rs
    ├── enums.rs
    └── routes.rs
```

## Key Design Decisions

| Decision | Rationale |
|----------|-----------|
| **SeaORM** over SQLx/Diesel | Runtime PG + SQLite support without compile-time DB dependency |
| **Dialect enum** | Encapsulates all PG vs SQLite SQL differences in one place |
| **Transport separation** | Same service layer serves HTTP (Axum) and IPC (embedded) |
| **Auth in Rust** | Reads Better-Auth DB tables directly, bcrypt compatible |
| **No JS runtime deps** | Migrations are SQL files, auth is native, codegen is build-time only |

## Code Generation

```bash
# Generate Rust types/enums from OpenAPI spec
npx tsx generator-rs.ts
```

Reads `specs/api/dist/openapi/openapi.json` and generates `src/generated/`. Run after updating TypeSpec definitions.

## Docker

```bash
docker build -t monobase-api .
docker run -p 7213:7213 -e DATABASE_URL=postgres://... monobase-api
```

## Feature Flags

| Flag | Purpose |
|------|---------|
| `postgres` (default) | PostgreSQL support |
| `sqlite` (default) | SQLite support |
| `embedded` | Tauri/iOS embedded mode (adds rusqlite) |

## API Documentation

Interactive API docs available at `/docs` when the server is running. The OpenAPI spec is served at `/docs/openapi.json`.

## Full Specification

See [API_REWRITE.md](./API_REWRITE.md) for the complete rewrite specification including all 68 routes, database schemas, and architectural decisions.
