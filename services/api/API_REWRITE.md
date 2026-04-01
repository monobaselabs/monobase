# API Rewrite: Bun/TypeScript → Rust

> **Historical document**: This was the rewrite specification used when porting from TypeScript/Bun to Rust. The TypeScript service has been removed. This document is kept for architectural reference.

Port `services/api` to native Rust as `services/api-rs`. Reference architecture: `~/Projects/mycure/mono.rs/services/hapihub-rs`.

## Why Rust

- Embed in Tauri for desktop (macOS/Windows) and mobile (iOS) without QuickJS overhead
- 2-3x improvement in cold start, memory, and request latency
- Single binary deployment, no runtime dependencies
- Native SQLite support for offline-first embedded mode

## Key Architectural Decisions

| Concern | Decision | Rationale |
|---------|----------|-----------|
| DB support | SQLite + PostgreSQL, runtime detection | Embedded uses SQLite, server uses PG |
| Transport | HTTP (Axum) + IPC (direct call) | Same service layer, two adapters |
| iOS/embedded | `embedded` feature flag, `release-embedded` profile | No Axum/network in embedded mode |
| Auth | Fully in Rust, reading Better-Auth DB tables | No JS sidecar, bcrypt compatible |
| Migrations | SQL files run by sea-orm or sqlx | Drizzle-generated SQL is portable |
| Code generation | `generator-rs.ts` reads OpenAPI spec → Rust types/routes | Only JS at build time, not runtime |
| Query style | SeaORM + raw SQL where needed | Dialect enum abstracts PG/SQLite differences |

---

## Library Stack

| Concern | Library | Version |
|---------|---------|---------|
| Async runtime | tokio | 1.40+ (full) |
| Web framework | axum | 0.8 |
| Tower middleware | tower, tower-http | 0.5, 0.6 (CORS, trace, timeout, gzip) |
| ORM / DB | sea-orm | 1.1 (sqlx-postgres, sqlx-sqlite, runtime-tokio-native-tls) |
| Query building | sea-query | 0.32 |
| Auth sessions | bcrypt, jsonwebtoken | 0.16, 9 |
| Validation | garde | 0.21 (derive, email, url) |
| OpenAPI | utoipa, utoipa-axum, utoipa-scalar | 5.3, 0.2, 0.2 |
| Serialization | serde, serde_json | 1.0 |
| Errors | thiserror, anyhow | 2.0, 1.0 |
| Logging | tracing, tracing-subscriber | 0.1, 0.3 (env-filter, json) |
| HTTP client | reqwest | 0.12 (json) |
| S3 storage | aws-sdk-s3 | latest |
| Stripe | async-stripe | latest |
| Email | lettre | latest |
| UUID | uuid | 1 (v4, v7, serde) |
| Time | chrono | 0.4 (serde) |
| CLI | clap | 4.5 (derive, env) |
| URL encoding | urlencoding | 2 |
| Async traits | async-trait | 0.1 |
| Embedded SQLite | rusqlite | 0.32 (bundled, optional) |

---

## Project Structure

```
services/api-rs/
├── Cargo.toml
├── migrations/                     # SQL files (from Drizzle, run by sea-orm-migration)
├── generator-rs.ts                 # OpenAPI spec → Rust types/routes (build-time only)
├── src/
│   ├── main.rs                     # tokio entry, clap CLI, server startup
│   ├── lib.rs                      # module declarations
│   ├── config.rs                   # env-var config (clap + dotenvy)
│   ├── error.rs                    # ApiError enum + IntoResponse
│   │
│   ├── db/
│   │   └── mod.rs                  # connect() — dialect detection, pool config
│   │
│   ├── model/
│   │   ├── mod.rs                  # base CRUD model, PaginatedResult
│   │   ├── dialect.rs              # Dialect enum (PG vs SQLite SQL differences)
│   │   └── query.rs               # filtering, sorting, pagination translation
│   │
│   ├── auth/
│   │   ├── mod.rs
│   │   ├── backend.rs              # create_user, authenticate, get_user_by_token
│   │   ├── password.rs             # bcrypt hash/verify (Better-Auth compatible)
│   │   └── session.rs              # session CRUD, HMAC cookie verification
│   │
│   ├── context.rs                  # ServiceContext (transport-agnostic)
│   │
│   ├── service/
│   │   ├── mod.rs                  # CrudService (generic CRUD + domain hooks)
│   │   ├── billing.rs              # Stripe integration
│   │   ├── email.rs                # SMTP/Postmark/OneSignal sending
│   │   ├── storage.rs              # S3/MinIO presigned URLs
│   │   ├── notifs.rs               # OneSignal push + in-app
│   │   ├── ws.rs                   # WebSocket connection management
│   │   └── jobs.rs                 # pg-boss table poller / job scheduler
│   │
│   ├── handlers/
│   │   ├── mod.rs                  # AppState struct (DI container)
│   │   ├── crud.rs                 # generic CRUD handler utilities
│   │   ├── macros.rs               # crud_routes!, crud_handlers!
│   │   ├── person/
│   │   ├── patient/
│   │   ├── provider/
│   │   ├── booking/
│   │   ├── billing/
│   │   ├── audit/
│   │   ├── notifs/
│   │   ├── comms/
│   │   ├── storage/
│   │   ├── email/
│   │   ├── reviews/
│   │   └── emr/
│   │
│   ├── middleware/
│   │   └── auth.rs                 # extract_auth() from bearer/cookie
│   │
│   ├── transport/
│   │   ├── mod.rs
│   │   ├── http.rs                 # Axum router, route registration
│   │   └── ipc.rs                  # IpcRequest/IpcResponse (no HTTP deps)
│   │
│   ├── embedded/                   # feature = "embedded"
│   │   ├── mod.rs
│   │   ├── runtime.rs              # MonobaseEmbedded, SQLite lifecycle
│   │   └── commands.rs             # Tauri IPC command handlers
│   │
│   └── generated/                  # codegen output — DO NOT EDIT
│       ├── mod.rs
│       ├── types.rs                # request/response structs
│       ├── enums.rs                # all DB/API enums
│       └── routes.rs               # route registration function
```

### Cargo.toml Features & Profiles

```toml
[features]
default = ["postgres", "sqlite"]
postgres = []
sqlite = []
embedded = ["rusqlite"]

[profile.release]
opt-level = 3
lto = true
codegen-units = 1
strip = true

[profile.release-embedded]
inherits = "release"
opt-level = "z"  # minimize binary size for mobile
```

---

## Configuration (Environment Variables)

### Server
| Variable | Default | Type |
|----------|---------|------|
| `SERVER_PORT` / `PORT` | 7213 | int |
| `SERVER_HOST` | 0.0.0.0 | string |
| `SERVER_PUBLIC_URL` / `PUBLIC_URL` | — | string? |

### Database
| Variable | Default | Type |
|----------|---------|------|
| `DATABASE_URL` | postgres://postgres:password@localhost:5432/monobase | string |
| `DB_POOL_MIN` | 2 | int |
| `DB_POOL_MAX` | 20 | int |
| `DB_IDLE_TIMEOUT` | 30000 | int (ms) |
| `DB_SSL` | false | bool |
| `DB_LOGGING` | false | bool |

### CORS
| Variable | Default | Type |
|----------|---------|------|
| `CORS_ORIGINS` | * | comma-list |
| `CORS_CREDENTIALS` | true | bool |
| `CORS_ALLOW_LOCAL_NETWORK` | true | bool |
| `CORS_ALLOW_TUNNELING` | true | bool |
| `CORS_STRICT` | false | bool |

### Logging
| Variable | Default | Type |
|----------|---------|------|
| `LOG_LEVEL` | info | debug\|info\|warn\|error |
| `LOG_PRETTY` | true | bool |

### Auth
| Variable | Default | Type |
|----------|---------|------|
| `AUTH_BASE_URL` | derived from server | string |
| `AUTH_SECRET` | random (MUST set in prod) | string |
| `AUTH_SESSION_EXPIRES_IN` | 604800 (7 days) | int (seconds) |
| `AUTH_RATE_LIMIT_ENABLED` | true | bool |
| `AUTH_RATE_LIMIT_WINDOW` | 60 | int (seconds) |
| `AUTH_RATE_LIMIT_MAX` | 10 | int |
| `AUTH_ADMIN_EMAILS` | [] | comma-list |
| `GOOGLE_CLIENT_ID` | — | string? |
| `GOOGLE_CLIENT_SECRET` | — | string? |

### Storage
| Variable | Default | Type |
|----------|---------|------|
| `STORAGE_PROVIDER` | minio | minio\|s3 |
| `STORAGE_ENDPOINT` | http://localhost:9000 | string |
| `STORAGE_PUBLIC_ENDPOINT` | http://localhost:9000 | string |
| `STORAGE_BUCKET` | monobase-files | string |
| `STORAGE_REGION` | us-east-1 | string |
| `STORAGE_ACCESS_KEY_ID` | minioadmin | string |
| `STORAGE_SECRET_ACCESS_KEY` | minioadmin | string |
| `STORAGE_UPLOAD_URL_EXPIRY` | 300 | int (seconds) |
| `STORAGE_DOWNLOAD_URL_EXPIRY` | 900 | int (seconds) |

### Email
| Variable | Default | Type |
|----------|---------|------|
| `EMAIL_PROVIDER` | smtp | smtp\|postmark\|onesignal |
| `EMAIL_FROM_NAME` | Monobase | string |
| `EMAIL_FROM_EMAIL` | noreply@monobase.com | string |
| `SMTP_HOST` | 127.0.0.1 | string |
| `SMTP_PORT` | 1025 | int |
| `SMTP_SECURE` | false | bool |
| `SMTP_USER` | — | string |
| `SMTP_PASS` | — | string |
| `POSTMARK_API_KEY` | — | string? |
| `POSTMARK_MESSAGE_STREAM` | outbound | string |

### Notifications
| Variable | Default | Type |
|----------|---------|------|
| `ONESIGNAL_APP_ID` | — | string? |
| `ONESIGNAL_API_KEY` | — | string? |

### Billing
| Variable | Default | Type |
|----------|---------|------|
| `STRIPE_SECRET_KEY` | — | string? |
| `STRIPE_WEBHOOK_SECRET` | — | string? |
| `STRIPE_URL` | — | string? (test mock) |

### WebRTC
| Variable | Default | Type |
|----------|---------|------|
| `WEBRTC_ICE_SERVERS` | Google STUN servers | comma-list |

### Rate Limiting
| Variable | Default | Type |
|----------|---------|------|
| `RATE_LIMIT_ENABLED` | true | bool |
| `RATE_LIMIT_MAX` | 100 | int |

---

## Authentication

### Strategy

Implement auth fully in Rust by reading Better-Auth's existing DB tables directly. No JS sidecar.

### Session Validation Flow

1. Extract token from `better-auth.session_token` cookie OR `Authorization: Bearer {token}` header
2. URL-decode the value
3. Split on last `.` → `(raw_token, signature)`
4. Verify: `HMAC-SHA256(AUTH_SECRET, raw_token) == base64_decode(signature)`
5. Query: `SELECT * FROM session WHERE token = raw_token AND expires_at > NOW()`
6. Join: `SELECT * FROM "user" WHERE id = session.user_id`
7. Check `user.banned` is false (or `ban_expires` has passed)
8. Parse `user.role` as comma-separated string into role set

### Auth Endpoints to Implement

- `POST /auth/sign-up/email` — create user + account + session, bcrypt hash password
- `POST /auth/sign-in/email` — verify password, create session
- `POST /auth/sign-out` — delete session
- `POST /auth/get-session` — return current user + session
- Advanced (incremental): passkeys, 2FA/TOTP, magic links, email OTP, OAuth, API keys

### Auth DB Tables

**user**
| Column | Type | Notes |
|--------|------|-------|
| id | text PK | |
| name | text NOT NULL | |
| email | text NOT NULL UNIQUE | |
| email_verified | bool NOT NULL | default false |
| image | text | nullable |
| role | text | comma-separated, nullable |
| banned | bool | default false |
| ban_reason | text | nullable |
| ban_expires | timestamp | nullable |
| two_factor_enabled | bool | default false |
| created_at | timestamp NOT NULL | default now |
| updated_at | timestamp NOT NULL | default now |

**session**
| Column | Type | Notes |
|--------|------|-------|
| id | text PK | |
| token | text NOT NULL UNIQUE | random UUID |
| user_id | text NOT NULL | FK → user.id CASCADE |
| expires_at | timestamp NOT NULL | |
| ip_address | text | nullable |
| user_agent | text | nullable |
| impersonated_by | text | nullable |
| created_at | timestamp NOT NULL | default now |
| updated_at | timestamp NOT NULL | |

**account**
| Column | Type | Notes |
|--------|------|-------|
| id | text PK | |
| account_id | text NOT NULL | |
| provider_id | text NOT NULL | "credential" for email/password |
| user_id | text NOT NULL | FK → user.id CASCADE |
| password | text | bcrypt hash, nullable |
| access_token | text | nullable (OAuth) |
| refresh_token | text | nullable (OAuth) |
| id_token | text | nullable (OAuth) |
| access_token_expires_at | timestamp | nullable |
| refresh_token_expires_at | timestamp | nullable |
| scope | text | nullable |
| created_at | timestamp NOT NULL | default now |
| updated_at | timestamp NOT NULL | |

**verification**
| Column | Type | Notes |
|--------|------|-------|
| id | text PK | |
| identifier | text NOT NULL | |
| value | text NOT NULL | |
| expires_at | timestamp NOT NULL | |
| created_at | timestamp NOT NULL | default now |
| updated_at | timestamp NOT NULL | default now |

**passkey**
| Column | Type | Notes |
|--------|------|-------|
| id | text PK | |
| name | text | nullable |
| public_key | text NOT NULL | |
| user_id | text NOT NULL | FK → user.id CASCADE |
| credential_id | text NOT NULL | |
| counter | int NOT NULL | |
| device_type | text NOT NULL | |
| backed_up | bool NOT NULL | |
| transports | text | nullable |
| created_at | timestamp | nullable |
| aaguid | text | nullable |

**two_factor**
| Column | Type | Notes |
|--------|------|-------|
| id | text PK | |
| secret | text NOT NULL | |
| backup_codes | text NOT NULL | |
| user_id | text NOT NULL | FK → user.id CASCADE |

**apikey**
| Column | Type | Notes |
|--------|------|-------|
| id | text PK | |
| name | text | nullable |
| start | text | nullable |
| prefix | text | nullable |
| key | text NOT NULL | |
| user_id | text NOT NULL | FK → user.id CASCADE |
| refill_interval | int | nullable |
| refill_amount | int | nullable |
| last_refill_at | timestamp | nullable |
| enabled | bool | default true |
| rate_limit_enabled | bool | default true |
| rate_limit_time_window | int | default 86400000 |
| rate_limit_max | int | default 10 |
| request_count | int | default 0 |
| remaining | int | nullable |
| last_request | timestamp | nullable |
| expires_at | timestamp | nullable |
| created_at | timestamp NOT NULL | |
| updated_at | timestamp NOT NULL | |
| permissions | text | nullable |
| metadata | text | nullable |

---

## Database Schema

### Base Entity Fields (all domain tables)

| Column | Type | Notes |
|--------|------|-------|
| id | uuid PK | default gen_random_uuid() |
| created_at | timestamp NOT NULL | default now |
| updated_at | timestamp NOT NULL | default now |
| version | int NOT NULL | default 1 (optimistic locking) |
| created_by | uuid | nullable |
| updated_by | uuid | nullable |

### Dialect Abstraction

Runtime detection from `DATABASE_URL` scheme. `Dialect` enum encapsulates:

| Method | PostgreSQL | SQLite |
|--------|-----------|--------|
| `now_expr()` | `NOW()` | `datetime('now')` |
| `json_cast()` | `::jsonb` | (empty) |
| `json_extract_text(col, key)` | `col->>'key'` | `json_extract(col, '$.key')` |
| `json_array_contains(col, k, v)` | `col @> '[{"k":v}]'::jsonb` | `EXISTS (SELECT 1 FROM json_each(col) WHERE json_extract(value, '$.k') = v)` |
| `json_scalar_overlap(col, vals)` | `col ?| array[...]::text[]` | `EXISTS (SELECT 1 FROM json_each(col) WHERE value IN (...))` |
| `regex_op(case_insensitive)` | `~*` / `~` | `LIKE` |
| `supports_returning()` | true | true (3.35+) |

### Enum Types

```
-- Audit
audit_action: create, read, update, delete, login, logout
audit_category: hipaa, security, privacy, administrative, clinical, financial
audit_event_type: authentication, data-access, data-modification, system-config, security, compliance
audit_outcome: success, failure, partial, denied
audit_retention_status: active, archived, pending-purge

-- Billing
capture_method: automatic, manual
invoice_status: draft, open, paid, void, uncollectible
payment_status: pending, requires_capture, processing, succeeded, failed, canceled

-- Booking
booking_event_status: draft, active, paused, archived
booking_status: pending, confirmed, rejected, cancelled, completed, no_show_client, no_show_provider
location_type: video, phone, in-person
slot_status: available, booked, blocked
recurrence_type: (defined in recurrence_pattern jsonb)

-- Comms
chat_room_status: active, archived
message_type: text, system, video_call

-- Email
email_provider: smtp, postmark, onesignal
email_queue_status: pending, processing, sent, failed, cancelled
template_status: draft, active, archived

-- EMR
consultation_status: draft, finalized, amended

-- Notifications
notification_channel: email, push, in-app
notification_status: queued, sent, delivered, read, failed, expired
notification_type: billing, security, system, booking-reminder, booking-confirmed,
                   booking-cancelled, booking-rejected, booking-completed,
                   comms-new-message, comms-video-call

-- Person
gender: male, female, non-binary, other, prefer-not-to-say

-- Storage
file_status: uploading, processing, available, failed
```

### Domain Tables

#### persons
| Column | Type | Notes |
|--------|------|-------|
| + base entity fields | | |
| first_name | varchar(50) NOT NULL | |
| last_name | varchar(50) | nullable |
| middle_name | varchar(50) | nullable |
| date_of_birth | date | nullable |
| gender | gender enum | nullable |
| primary_address | jsonb | nullable |
| contact_info | jsonb | nullable |
| avatar | jsonb | nullable |
| languages_spoken | jsonb (string[]) | nullable |
| timezone | varchar(50) | nullable |
| **Indexes**: name (first_name, last_name) | | |

#### patients
| Column | Type | Notes |
|--------|------|-------|
| + base entity fields | | |
| person_id | uuid NOT NULL UNIQUE | FK → persons.id CASCADE |
| primary_provider | jsonb | nullable |
| primary_pharmacy | jsonb | nullable |

#### providers
| Column | Type | Notes |
|--------|------|-------|
| + base entity fields | | |
| person_id | uuid NOT NULL UNIQUE | FK → persons.id CASCADE |
| provider_type | varchar(50) NOT NULL | |
| years_of_experience | int | nullable |
| biography | text | nullable |
| minor_ailments_specialties | jsonb (string[]) | nullable |
| minor_ailments_practice_locations | jsonb (string[]) | nullable |
| **Indexes**: person_id, provider_type | | |

#### booking_events
| Column | Type | Notes |
|--------|------|-------|
| + base entity fields | | |
| owner_id | uuid NOT NULL | FK → persons.id CASCADE |
| context_id | text | nullable |
| title | text NOT NULL | |
| description | text | nullable |
| keywords | jsonb (string[]) | default '[]' |
| tags | jsonb (string[]) | default '[]' |
| timezone | text NOT NULL | default 'America/New_York' |
| location_types | jsonb (string[]) | default all |
| max_booking_days | int NOT NULL | default 30, range 0-365 |
| min_booking_minutes | int NOT NULL | default 1440, range 0-4320 |
| form_config | jsonb | nullable |
| billing_config | jsonb | nullable |
| status | booking_event_status | default 'active' |
| effective_from | timestamp NOT NULL | default now |
| effective_to | timestamp | nullable |
| daily_configs | jsonb NOT NULL | day-of-week → config map |
| **Indexes**: owner, context, status, GIN(keywords), GIN(tags), search | | |

#### time_slots
| Column | Type | Notes |
|--------|------|-------|
| + base entity fields | | |
| owner_id | uuid NOT NULL | FK → persons.id CASCADE |
| event_id | uuid NOT NULL | FK → booking_events.id CASCADE |
| context | text | nullable |
| start_time | timestamp NOT NULL | |
| end_time | timestamp NOT NULL | |
| location_types | jsonb (string[]) NOT NULL | |
| status | slot_status | default 'available' |
| billing_config | jsonb | nullable |
| booking_id | uuid | FK → bookings.id SET NULL, nullable |
| **Constraints**: UNIQUE(event_id, start_time) | | |

#### bookings
| Column | Type | Notes |
|--------|------|-------|
| + base entity fields | | |
| client_id | uuid NOT NULL | FK → persons.id CASCADE |
| provider_id | uuid NOT NULL | FK → persons.id CASCADE |
| slot_id | uuid NOT NULL | FK → time_slots.id CASCADE |
| location_type | location_type NOT NULL | |
| reason | text | nullable, max 500 |
| status | booking_status | default 'pending' |
| booked_at | timestamp NOT NULL | default now |
| confirmation_timestamp | timestamp | nullable |
| scheduled_at | timestamp NOT NULL | |
| duration_minutes | int NOT NULL | range 15-480 |
| cancellation_reason | text | nullable |
| cancelled_by | text | 'client' or 'provider' |
| cancelled_at | timestamp | nullable |
| no_show_marked_by | text | 'client' or 'provider' |
| no_show_marked_at | timestamp | nullable |
| form_responses | jsonb | nullable |
| invoice_id | uuid | nullable |

#### schedule_exceptions
| Column | Type | Notes |
|--------|------|-------|
| + base entity fields | | |
| event_id | uuid NOT NULL | FK → booking_events.id CASCADE |
| owner_id | uuid NOT NULL | FK → persons.id CASCADE |
| context_id | text | nullable |
| timezone | text NOT NULL | default 'America/New_York' |
| start_datetime | timestamp NOT NULL | |
| end_datetime | timestamp NOT NULL | |
| reason | text NOT NULL | max 500 |
| recurring | bool NOT NULL | default false |
| recurrence_pattern | jsonb | nullable |

#### invoices
| Column | Type | Notes |
|--------|------|-------|
| + base entity fields | | |
| invoice_number | varchar(50) NOT NULL UNIQUE | |
| customer_id | uuid NOT NULL | FK → persons.id CASCADE |
| merchant_id | uuid NOT NULL | FK → persons.id CASCADE |
| merchant_account_id | uuid | FK → merchant_accounts.id SET NULL |
| context | varchar(255) UNIQUE | nullable |
| status | invoice_status | default 'draft' |
| subtotal | int NOT NULL | cents |
| tax | int | cents, nullable |
| total | int NOT NULL | cents |
| currency | varchar(3) | default 'USD' |
| payment_capture_method | capture_method | default 'automatic' |
| payment_due_at | timestamp | nullable |
| payment_status | payment_status | nullable |
| paid_at | timestamp | nullable |
| paid_by | uuid | nullable |
| voided_at | timestamp | nullable |
| voided_by | uuid | nullable |
| void_threshold_minutes | int | nullable |
| authorized_at | timestamp | nullable |
| authorized_by | uuid | nullable |
| metadata | jsonb | nullable |

#### invoice_line_items
| Column | Type | Notes |
|--------|------|-------|
| + base entity fields | | |
| invoice_id | uuid NOT NULL | FK → invoices.id CASCADE |
| description | varchar(500) NOT NULL | |
| quantity | int NOT NULL | default 1, min 1 |
| unit_price | int NOT NULL | cents |
| amount | int NOT NULL | cents |
| metadata | jsonb | nullable |

#### merchant_accounts
| Column | Type | Notes |
|--------|------|-------|
| + base entity fields | | |
| person_id | uuid NOT NULL UNIQUE | FK → persons.id CASCADE |
| active | bool NOT NULL | default true |
| metadata | jsonb NOT NULL | |

#### chat_rooms
| Column | Type | Notes |
|--------|------|-------|
| + base entity fields | | |
| participants | jsonb (string[]) NOT NULL | |
| admins | jsonb (string[]) NOT NULL | |
| context_id | text | nullable |
| status | chat_room_status | default 'active' |
| last_message_at | timestamp | nullable |
| message_count | int NOT NULL | default 0 |
| active_video_call_message_id | uuid | nullable |
| **Indexes**: GIN(participants), GIN(admins) | | |

#### chat_messages
| Column | Type | Notes |
|--------|------|-------|
| + base entity fields | | |
| chat_room_id | uuid NOT NULL | FK → chat_rooms.id CASCADE |
| sender_id | uuid NOT NULL | |
| timestamp | timestamp NOT NULL | default now |
| message_type | message_type NOT NULL | |
| message | text | nullable, max 5000 |
| video_call_data | jsonb | nullable |

#### email_templates
| Column | Type | Notes |
|--------|------|-------|
| + base entity fields | | |
| name | varchar(255) NOT NULL | |
| description | text | nullable |
| subject | varchar(500) NOT NULL | |
| body_html | text NOT NULL | |
| body_text | text | nullable |
| tags | jsonb (string[]) | nullable |
| variables | jsonb NOT NULL | template variable definitions |
| from_name | varchar(255) | nullable |
| from_email | varchar(255) | nullable |
| reply_to_email | varchar(255) | nullable |
| reply_to_name | varchar(255) | nullable |
| status | template_status | default 'draft' |

#### email_queue
| Column | Type | Notes |
|--------|------|-------|
| + base entity fields | | |
| template_id | uuid | FK → email_templates.id SET NULL |
| template_tags | jsonb (string[]) | nullable |
| recipient_email | varchar(255) NOT NULL | |
| recipient_name | varchar(255) | nullable |
| variables | jsonb NOT NULL | |
| metadata | jsonb | nullable |
| status | email_queue_status | default 'pending' |
| priority | int NOT NULL | default 5 |
| scheduled_at | timestamp | nullable |
| attempts | int NOT NULL | default 0 |
| last_attempt_at | timestamp | nullable |
| next_retry_at | timestamp | nullable |
| last_error | text | nullable |
| sent_at | timestamp | nullable |
| provider | email_provider | nullable |
| provider_message_id | varchar(255) | nullable |
| cancelled_at | timestamp | nullable |
| cancelled_by | uuid | nullable |
| cancellation_reason | text | nullable |

#### audit_log_entries
| Column | Type | Notes |
|--------|------|-------|
| + base entity fields | | |
| event_type | audit_event_type NOT NULL | |
| category | audit_category NOT NULL | |
| action | audit_action NOT NULL | |
| outcome | audit_outcome NOT NULL | |
| user_id | uuid | nullable |
| user_type | varchar(20) | nullable |
| resource_type | varchar(100) NOT NULL | |
| resource | varchar(255) NOT NULL | |
| description | varchar(1000) NOT NULL | |
| details | jsonb | nullable |
| ip_address | varchar(45) | nullable |
| user_agent | varchar(500) | nullable |
| session_id | varchar(255) | nullable |
| request_id | varchar(255) | nullable |
| integrity_hash | varchar(64) | nullable |
| retention_status | audit_retention_status | default 'active' |
| archived_at | timestamp | nullable |
| archived_by | text | nullable |
| purge_after | timestamp | nullable |

#### notifications
| Column | Type | Notes |
|--------|------|-------|
| + base entity fields | | |
| recipient_id | uuid NOT NULL | |
| type | notification_type NOT NULL | |
| channel | notification_channel NOT NULL | |
| title | varchar(200) NOT NULL | |
| message | varchar(1000) NOT NULL | |
| scheduled_at | timestamp | nullable |
| related_entity_type | varchar(50) | nullable |
| related_entity_id | uuid | nullable |
| status | notification_status | default 'queued' |
| sent_at | timestamp | nullable |
| read_at | timestamp | nullable |
| consent_validated | bool NOT NULL | default false |

#### consultation_notes
| Column | Type | Notes |
|--------|------|-------|
| + base entity fields | | |
| patient_id | uuid NOT NULL | FK → patients.id CASCADE |
| provider_id | uuid NOT NULL | FK → providers.id CASCADE |
| context | varchar(255) UNIQUE | nullable |
| chief_complaint | text | nullable, max 500 |
| assessment | text | nullable, max 2000 |
| plan | text | nullable, max 2000 |
| vitals | jsonb | nullable |
| symptoms | jsonb | nullable |
| prescriptions | jsonb (array) | nullable |
| follow_up | jsonb | nullable |
| external_documentation | jsonb | nullable |
| status | consultation_status | default 'draft' |
| finalized_at | timestamp | nullable |
| finalized_by | uuid | nullable |

#### reviews
| Column | Type | Notes |
|--------|------|-------|
| + base entity fields | | |
| context_id | uuid NOT NULL | |
| reviewer_id | uuid NOT NULL | FK → persons.id CASCADE |
| review_type | varchar(50) NOT NULL | |
| reviewed_entity_id | uuid | FK → persons.id CASCADE, nullable |
| nps_score | int NOT NULL | range 0-10 |
| comment | text | nullable, max 1000 |
| **Constraints**: UNIQUE(context_id, reviewer_id, review_type) | | |

#### stored_files
| Column | Type | Notes |
|--------|------|-------|
| + base entity fields | | |
| filename | varchar(255) NOT NULL | |
| mime_type | varchar(100) NOT NULL | |
| size | bigint NOT NULL | |
| status | file_status | default 'uploading' |
| owner_id | uuid NOT NULL | |
| uploaded_at | timestamp | default now |

---

## API Routes (68 total)

### Person (`/persons`) — 4 routes
| Method | Path | Handler | Auth |
|--------|------|---------|------|
| POST | /persons | createPerson | user |
| GET | /persons | listPersons | admin, support |
| GET | /persons/:person | getPerson | admin, support, user:owner |
| PATCH | /persons/:person | updatePerson | user:owner |

### Patient (`/patients`) — 5 routes
| Method | Path | Handler | Auth |
|--------|------|---------|------|
| POST | /patients | createPatient | user |
| GET | /patients | listPatients | admin, support |
| GET | /patients/:patient | getPatient | admin, support, patient:owner |
| PATCH | /patients/:patient | updatePatient | patient:owner |
| DELETE | /patients/:patient | deletePatient | patient:owner |

### Provider (`/providers`) — 5 routes
| Method | Path | Handler | Auth |
|--------|------|---------|------|
| POST | /providers | createProvider | user |
| GET | /providers | listProviders | authenticated |
| GET | /providers/:provider | getProvider | authenticated |
| PATCH | /providers/:provider | updateProvider | provider:owner |
| DELETE | /providers/:provider | deleteProvider | provider:owner |

### Booking (`/booking`) — 16 routes
| Method | Path | Handler | Auth |
|--------|------|---------|------|
| POST | /booking/bookings | createBooking | user |
| GET | /booking/bookings | listBookings | client:owner, provider:owner, admin, support |
| GET | /booking/bookings/:booking | getBooking | client:owner, provider:owner, admin, support |
| POST | /booking/bookings/:booking/cancel | cancelBooking | client:owner, provider:owner, admin |
| POST | /booking/bookings/:booking/confirm | confirmBooking | provider:owner, admin |
| POST | /booking/bookings/:booking/no-show | markNoShowBooking | client:owner, provider:owner, admin |
| POST | /booking/bookings/:booking/reject | rejectBooking | provider:owner, admin |
| GET | /booking/events | listBookingEvents | optional |
| POST | /booking/events | createBookingEvent | user |
| GET | /booking/events/:event | getBookingEvent | optional |
| PATCH | /booking/events/:event | updateBookingEvent | event:owner, admin |
| DELETE | /booking/events/:event | deleteBookingEvent | event:owner, admin |
| POST | /booking/events/:event/exceptions | createScheduleException | event:owner, admin |
| GET | /booking/events/:event/exceptions | listScheduleExceptions | event:owner, admin, support |
| GET | /booking/events/:event/exceptions/:exception | getScheduleException | event:owner, admin, support |
| DELETE | /booking/events/:event/exceptions/:exception | deleteScheduleException | event:owner, admin |
| GET | /booking/events/:event/slots | listEventSlots | optional |
| GET | /booking/slots/:slotId | getTimeSlot | authenticated |

### Billing (`/billing`) — 17 routes
| Method | Path | Handler | Auth |
|--------|------|---------|------|
| POST | /billing/invoices | createInvoice | authenticated |
| GET | /billing/invoices | listInvoices | authenticated |
| GET | /billing/invoices/:invoice | getInvoice | authenticated |
| PATCH | /billing/invoices/:invoice | updateInvoice | authenticated |
| DELETE | /billing/invoices/:invoice | deleteInvoice | authenticated |
| POST | /billing/invoices/:invoice/capture | captureInvoicePayment | authenticated |
| POST | /billing/invoices/:invoice/finalize | finalizeInvoice | authenticated |
| POST | /billing/invoices/:invoice/mark-uncollectible | markInvoiceUncollectible | authenticated |
| POST | /billing/invoices/:invoice/pay | payInvoice | authenticated |
| POST | /billing/invoices/:invoice/refund | refundInvoicePayment | authenticated |
| POST | /billing/invoices/:invoice/void | voidInvoice | authenticated |
| POST | /billing/merchant-accounts | createMerchantAccount | authenticated |
| GET | /billing/merchant-accounts/:merchantAccount | getMerchantAccount | authenticated |
| POST | /billing/merchant-accounts/:merchantAccount/dashboard | getMerchantDashboard | authenticated |
| POST | /billing/merchant-accounts/:merchantAccount/onboard | onboardMerchantAccount | authenticated |
| POST | /billing/webhooks/stripe | handleStripeWebhook | none (public) |

### Communications (`/comms`) — 10 routes
| Method | Path | Handler | Auth |
|--------|------|---------|------|
| POST | /comms/chat-rooms | createChatRoom | user |
| GET | /comms/chat-rooms | listChatRooms | user:participant |
| GET | /comms/chat-rooms/:room | getChatRoom | user:participant |
| GET | /comms/chat-rooms/:room/messages | getChatMessages | user:participant |
| POST | /comms/chat-rooms/:room/messages | sendChatMessage | user:participant |
| POST | /comms/chat-rooms/:room/video-call/end | endVideoCall | user:admin |
| POST | /comms/chat-rooms/:room/video-call/join | joinVideoCall | user:participant |
| POST | /comms/chat-rooms/:room/video-call/leave | leaveVideoCall | user:participant |
| PATCH | /comms/chat-rooms/:room/video-call/participant | updateVideoCallParticipant | user:participant |
| GET | /comms/ice-servers | getIceServers | user |

### Email (`/email`) — 9 routes
| Method | Path | Handler | Auth |
|--------|------|---------|------|
| GET | /email/queue | listEmailQueueItems | admin |
| GET | /email/queue/:queue | getEmailQueueItem | admin |
| POST | /email/queue/:queue/cancel | cancelEmailQueueItem | admin |
| POST | /email/queue/:queue/retry | retryEmailQueueItem | admin |
| GET | /email/templates | listEmailTemplates | admin |
| POST | /email/templates | createEmailTemplate | admin |
| GET | /email/templates/:template | getEmailTemplate | admin |
| PATCH | /email/templates/:template | updateEmailTemplate | admin |
| POST | /email/templates/:template/test | testEmailTemplate | admin |

### EMR (`/emr`) — 6 routes
| Method | Path | Handler | Auth |
|--------|------|---------|------|
| POST | /emr/consultations | createConsultation | provider |
| GET | /emr/consultations | listConsultations | provider, admin, patient |
| GET | /emr/consultations/:consultation | getConsultation | admin, provider:owner, patient:owner |
| PATCH | /emr/consultations/:consultation | updateConsultation | provider:owner |
| POST | /emr/consultations/:consultation/finalize | finalizeConsultation | provider:owner |
| GET | /emr/patients | listEMRPatients | provider, admin |

### Notifications (`/notifs`) — 4 routes
| Method | Path | Handler | Auth |
|--------|------|---------|------|
| GET | /notifs | listNotifications | user, admin |
| POST | /notifs/read-all | markAllNotificationsAsRead | user |
| GET | /notifs/:notif | getNotification | user, admin |
| POST | /notifs/:notif/read | markNotificationAsRead | user |

### Reviews (`/reviews`) — 4 routes
| Method | Path | Handler | Auth |
|--------|------|---------|------|
| POST | /reviews/ | createReview | user |
| GET | /reviews/ | listReviews | user |
| GET | /reviews/:review | getReview | user |
| DELETE | /reviews/:review | deleteReview | review:owner, admin |

### Storage (`/storage`) — 6 routes
| Method | Path | Handler | Auth |
|--------|------|---------|------|
| GET | /storage/files | listFiles | authenticated |
| POST | /storage/files/upload | uploadFile | user |
| GET | /storage/files/:file | getFile | admin, user:owner |
| DELETE | /storage/files/:file | deleteFile | user:owner |
| POST | /storage/files/:file/complete | completeFileUpload | user:owner |
| GET | /storage/files/:file/download | getFileDownload | admin, user:owner |

### Audit (`/audit`) — 1 route
| Method | Path | Handler | Auth |
|--------|------|---------|------|
| GET | /audit/logs | listAuditLogs | admin, support |

### Health (internal) — not counted
| Method | Path | Handler | Auth |
|--------|------|---------|------|
| GET | /health | healthCheck | none |
| GET | /livez | liveness | none |
| GET | /readyz | readiness | none |

### WebSocket
| Path | Purpose |
|------|---------|
| /ws/user | Personal notifications, booking updates |
| /ws/comms/chat-rooms/:room | Chat messages, typing indicators, video signaling |

---

## Error Handling

### ApiError Enum

```rust
pub enum ApiError {
    // Standard HTTP errors
    NotFound { message, resource_type?, resource?, suggestions? }        // 404, NOT_FOUND
    Unauthorized { message }                                              // 401, UNAUTHORIZED
    Forbidden { message }                                                 // 403, FORBIDDEN
    Validation { message, field_errors, global_errors? }                  // 400, VALIDATION_ERROR
    BusinessLogic { message, code }                                       // 422, custom code
    Conflict { message }                                                  // 409, CONFLICT
    RateLimit { message, limit_type?, limit?, reset_time? }               // 429, RATE_LIMIT
    Timeout { message, timeout_ms, operation?, retryable }                // 408, TIMEOUT_ERROR
    ExternalService { message, service, operation?, retryable }           // 503, EXTERNAL_SERVICE_ERROR

    // Domain-specific
    Authentication { message, scheme?, supported_schemes? }               // 401, AUTHENTICATION_ERROR
    Authorization { message, required_permission?, resource? }            // 403, AUTHORIZATION_ERROR
    HipaaCompliance { message, hipaa_rule, violation_type }               // 400, HIPAA_COMPLIANCE_ERROR

    // Internal
    Internal { message }                                                  // 500, INTERNAL_ERROR
    Database(sea_orm::DbErr)                                              // 500
    Anyhow(anyhow::Error)                                                 // 500
}
```

### Error Response Format

```json
{
  "message": "Human-readable error message",
  "code": "MACHINE_ERROR_CODE",
  "statusCode": 400,
  "requestId": "uuid",
  "timestamp": "2026-04-01T00:00:00.000Z",
  "fieldErrors": [
    { "field": "email", "code": "invalid_string", "message": "Invalid email", "value": "bad" }
  ],
  "globalErrors": ["Some global error"]
}
```

---

## Transport Layer

### HTTP (server mode)

Axum router with tower middleware:
1. `TraceLayer` — request/response tracing with request ID
2. `CorsLayer` — configurable origin validation
3. `TimeoutLayer` — request timeout
4. `CompressionLayer` — gzip response compression
5. Auth extraction per-handler (not global middleware)

State via `Arc<AppState>`:
```rust
pub struct AppState {
    pub db: DatabaseConnection,
    pub config: Config,
    pub dialect: Dialect,
    // Services
    pub billing: BillingService,
    pub email: EmailService,
    pub storage: StorageService,
    pub notifs: NotificationService,
    pub ws: WebSocketService,
    pub jobs: JobScheduler,
}
```

### IPC (embedded mode)

```rust
pub struct IpcRequest {
    pub method: String,
    pub path: String,
    pub body: Option<serde_json::Value>,
    pub headers: HashMap<String, String>,
}

pub struct IpcResponse {
    pub status: u16,
    pub body: serde_json::Value,
}
```

Tauri commands:
- `monobase_init(db_path)` — initialize embedded instance with SQLite
- `monobase_request(IpcRequest)` — dispatch to service layer
- `monobase_checkpoint()` — force WAL checkpoint (iOS lifecycle)
- `monobase_shutdown()` — graceful shutdown

### ServiceContext (shared across transports)

```rust
pub struct ServiceContext {
    pub user: Option<AuthUser>,
    pub query: serde_json::Value,
    pub params: HashMap<String, String>,
    pub provider: String, // "rest" or "ipc"
}
```

---

## Background Jobs

Poll `pgboss.job` table with `SELECT ... FOR UPDATE SKIP LOCKED`:

| Job | Schedule | Purpose |
|-----|----------|---------|
| email.processor | on trigger | Process pending email queue |
| notifs.processScheduled | */5 * * * * | Deliver due scheduled notifications |
| notifs.cleanup | 0 0 * * * | Remove notifications > 90 days |
| audit.archive | configurable | Archive old audit entries |
| booking.reminders | configurable | Send booking reminders |

---

## Implementation Phases

### Phase 1: Foundation
- Cargo project, feature flags, profiles
- `db/`: connect(), dialect detection (PG + SQLite)
- `model/`: Dialect enum, base CRUD, PaginatedResult
- `config.rs`: clap CLI + env vars (all variables above)
- `error.rs`: ApiError enum + IntoResponse
- `auth/`: HMAC session verification, sign-up, sign-in, bcrypt
- `transport/http.rs`: Axum router skeleton
- `transport/ipc.rs`: IpcRequest/IpcResponse
- Copy SQL migrations
- Port **person** module end-to-end (4 routes)

### Phase 2: Core Modules
- **patient** (5 routes), **provider** (5 routes)
- **reviews** (4 routes), **audit** (1 route)
- Code generator script (OpenAPI → Rust types)
- `crud_routes!` macro

### Phase 3: Complex Modules
- **booking** (16 routes) — state machine, slot generation
- **billing** (17 routes) — Stripe, webhooks, invoices
- **comms** (10 routes) — chat, WebSocket, video signaling
- **emr** (6 routes) — consultations, finalize workflow
- **email** (9 routes) — queue, templates, multi-provider
- **storage** (6 routes) — S3 presigned URLs
- **notifs** (4 routes) — OneSignal, in-app, WebSocket
- Job scheduler, WebSocket service

### Phase 4: Embedded + Integration
- `embedded/`: MonobaseEmbedded, Tauri commands, SQLite lifecycle
- `release-embedded` profile testing
- E2E testing, Docker config

---

## Verification

1. Diff utoipa-generated OpenAPI spec against `specs/api/dist/openapi/openapi.json`
2. Test bcrypt hash compatibility (Better-Auth ↔ Rust)
3. Test HMAC cookie verification matches Better-Auth signing
4. Run existing Playwright E2E tests against Rust server
5. Rust unit tests with `sqlx::test` / test transactions
6. Compile `--features embedded --profile release-embedded`, test IPC
7. Run CRUD tests against both PG and SQLite

---

## Files to Reference

| Purpose | Path |
|---------|------|
| Current app setup | `services/api/src/app.ts` |
| Config / env vars | `services/api/src/core/config.ts` |
| Auth middleware | `services/api/src/middleware/auth.ts` |
| Better-Auth config | `services/api/src/core/auth.ts` |
| Auth DB schema | `services/api/src/generated/better-auth/schema.ts` |
| Error types | `services/api/src/core/errors.ts` |
| Base repository | `services/api/src/core/database.repo.ts` |
| Route registry | `services/api/src/generated/openapi/routes.ts` |
| Validators (Zod) | `services/api/src/generated/openapi/validators.ts` |
| Job scheduler | `services/api/src/core/jobs.ts` |
| WebSocket service | `services/api/src/core/ws.ts` |
| OpenAPI spec | `specs/api/dist/openapi/openapi.json` |
| **Reference Rust** | `~/Projects/mycure/mono.rs/services/hapihub-rs/` |
| Dialect pattern | `hapihub-rs/src/model/dialect.rs` |
| Transport layer | `hapihub-rs/src/transport/` |
| Embedded runtime | `hapihub-rs/src/embedded/runtime.rs` |
| IPC structs | `hapihub-rs/src/transport/ipc.rs` |
| Rust auth | `hapihub-rs/src/auth/` |
