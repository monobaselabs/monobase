# Monobase API E2E Test Suite

Language-agnostic E2E tests for the Monobase API. Works against **both** the TypeScript (Bun/Hono) and Rust (Axum/SeaORM) implementations.

All tests use real HTTP requests — no internal imports from either server.

## Quick Start

```bash
# Install dependencies
cd services/api-rs-tests
bun install

# Start test infrastructure (PostgreSQL, MinIO, Mailpit)
bun run deps:up

# Start the server you want to test (in another terminal):
# TypeScript: cd services/api && bun dev
# Rust:       cd services/api-rs && cargo run

# Run tests
bun test

# Or use the convenience scripts:
./scripts/run-against-rs.sh    # Sets env vars for Rust server
./scripts/run-against-ts.sh    # Sets env vars for TS server
```

## Configuration

| Variable | Default | Purpose |
|----------|---------|---------|
| `API_URL` | `http://localhost:7213` | Server base URL |
| `DATABASE_URL` | `postgresql://postgres:password@127.0.0.1:5432/monobase` | Direct DB access for cleanup |
| `AUTH_ADMIN_EMAILS` | (none) | Set to `admin-test@test.local` for admin tests |

## Test Modules

| Module | File | Routes Tested |
|--------|------|---------------|
| Health | `tests/health/` | GET /health, /livez, /readyz |
| Auth | `tests/auth/` | sign-up, sign-in, get-session, sign-out |
| Person | `tests/person/` | CRUD /persons |
| Patient | `tests/patient/` | CRUD /patients |
| Provider | `tests/provider/` | CRUD /providers |
| Booking | `tests/booking/` | Events, slots, bookings |
| Billing | `tests/billing/` | Invoices, merchant accounts, Stripe webhook |
| Comms | `tests/comms/` | Chat rooms, messages, ICE servers |
| EMR | `tests/emr/` | Consultations, finalize workflow |
| Email | `tests/email/` | Templates, queue (admin only) |
| Storage | `tests/storage/` | Upload, download, complete, delete |
| Notifs | `tests/notifs/` | List, mark-read |
| Reviews | `tests/reviews/` | CRUD, NPS validation |
| Audit | `tests/audit/` | List logs (admin only) |

## Architecture

```
src/
├── client.ts      # HTTP API client (fetch-based, no server imports)
├── auth.ts        # Auth helpers (signup, signin, admin client)
├── fixtures.ts    # Test data generators (create via API calls)
├── db.ts          # Direct PostgreSQL access for cleanup
└── setup.ts       # Global test preload

tests/
├── health/        # One test file per module
├── auth/
├── person/
├── patient/
└── ...
```

## Design Principles

1. **Language-agnostic**: Pure HTTP — works against any server implementing the API
2. **Self-contained**: Each test creates its own users and data
3. **Parallel-safe**: No shared global state between tests
4. **Data-aware assertions**: Find YOUR data by ID, don't assume list counts
