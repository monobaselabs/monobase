# CLAUDE.md

This file provides AI-specific guidance for Claude Code when working with the Monobase Application Platform.

## Documentation Map

For detailed information, refer to:
- **[README.md](./README.md)** - Project overview, installation, commands, technology stack
- **[CONTRIBUTING.md](./CONTRIBUTING.md)** - Development workflows, coding standards, testing guidelines

## Repository Overview

**Monobase Application Platform** - A comprehensive full-stack monorepo platform providing video sessions, service marketplace, and user management. Built with Bun runtime for 3x faster performance than Node.js.

**Key Technologies**: Bun, PostgreSQL, Drizzle ORM, Hono API, TypeSpec, TanStack Router, Better-Auth, OneSignal, S3/MinIO, Rust/Axum/SeaORM

**Monorepo Structure**:
- `apps/` - Frontend applications (currently only account app)
- `services/api/` - Backend API service — TypeScript (Hono + Bun)
- `services/api-rs/` - Backend API service — Rust (Axum + SeaORM), drop-in replacement
- `specs/api/` - TypeSpec API definitions
- `packages/` - Shared packages (typescript-config and ui)

## Business Domain Modules

The platform implements the following core modules:

1. **person** - User profile management and central PII safeguard
2. **booking** - Professional booking and scheduling system
3. **billing** - Invoice-based payments (Stripe integration)
4. **audit** - Compliance logging (Pino structured logging)
5. **notifs** - Multi-channel notifications (email, push via OneSignal)
6. **comms** - Video/chat sessions (WebRTC) and messaging
7. **storage** - File upload/download (S3/MinIO)
8. **email** - Transactional emails (SMTP/Postmark)
9. **reviews** - NPS review system

**Note**: Authentication is handled by Better-Auth (integrated, not a separate module). Consent management is implemented as JSONB fields on the Person model (not a standalone module).

## Key Architectural Patterns

### Person-Centric Design
The Person module is the central PII safeguard for user data.

### Consent Management
Consent is embedded in the Person model as JSONB fields rather than a standalone module:
```typescript
{
  granted: boolean,
  granted_at: timestamp,
  ip_address: string,
  updated_at: timestamp,
  updated_by: string
}
```

Consent types on Person:
- **marketing_consent**: Marketing communications
- **data_sharing_consent**: Data sharing preferences
- **sms_consent**: SMS notifications
- **email_consent**: Email communications

### API-First Development
Always follow this workflow:
1. Define APIs in TypeSpec (`specs/api/src/modules/`)
2. Generate OpenAPI + TypeScript types (`cd specs/api && bun run build`)
3. Generate routes/validators/handlers (`cd services/api && bun run generate`)
4. Implement handler business logic (`services/api/src/handlers/`)
5. Use generated types from `@monobase/api-spec` in frontends

**Why**: Type safety across frontend/backend, single source of truth, auto-generated docs

**⚠️ CRITICAL - Never Edit Generated Files**:
- `services/api/src/generated/openapi/*` - Routes, validators, registry (regenerated every time)
- `services/api/src/generated/better-auth/*` - Auth schema and specs
- `services/api/src/generated/migrations/*` - Database migrations

**✅ Only Edit**:
- TypeSpec files (`specs/api/src/modules/*.tsp`)
- Handler implementations (`services/api/src/handlers/{module}/*.ts`)
- Database schemas (`services/api/src/db/schema/*.ts`)

See [CONTRIBUTING.md#code-generation](./CONTRIBUTING.md#code-generation---do-not-edit) for complete details.

### Configuration Approach
Environment variables are parsed into typed configuration objects (see `services/api/src/core/config.ts`). Not file-based configuration.

### OneSignal Multi-App Architecture
OneSignal follows an **app-agnostic pattern** like other services (Storage, Email, Billing):

**Single App ID Approach**:
- Use the **same** `ONESIGNAL_APP_ID` across all frontends (client, service provider, website)
- Frontend apps: Set `VITE_ONESIGNAL_APP_ID` to the same value
- Backend API: Uses same app ID to send notifications

**Optional App Tagging**:
- Set `VITE_ONESIGNAL_APP_TAG=client` or `service_provider` in frontend .env (optional)
- Apps auto-tag themselves on initialization
- Most notifications ignore tags (app-agnostic)
- Use `targetApp` parameter only for app-specific announcements

**Why This Works**:
- OneSignal uses `external_id` (person ID) to target users across devices/apps
- Users with both client/service provider roles receive notifications in whichever app they're using
- Production deployment should use subdomains: `user.example.com`, `admin.example.com`

**API Pattern**:
```typescript
// Send to user (app-agnostic - default)
notificationRepo.createNotificationForModule({
  recipient: personId,
  type: 'booking-reminder',
  channel: 'push',
  // No targetApp - reaches user in any app
});

// Send only to specific app (rare)
notificationRepo.createNotificationForModule({
  recipient: personId,
  type: 'system',
  channel: 'push',
  targetApp: 'client', // Only if VITE_ONESIGNAL_APP_TAG is configured
});
```

### Module Structure Pattern

**TypeScript** (`services/api/`): Router → Validators → Service → Handlers
- Handler files (CRUD operations)
- `repos/` - Database repositories
- `jobs/` - Background job definitions
- `utils/` - Module-specific utilities

**Rust** (`services/api-rs/`): Router → Auth middleware → Handler → Repo
- `handlers/{module}/mod.rs` - Route handlers with auth + validation
- `handlers/{module}/repo.rs` - SQL queries via SeaORM
- `service/` - External integrations (Stripe, S3, email, notifications)
- `transport/` - HTTP (Axum) and IPC adapters

## Enterprise Compliance Requirements

When working with sensitive data:

### Data Privacy Compliance
- **Audit Trails**: All user data access must be logged with Pino
- **Consent Validation**: Check JSONB consent fields before processing
- **Role-Based Access**: Verify user roles via Better-Auth
- **Correlation IDs**: Include in all log entries for traceability

### Data Security
- Use Drizzle ORM for type-safe, SQL-injection-proof queries
- Validate all inputs with Zod schemas
- Never log sensitive personal information (PII) in plain text
- Follow secure patterns in existing handlers

## OpenAPI Specification

The canonical API reference is at: `specs/api/dist/openapi/openapi.json`

**Before implementing frontend features**:
1. Check the OpenAPI spec for endpoint definitions
2. Import TypeScript types from `@monobase/api-spec/types`
3. Validate your implementation matches the schema

**Helpful commands**: See [README.md#api-schema-reference](./README.md#api-schema-reference)

## Database Patterns

### Drizzle ORM Usage
- Use prepared statements for performance
- Leverage type inference from schema definitions
- Use transactions for multi-table operations
- Reference existing patterns in `services/api/src/handlers/*/repos/`

### Migration Workflow
1. Modify schema in `services/api/src/db/schema/`
2. Generate migration: `cd services/api && bun run db:generate`
3. Review generated SQL in `src/generated/migrations/`
4. Migrations run automatically on server start

**Details**: See [CONTRIBUTING.md#database-workflow](./CONTRIBUTING.md#database-workflow)

## Frontend Development

### Account App (Vite + TanStack Router)
- **Port**: 3002
- **Routing**: File-based in `src/routes/`
- **Auth**: Better-Auth with TanStack integration
- **Data Fetching**: TanStack Query with React Query
- **UI Components**: Radix UI primitives (shadcn/ui patterns)

**Standards**: See [CONTRIBUTING.md#coding-standards](./CONTRIBUTING.md#coding-standards)

## Testing Approach

- **API**: Bun test framework (`cd services/api && bun test`)
- **Frontend**: Playwright E2E tests (`cd apps/account && bun run test:e2e`)
- **Type Safety**: TypeScript checking across all workspaces

**Details**: See [CONTRIBUTING.md#testing-requirements](./CONTRIBUTING.md#testing-requirements)

## Common Commands Quick Reference

**Full command reference**: See [README.md#available-commands](./README.md#available-commands)

Essential commands:
```bash
# Install dependencies
bun install

# API-first workflow
cd specs/api && bun run build              # Generate OpenAPI + types
cd ../../services/api && bun run generate  # Generate routes/validators

# Start development (TypeScript API)
cd services/api && bun dev        # API on port 7213
cd apps/account && bun dev        # Account app on port 3002

# Start development (Rust API — same port, same API surface)
cd services/api-rs && cargo run   # API on port 7213

# Database
cd services/api && bun run db:generate  # Generate migration
cd services/api && bun run db:studio    # Open Drizzle Studio

# Testing
cd services/api && bun test             # API tests (TypeScript)
cd services/api-rs && cargo test        # API tests (Rust)
cd apps/account && bun run test:e2e     # E2E tests

# Rust code generation (from OpenAPI spec)
cd services/api-rs && npx tsx generator-rs.ts

# Rust release build
cd services/api-rs && cargo build --release          # Server binary (9.6 MB)
cd services/api-rs && cargo build --features embedded --profile release-embedded  # iOS/mobile
```

## Important Notes

### What Exists
- ✅ **packages/ui/** - Shared UI component library
- ✅ **packages/sdk/** - Type-safe API client
- ✅ **Authentication** via Better-Auth (integrated, not a separate module)
- ✅ **Consent** as JSONB fields on Person model (not a separate module)
- ✅ **9 Core Modules**: person, booking, billing, audit, notifs, comms, storage, email, reviews
- ✅ **services/api-rs/** - Native Rust port (Axum + SeaORM), all 68 routes, SQLite+PG, embedded/iOS support

### What Does Not Exist
- ❌ **apps/admin/** - No admin/service provider app yet
- ❌ **apps/website/** - No Next.js marketing website yet

## Rust API Service (`services/api-rs/`)

The Rust service is a drop-in replacement for the TypeScript service. Same API surface (68 routes), same database schema, same auth cookies.

**Key differences from TypeScript service**:
- **Dual DB**: Supports both PostgreSQL (server) and SQLite (embedded) via runtime dialect detection
- **Transport separation**: HTTP (Axum) and IPC (direct call) share the same service layer
- **Embedded mode**: `--features embedded` for Tauri/iOS, no HTTP/network dependencies
- **Auth in Rust**: Reads Better-Auth DB tables directly (bcrypt, HMAC session cookies)
- **No JS runtime deps**: Migrations are SQL files, auth is native, code generation is build-time only

**Structure**: `src/{auth, config, context, db, embedded, error, generated, handlers, middleware, model, service, transport}`

**Full spec**: See [API_REWRITE.md](./API_REWRITE.md)

## When in Doubt

1. Check [README.md](./README.md) for commands and setup
2. Check [CONTRIBUTING.md](./CONTRIBUTING.md) for development patterns
3. Reference existing handlers in `services/api/src/handlers/` (TypeScript) or `services/api-rs/src/handlers/` (Rust) for implementation patterns
4. Check OpenAPI spec at `specs/api/dist/openapi/openapi.json` for API contracts
5. Check [API_REWRITE.md](./API_REWRITE.md) for Rust service architecture and decisions
