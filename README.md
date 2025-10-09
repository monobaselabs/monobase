# Monobase Application Platform

A full-stack monorepo platform providing video sessions, messaging, and user management. Built with Bun runtime for 3x faster performance than Node.js.

## Overview

Monobase is a modern application platform designed to streamline user management and business workflows. The platform provides:

- **Account App** - Self-service account management and video sessions
- **API Service** - Backend with core business modules

## Key Features

- **Video Sessions** - Real-time video calls and secure messaging (WebRTC)
- **User Management** - Comprehensive user profiles and role management
- **Enterprise Compliance** - Audit trails, consent management, and secure data handling
- **Real-time Notifications** - Multi-channel delivery (email, push via OneSignal)
- **File Storage** - Secure file upload and download (S3/MinIO)

## Monorepo Structure

```
monobase/
├── apps/                      # Frontend applications
│   └── account/              # Account app (Vite + TanStack Router)
├── packages/                  # Shared libraries
│   ├── typescript-config/    # Shared TypeScript configurations
│   └── ui/                   # Shared UI components
├── services/                  # Backend services
│   └── api/                  # Main API service (Hono + Bun)
├── specs/                     # API specifications
│   └── api/                  # TypeSpec source definitions
├── CLAUDE.md                 # AI assistant project guide
└── package.json              # Monorepo workspace configuration
```

## Prerequisites

- **Bun** >= 1.2.21 ([installation guide](https://bun.sh))
- **PostgreSQL** >= 14
- **Node.js** >= 18 (for some tooling compatibility)
- **Git** for version control

### Optional Services
- **AWS S3** or **MinIO** for file storage
- **SMTP** server or **Postmark** for email delivery
- **OneSignal** for push notifications

## Quick Start

### 1. Clone and Install

```bash
git clone <repository-url>
cd monobase
bun install
```

### 2. Database Setup

```bash
# Create PostgreSQL database
createdb monobase

# Generate database schema
cd services/api
bun run db:generate
```

### 3. Configure Environment

Create `.env` files in each service/app directory (see individual READMEs for required variables):

```bash
# services/api/.env
DATABASE_URL=postgresql://user:password@localhost:5432/monobase
PORT=7213
```

### 4. Start Development Servers

```bash
# Terminal 1 - API Service
cd services/api
bun dev

# Terminal 2 - Account App
cd apps/account
bun dev
```

## Development Workflow

### API-First Development

1. **Define API** - Create/modify TypeSpec definitions in `specs/api/src/modules/`
2. **Generate** - Run `cd specs/api && bun run build:all`
3. **Implement** - Build Hono handlers in `services/api/src/handlers/`
4. **Test** - Write tests and run `cd services/api && bun test`
5. **Integrate** - Use generated TypeScript types in frontend apps

### Working with the Monorepo

```bash
# Install dependencies across all workspaces
bun install

# Build all packages
bun run --filter '*' build

# Clean build artifacts
bun run clean

# Run specific workspace command
cd apps/account && bun dev
```

## Available Commands

### Root Level

```bash
bun install                    # Install all workspace dependencies
bun run --filter '*' build    # Build all packages
bun run clean                  # Clean build artifacts
```

### API Service (`services/api/`)

```bash
bun dev                        # Start development server (port 7213)
bun run build                  # Build production bundle
bun run generate               # Generate routes, validators, handlers from OpenAPI
bun test                       # Run test suite
bun run typecheck              # TypeScript type checking
bun run db:generate            # Generate Drizzle migrations
bun run db:studio              # Open Drizzle Studio
```

**⚠️ Code Generation**: The API service auto-generates routes, validators, and handler stubs from TypeSpec. See [CONTRIBUTING.md#code-generation](./CONTRIBUTING.md#code-generation---do-not-edit) for what files to never edit manually.

### API Specifications (`specs/api/`)

```bash
bun run build                  # Generate OpenAPI specs
bun run build:types            # Generate TypeScript types
bun run build:all              # Generate both OpenAPI and types
```

### Account App (`apps/account/`)

```bash
bun dev                        # Start dev server (port 3001)
bun run build                  # Build production bundle
bun run typecheck              # TypeScript type checking
bun run test:e2e               # Run Playwright E2E tests
```

## Applications

### Account App

**Technology**: Vite + TanStack Router + React 19

User-facing application for:
- Account management and profile
- Video sessions and messaging
- File access and management

**Development**: `cd apps/account && bun dev`
**Port**: 3001

## API Service

### Business Modules

The API service is organized into domain-specific modules:

1. **Person** - User profile management and PII safeguard
2. **Audit** - Compliance logging and activity tracking
3. **Comms** - Video/chat sessions (WebRTC) and messaging
4. **Notifs** - Multi-channel notifications (email, push via OneSignal)
5. **Storage** - File upload/download (S3/MinIO)
6. **Email** - Transactional email delivery

**Authentication** is handled by Better-Auth (integrated, not a separate module).

### Key Architectural Patterns

**Person-Centric Design**: The Person module serves as the central PII safeguard for user data.

**Consent Management**: Consent is managed via JSONB fields on the Person model:
- marketing_consent: Marketing communications
- data_sharing_consent: Data sharing preferences
- sms_consent: SMS notifications
- email_consent: Email communications

### API Documentation

- **OpenAPI Spec**: `specs/api/dist/openapi/openapi.json`
- **TypeScript Types**: Generated to `@monobase/api-spec` package
- **Interactive Docs**: Scalar UI available at `/docs` endpoint

## Technology Stack

### Runtime & Build
- **Bun** 1.2.21+ - Fast JavaScript runtime and package manager
- **TypeScript** 5.9.2 - Type-safe development
- **ESM** - Modern module system

### Frontend
- **React** 19 - UI library
- **TanStack Router** - Type-safe routing
- **Radix UI** - Accessible component primitives
- **Tailwind CSS** - Utility-first styling
- **shadcn/ui** - Component library
- **Framer Motion** - Animations
- **React Hook Form** + **Zod** - Form validation

### Backend
- **Hono** - Fast web framework
- **Drizzle ORM** - Type-safe database queries
- **PostgreSQL** - Primary database
- **Better-Auth** - Authentication (no external service)
- **Pino** - Structured JSON logging
- **Zod** - Runtime validation

### API & Types
- **TypeSpec** - API-first specification language
- **OpenAPI** - REST API documentation
- **Type Generation** - Automatic TypeScript types from specs

### Infrastructure
- **AWS S3** / **MinIO** - Object storage
- **Postmark** / **SMTP** - Email delivery
- **OneSignal** - Push notifications

## Testing

### Unit & Integration Tests
```bash
cd services/api
bun test
```

### End-to-End Tests
```bash
# Account app E2E tests
cd apps/account
bun run test:e2e
```

### Type Checking
```bash
# Check all TypeScript types
cd services/api && bun run typecheck
cd apps/account && bun run typecheck
```

## Documentation

- **CLAUDE.md** - Comprehensive project guide for AI assistants and developers
- **CONTRIBUTING.md** - Developer contribution guidelines

## Enterprise Compliance

- **Audit Trails** - All data access includes comprehensive audit logging
- **Consent** - Granular consent management for all data operations
- **Security** - TLS 1.3, field-level encryption, role-based access
- **Audit** - Structured logging with correlation IDs
- **Data Integrity** - ACID-compliant PostgreSQL transactions

## Performance

- **3x Faster Startup** - Bun vs Node.js
- **Native TypeScript** - No transpilation overhead
- **Connection Pooling** - Optimized database queries
- **JSONB Indexing** - Fast consent and config queries

## License

[Add your license here - e.g., MIT, Apache 2.0, Proprietary]

---

For detailed development guidelines, see [CONTRIBUTING.md](./CONTRIBUTING.md).  
For AI assistant integration, see [CLAUDE.md](./CLAUDE.md).
