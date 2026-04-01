# Contributing to Monobase Application Platform

Thank you for your interest in contributing to Monobase! This guide will help you get started with development and understand our workflows.

## Table of Contents

- [Development Setup](#development-setup)
- [Project Structure](#project-structure)
- [API-First Development](#api-first-development)
- [Coding Standards](#coding-standards)
- [Module Structure Patterns](#module-structure-patterns)
- [Database Workflow](#database-workflow)
- [Testing Requirements](#testing-requirements)
- [Git Workflow](#git-workflow)
- [Pull Request Process](#pull-request-process)
- [Code Review Guidelines](#code-review-guidelines)
- [Debugging Tips](#debugging-tips)
- [Frontend Development Patterns](#frontend-development-patterns)
- [Getting Help](#getting-help)

## Development Setup

### 1. Initial Setup

```bash
# Clone the repository
git clone <repository-url>
cd monobase

# Install dependencies
bun install

# Set up PostgreSQL database
createdb monobase
```

### 2. Environment Configuration

Each service/app requires its own `.env` file:

**API Service** (`services/api/.env`):
```bash
DATABASE_URL=postgresql://user:password@localhost:5432/monobase
PORT=7213
AUTH_SECRET=your-secret-key-here
STRIPE_SECRET_KEY=sk_test_...
AWS_ACCESS_KEY_ID=...
AWS_SECRET_ACCESS_KEY=...
```

**Account App** (`apps/account/.env`):
```bash
VITE_API_URL=http://localhost:7213
```

### 3. Database Initialization

```bash
cd services/api
bun run db:generate  # Generate initial schema
```

### 4. Verify Setup

```bash
# Start API service
cd services/api && bun dev

# In another terminal, start account app
cd apps/account && bun dev

# Verify API is running
curl http://localhost:7213/health
```

## Project Structure

### Workspace Organization

```
monobase/
├── apps/                      # Frontend applications
│   └── account/              # Vite + TanStack Router account portal
├── packages/                  # Shared packages
│   ├── typescript-config/    # Shared TypeScript configs
│   ├── ui/                   # Shared UI components
│   └── sdk/                  # Type-safe API client
├── services/                  # Backend services
│   └── api/                  # API service (Axum + SeaORM)
└── specs/                     # API specifications
    └── api/                  # TypeSpec definitions
```

### Code Organization Standards

**Frontend Apps**:
- `src/routes/` - File-based routing (TanStack/Next.js)
- `src/components/` - App-specific components
- `src/lib/` - Utility functions and API clients
- `src/hooks/` - Custom React hooks

**API Service** (`services/api/`):
- `src/handlers/` - Axum route handlers with embedded auth + validation
- `src/handlers/{module}/repo.rs` - SQL queries via SeaORM
- `src/service/` - External integrations (Stripe, S3, email, notifications)
- `src/transport/` - HTTP (Axum) and IPC (embedded) adapters
- `src/model/` - Dialect abstraction (PG + SQLite), base CRUD model
- `src/auth/` - Session validation, bcrypt passwords (Better-Auth compatible)

**Shared UI Package**:
- `src/components/` - Reusable shadcn/ui components
- `src/lib/` - Utility functions (cn, etc.)

## API-First Development

Monobase follows an **API-first workflow** using TypeSpec. This ensures frontend and backend stay in sync.

### Workflow Steps

#### 1. Define API in TypeSpec

Edit or create TypeSpec definitions in `specs/api/src/modules/`:

```typescript
// specs/api/src/modules/person.tsp
import "@typespec/http";
import "@typespec/openapi3";

namespace Persons {
  @route("/persons")
  interface PersonOperations {
    @get
    @summary("List all persons")
    listPersons(): Person[];

    @post
    @summary("Create a new person")
    createPerson(@body person: CreatePersonRequest): Person;
  }
}

model Person {
  id: string;
  firstName: string;
  lastName: string;
  email: string;
  created_at: utcDateTime;
}

model CreatePersonRequest {
  firstName: string;
  lastName: string;
  email: string;
}
```

#### 2. Generate OpenAPI and TypeScript Types

```bash
cd specs/api
bun run build
```

This generates:
- `dist/openapi/openapi.json` - OpenAPI 3.0 specification
- `dist/typescript-types/api.d.ts` - TypeScript type definitions

#### 3. Implement Backend Handler

Create or update the handler in `services/api/src/handlers/`:

```typescript
// services/api/src/handlers/person.ts
import { Hono } from 'hono';
import { z } from 'zod';
import type { Person } from '@monobase/api-spec';

const personRouter = new Hono();

personRouter.get('/', async (c) => {
  // Implementation
  const persons = await db.select().from(personsTable);
  return c.json(persons);
});

personRouter.post('/', async (c) => {
  // Validation with Zod
  const body = await c.req.json();
  // Implementation
  const person = await db.insert(personsTable).values(body);
  return c.json(person);
});

export default personRouter;
```

#### 4. Use Types in Frontend

Import generated types in your frontend apps:

```typescript
// apps/account/src/lib/api.ts
import type { Person, CreatePersonRequest } from '@monobase/api-spec';

export async function createPerson(data: CreatePersonRequest): Promise<Person> {
  const response = await fetch('/api/persons', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(data),
  });
  return response.json();
}
```

### Why API-First?

- **Type Safety**: Frontend and backend share exact types
- **Documentation**: OpenAPI spec is always up-to-date
- **Contract Testing**: Catch breaking changes early
- **Developer Experience**: Autocomplete and type checking
- **Consistency**: Single source of truth for API structure

## Code Generation - Do Not Edit

⚠️ **IMPORTANT**: The API service uses code generation. Many files are auto-generated and should **NEVER** be edited manually.

### Generated Files (Never Edit)

Running `cd services/api && bun run generate` creates these files automatically:

#### 1. OpenAPI Code (`src/generated/openapi/`)
- **`types.ts`** - TypeScript types (re-exported from `@monobase/api-spec`)
- **`validators.ts`** - Zod schemas for request/response validation
- **`routes.ts`** - Hono routes with automatic validation middleware
- **`registry.ts`** - Maps operation IDs to handler functions

**⚠️ DO NOT EDIT** - These are regenerated every time you run `bun run generate`

#### 2. Better-Auth Generated (`src/generated/better-auth/`)
- **`schema.ts`** - Better-Auth database schema
- **`openapi.json`** - Better-Auth OpenAPI specification

**⚠️ DO NOT EDIT** - Managed by Better-Auth CLI

#### 3. Database Migrations (`src/generated/migrations/`)
- SQL migration files generated by Drizzle Kit
- Generated via `bun run db:generate`

**⚠️ DO NOT EDIT** - Migrations should never be modified after creation

### Generated Once (Safe to Edit)

#### Handler Stubs (`src/handlers/{module}/{operationId}.ts`)
The generator creates handler stub files **only if they don't exist**:

```typescript
// src/handlers/patient/createPatient.ts
import { Context } from 'hono';

export async function createPatient(ctx: Context) {
  const body = ctx.req.valid('json');

  // TODO: Implement business logic
  throw new Error('Not implemented: createPatient');
}
```

**✅ SAFE TO EDIT** - These are implementation files you write
**Note**: Re-running `bun run generate` will NOT overwrite existing handlers

### What to Edit Manually

You should **only** edit these files:

1. **TypeSpec Definitions** (`specs/api/src/modules/*.tsp`)
   - API contracts, request/response models, endpoints

2. **Handler Implementations** (`src/handlers/{module}/*.ts`)
   - Business logic for each endpoint

3. **Database Schemas** (`src/db/schema/*.ts`)
   - Drizzle schema definitions (migrations are auto-generated from these)

4. **Repositories** (`src/handlers/{module}/repos/*.ts`)
   - Database access layer

5. **Services** (`src/core/*.ts`)
   - Core application services (auth, storage, email, etc.)

### Code Generation Workflow

```bash
# 1. Modify TypeSpec definitions
vim specs/api/src/modules/patient.tsp

# 2. Generate OpenAPI spec and types
cd specs/api && bun run build

# 3. Generate API code (routes, validators, handlers)
cd ../../services/api && bun run generate

# 4. Implement business logic in handlers
vim src/handlers/patient/createPatient.ts

# 5. Test your changes
bun test
```

### API Code Generation

The code generator (`services/api/generator-rs.ts`):

1. ✅ Reads OpenAPI spec from `specs/api/dist/openapi/openapi.json`
2. ✅ Generates `src/generated/enums.rs` — Rust enums with serde derives
3. ✅ Generates `src/generated/types.rs` — Request/response structs with serde
4. ✅ Generates `src/generated/routes.rs` — Route documentation and metadata
5. ✅ Updates `src/generated/mod.rs` — Module declarations

**⚠️ DO NOT EDIT** files in `services/api/src/generated/` — they are overwritten by the generator.

### Troubleshooting

**Problem**: Routes not updating after TypeSpec changes
**Solution**:
```bash
cd specs/api && bun run build  # Regenerate OpenAPI
cd ../../services/api && bun run generate  # Regenerate routes
```

**Problem**: Handler not found error
**Solution**: Make sure your handler file name matches the `operationId` in TypeSpec

**Problem**: Validation errors
**Solution**: Check that your TypeSpec schema matches what you're sending in requests

## Coding Standards

### TypeScript

- Use TypeScript 5.9+ features
- Enable strict mode
- Prefer `interface` over `type` for object shapes
- Use `const` assertions where appropriate
- Avoid `any` - use `unknown` or proper types

**Good**:
```typescript
interface PersonData {
  firstName: string;
  lastName: string;
  email: string;
}

function processPerson(data: PersonData): void {
  // Implementation
}
```

**Avoid**:
```typescript
function processPerson(data: any) {
  // No type safety
}
```

### React Components

- Use functional components with hooks
- Prefer named exports for components
- Use TypeScript for prop types
- Extract complex logic into custom hooks
- Keep components focused and small

```typescript
// Good component structure
interface PersonCardProps {
  person: Person;
  onSelect: (id: string) => void;
}

export function PersonCard({ person, onSelect }: PersonCardProps) {
  return (
    <div onClick={() => onSelect(person.id)}>
      <h3>{person.firstName} {person.lastName}</h3>
    </div>
  );
}
```

### API Handlers

- Use Zod for request validation
- Return consistent error responses
- Include audit logging for sensitive operations
- Handle errors gracefully
- Use middleware for cross-cutting concerns

```typescript
// Good handler pattern
personRouter.post('/', authMiddleware, async (c) => {
  try {
    // Validate request
    const schema = z.object({
      firstName: z.string(),
      lastName: z.string(),
      email: z.string().email(),
    });
    const body = schema.parse(await c.req.json());

    // Audit log
    logger.info({ user_id: c.get('user').id, action: 'create_person' });

    // Implementation
    const person = await createPerson(body);

    return c.json(person, 201);
  } catch (error) {
    if (error instanceof z.ZodError) {
      return c.json({ error: 'Validation failed', details: error.errors }, 400);
    }
    logger.error(error);
    return c.json({ error: 'Internal server error' }, 500);
  }
});
```

### Database Queries

- Use Drizzle ORM for type-safe queries
- Avoid raw SQL unless necessary
- Use transactions for multi-step operations
- Index frequently queried fields
- Include proper error handling

```typescript
// Good database pattern
import { db } from '../db';
import { persons } from '../db/schema';
import { eq } from 'drizzle-orm';

async function getPersonByEmail(email: string) {
  return await db
    .select()
    .from(persons)
    .where(eq(persons.email, email))
    .limit(1);
}
```

### Logging

The Monobase platform uses different logging approaches for backend and frontend applications.

#### Backend Logging (API Services)

**Technology**: [Pino](https://getpino.io/) for structured, high-performance logging

**Location**: `/services/api/src/core/logger.ts`

**Usage**:
```typescript
import { logger } from '@/core/logger'

// Log levels
logger.debug('Detailed diagnostic information')
logger.info('General informational message')
logger.warn('Warning condition')
logger.error('Error condition', { error })
logger.fatal('Critical system failure')

// Structured logging with context
logger.info('User action', {
  correlationId: req.correlationId,
  userId: user.id,
  action: 'profile_update',
  module: 'person'
})
```

**Best Practices**:

✅ **DO:**
- Include correlation IDs for request tracing
- Use appropriate log levels
- Add structured context (objects, not strings)
- Log errors with full error objects
- Sanitize PHI before logging (never log sensitive patient data)

❌ **DON'T:**
- Use console.* methods in backend code
- Log sensitive patient information (PHI)
- Log passwords, tokens, or credentials
- Use string concatenation for structured data

**Enterprise Compliance**:

The API handles Personally Identifiable Information (PII) and sensitive business data. Follow these rules:

1. **Never log PII directly** - Use sanitization functions
2. **Include correlation IDs** - Required for audit trails
3. **Log access events** - WHO accessed WHAT and WHEN
4. **Use appropriate levels** - Errors must be captured for compliance

```typescript
// ❌ WRONG - Logs PII
logger.info('Person record accessed', { person })

// ✅ CORRECT - Logs event without PII
logger.info('Person record accessed', {
  correlationId: req.correlationId,
  userId: user.id,
  personId: person.id,  // ID only, not full record
  action: 'read',
  resource: 'person_record'
})
```

#### Frontend Logging (Account App)

**Technology**: Native console methods with automatic production stripping

**Configuration**:

*Vite Apps (Account)*:
```typescript
// vite.config.ts
export default defineConfig({
  build: {
    esbuild: {
      // Automatically removes console.log in production
      // Keeps console.error and console.warn for debugging
      drop: ['console.log'],
    },
  },
})
```


**Usage**:
```typescript
// Development debugging - auto-removed in production
console.log('User navigated to dashboard')

// Error logging - KEPT in production for debugging
console.error('Failed to load user data', error)

// Warnings - KEPT in production
console.warn('API response time exceeded threshold')

// Info - KEPT in production
console.info('Feature flag enabled:', featureName)
```

**Best Practices**:

✅ **DO:**
- Use `console.log` freely during development
- Use `console.error` for errors (kept in production)
- Use `console.warn` for warnings (kept in production)
- Log user-friendly messages
- Avoid logging full API responses

❌ **DON'T:**
- Log sensitive user data (PHI, credentials, tokens)
- Log full API responses that might contain PHI
- Manually remove console statements from code
- Use console methods for production analytics

**Data Privacy Considerations**:

Even though frontend logs are stripped in production, follow these rules during development:

```typescript
// ❌ WRONG - Logs full person data
console.log('Profile updated:', personData)

// ✅ CORRECT - Logs confirmation without PII
console.log('Profile updated successfully')

// ❌ WRONG - Logs API response with PII
console.log('Video call joined:', response)

// ✅ CORRECT - Logs event without sensitive data
console.log('Video call joined successfully')
```

#### Production Monitoring

**Backend**:
- Pino logs to stdout/files
- Can integrate with log aggregation services (Datadog, LogRocket)
- Logs are structured JSON, easily searchable
- Set up alerts on error/fatal log levels

**Frontend**:
- Console logs stripped in production builds
- Consider error tracking services (Sentry, LogRocket)
- Use proper analytics tools for user monitoring

**Recommended Tools**:
- [Sentry](https://sentry.io/) - Error tracking and performance monitoring
- [LogRocket](https://logrocket.com/) - Session replay and error tracking
- [Datadog RUM](https://www.datadoghq.com/product/real-user-monitoring/) - Real user monitoring

#### Migration Path

**Backend (API)**:
- Replace all `console.*` with `logger.*` methods
- Add correlation IDs and structured context
- Ensure PHI sanitization

**Frontend (Account)**:
- Keep existing `console.*` statements
- Build configuration handles production stripping
- Review logs that might expose PHI

### Naming Conventions

#### General Rules

- **Files**: `kebab-case.ts` (e.g., `client-card.tsx`)
- **Components**: `PascalCase` (e.g., `ClientCard`)
- **Functions**: `camelCase` (e.g., `createClient`)
- **Constants**: `UPPER_SNAKE_CASE` (e.g., `MAX_RETRY_ATTEMPTS`)
- **Types/Interfaces**: `PascalCase` (e.g., `ClientData`)
- **Database Tables**: `snake_case` (e.g., `client_records`)

#### Component File Naming (Critical)

**⚠️ All component files MUST use kebab-case naming.**

This is a strict requirement across all apps (account, website). Inconsistent file naming causes confusion and maintenance issues.

**Correct** ✅:
```
src/components/booking/booking-flow-layout.tsx
src/components/services/service-card.tsx
src/components/documents/document-list.tsx
src/components/metrics/metrics-display.tsx
```

**Incorrect** ❌:
```
src/components/booking/BookingFlowLayout.tsx        // PascalCase - wrong
src/components/services/ServiceCard.tsx            // PascalCase - wrong
src/components/documents/DocumentList.tsx          // PascalCase - wrong
```

**Component Names vs. File Names**:
- **File name**: `service-card.tsx` (kebab-case)
- **Component name**: `ServiceCard` (PascalCase)
- **Export**: `export function ServiceCard() { ... }`
- **Import**: `import { ServiceCard } from './service-card'`

**Test Files** (see [Test File Organization](#test-file-organization-and-naming)):
- Unit tests: Match source file name with `.test.ts` suffix
  - `service-card.tsx` → `service-card.test.tsx`
- E2E tests: Use `.spec.ts` suffix in `tests/e2e/`
  - `booking.spec.ts`, `services.spec.ts`

**Rationale**:
- **Consistency**: Matches modern JavaScript/TypeScript conventions (React, Next.js, Remix)
- **Cross-platform**: Avoids case-sensitivity issues across operating systems
- **Git safety**: Prevents issues with case-insensitive filesystems (macOS, Windows)
- **Predictability**: Alphabetical sorting is consistent and easy to navigate
- **Industry standard**: Aligns with community best practices

### Date and Time Handling

**All date/time operations MUST follow the two-utility pattern for consistency across the codebase.**

#### The Two-Utility Pattern

1. **Formatting** → Use `@monobase/ui/lib/format-date`
2. **Manipulation/Logic** → Use `date-fns` directly

#### Formatting Dates

Always use the centralized `formatDate()` and `formatRelativeDate()` utilities:

```typescript
import { formatDate, formatRelativeDate } from '@monobase/ui/lib/format-date'

// Date formatting
formatDate(new Date(), { format: 'short' })      // "10/5/23"
formatDate(new Date(), { format: 'medium' })     // "Oct 5, 2023"
formatDate(new Date(), { format: 'long' })       // "October 5, 2023"
formatDate(new Date(), { format: 'full' })       // "Thursday, October 5, 2023"
formatDate(new Date(), { format: 'time' })       // "3:30 PM"
formatDate(new Date(), { format: 'datetime' })   // "Oct 5, 2023, 3:30 PM"
formatDate(new Date(), { format: 'iso' })        // "2023-10-05T15:30:00.000Z"

// Relative time formatting
formatRelativeDate(pastDate)                      // "3 hours ago"
formatRelativeDate(futureDate)                    // "in 2 days"
formatRelativeDate(date, { style: 'short' })      // "3h ago"
```

**React Components**:
```typescript
import { useFormatDate } from '@monobase/ui/hooks/use-format-date'

function MyComponent({ date }: { date: Date }) {
  const { formatDate, formatRelativeDate } = useFormatDate()

  return (
    <div>
      <p>{formatDate(date, { format: 'long' })}</p>
      <p>{formatRelativeDate(date)}</p>
    </div>
  )
}
```

#### Date Manipulation and Logic

Use `date-fns` directly for all date manipulation, comparisons, and calculations:

```typescript
import { addDays, subDays, differenceInMinutes, isAfter, isBefore, parseISO } from 'date-fns'

// Date arithmetic
const tomorrow = addDays(new Date(), 1)
const lastWeek = subDays(new Date(), 7)

// Time calculations
const minutesUntil = differenceInMinutes(futureDate, new Date())

// Date comparisons
if (isAfter(date1, date2)) { ... }
if (isBefore(date, new Date())) { ... }

// Parsing
const parsedDate = parseISO('2023-10-05T15:30:00.000Z')
```

#### Anti-Patterns (Do NOT Use)

**❌ Never use these patterns:**

```typescript
// ❌ Don't use .toISOString() directly
const isoString = new Date().toISOString()
// ✅ Use formatDate instead
const isoString = formatDate(new Date(), { format: 'iso' })

// ❌ Don't use locale methods
const dateStr = date.toLocaleDateString('en-US', { month: 'long' })
// ✅ Use formatDate instead
const dateStr = formatDate(date, { format: 'long' })

// ❌ Don't use manual date arithmetic
const tomorrow = new Date(Date.now() + 24 * 60 * 60 * 1000)
// ✅ Use date-fns instead
const tomorrow = addDays(new Date(), 1)

// ❌ Don't mutate dates
const date = new Date()
date.setDate(date.getDate() + 1)
// ✅ Use immutable date-fns functions
const tomorrow = addDays(date, 1)
```

#### Why This Pattern?

- **Consistency**: All formatting goes through one utility
- **Maintainability**: Easy to update date formatting globally
- **Type Safety**: TypeScript support for format options
- **Immutability**: date-fns never mutates dates
- **Clarity**: `addDays(date, 7)` is clearer than `Date.now() + 7*24*60*60*1000`

#### Common Use Cases

**ISO Date Strings (API Requests)**:
```typescript
import { formatDate } from '@monobase/ui/lib/format-date'

const payload = {
  scheduledAt: formatDate(appointmentDate, { format: 'iso' }),
  dateOfBirth: formatDate(dob, { format: 'iso' })
}
```

**Age Calculations**:
```typescript
import { differenceInYears } from 'date-fns'

const age = differenceInYears(new Date(), new Date(dateOfBirth))
```

**Time Windows (e.g., meeting join logic)**:
```typescript
import { differenceInMinutes, isToday } from 'date-fns'

const scheduledDate = parseISO(appointment.scheduledAt)
const minutesUntil = differenceInMinutes(scheduledDate, new Date())
const canJoin = isToday(scheduledDate) && minutesUntil >= -30 && minutesUntil <= 30
```

**Date Ranges**:
```typescript
import { startOfDay, endOfDay, isWithinInterval } from 'date-fns'

const isInRange = isWithinInterval(date, {
  start: startOfDay(new Date()),
  end: endOfDay(new Date())
})
```

### International Data Standards

When working with language, country, and timezone data, strict casing standards MUST be followed for system interoperability.

**Constants Location**: `packages/ui/src/constants/`

#### Language Codes (ISO 639-1)

**Standard**: Lowercase two-letter codes

**Examples**:
- ✅ `'en'` (English)
- ✅ `'es'` (Spanish)
- ✅ `'ja'` (Japanese)
- ❌ `'EN'`, `'Es'`, `'JA'`

**Why Lowercase Matters**:
- **BCP 47 Language Tags**: `'en-US'`, `'fr-CA'` (language part is lowercase)
- **HTTP Accept-Language Headers**: `'en-US,fr;q=0.9'`
- **HTML lang Attributes**: `<html lang="en">`
- **i18n Libraries**: Most expect lowercase ISO 639-1 codes

**Reference**: `packages/ui/src/constants/languages.ts`

#### Country Codes (ISO 3166-1 alpha-2)

**Standard**: Uppercase two-letter codes

**Examples**:
- ✅ `'US'` (United States)
- ✅ `'GB'` (United Kingdom)
- ✅ `'JP'` (Japan)
- ❌ `'us'`, `'Gb'`, `'jp'`

**Why Uppercase Matters**:
- **BCP 47 Region Subtags**: `'en-US'`, `'fr-CA'` (region part is uppercase)
- **Domain Country Codes**: `.US`, `.UK`, `.JP`
- **Banking Standards**: IBAN, SWIFT use uppercase country codes
- **Geographic APIs**: Most expect uppercase ISO 3166-1 alpha-2

**Reference**: `packages/ui/src/constants/countries.ts`

#### Timezone Identifiers (IANA)

**Standard**: Area/Location format (case-sensitive)

**Examples**:
- ✅ `'America/New_York'`
- ✅ `'Europe/London'`
- ✅ `'Asia/Tokyo'`
- ❌ `'america/new_york'`, `'EUROPE/LONDON'`, `'EST'`

**Why IANA Format Matters**:
- **JavaScript Intl API**: `Intl.DateTimeFormat('en-US', { timeZone: 'America/New_York' })`
- **Database Timezone Columns**: PostgreSQL, MySQL use IANA names
- **Backend Libraries**: dayjs, date-fns, luxon expect IANA format
- **Cross-platform Consistency**: Works across all environments

**Reference**: `packages/ui/src/constants/timezones.ts`

#### Validation in Code Reviews

When reviewing code that uses international data:

✅ **Check**:
```typescript
// Correct usage
const locale = 'en-US'  // lowercase language + uppercase country
const timezone = 'America/New_York'  // IANA format
const country = 'US'  // uppercase
const language = 'en'  // lowercase
```

❌ **Reject**:
```typescript
// Wrong casing
const locale = 'EN-us'  // ❌ wrong casing
const timezone = 'america/new_york'  // ❌ lowercase IANA
const country = 'us'  // ❌ lowercase country
const language = 'EN'  // ❌ uppercase language
```

#### Enforcement Mechanisms

**TypeSpec Validation** (`specs/api/src/common/models.tsp`):
- `CountryCode`: `@pattern("^[A-Z]{2}$")` enforces uppercase
- `LanguageCode`: `@pattern("^[a-z]{2}$")` enforces lowercase
- `TimezoneId`: `@pattern("^[A-Za-z_]+\/[A-Za-z_]+$")` enforces IANA format

**API Validation** (auto-generated from TypeSpec):
- Zod validators reject invalid formats at runtime
- Returns 400 error for casing violations

**Test Coverage** (`services/api/tests/e2e/person/person.test.ts`):
- "International Data Validation" suite tests casing enforcement
- Validates rejection of incorrect formats
- Ensures acceptance of correct formats

## Module Structure Patterns

Each business module follows a consistent structure for maintainability:

### Backend Module Structure

```
services/api/src/handlers/person/
├── mod.rs                  # All handlers (create, get, list, update) + request/response types
└── repo.rs                 # SQL queries via SeaORM (JsonValue + raw SQL)
```

The Rust service collocates handlers and types in `mod.rs` (no separate file per operation). The repo uses `sea_orm::JsonValue::find_by_statement` for flexible query results without codegen'd entity structs.

### Module Implementation Pattern

**Handler Example (createPerson.ts)**:
```typescript
import { Context } from 'hono';
import { PersonRepository } from './repos/person.repo';

export async function createPerson(ctx: Context) {
  const body = ctx.req.valid('json');
  const repo = ctx.get('personRepo') as PersonRepository;

  const person = await repo.create(body);

  return ctx.json(person, 201);
}
```

**Repository Example (repos/person.repo.ts)**:
```typescript
import { db } from '@/core/database';
import { persons } from '@/core/database.schema';
import { eq } from 'drizzle-orm';
import type { Logger } from '@/types/logger';

export class PersonRepository {
  constructor(
    private db: typeof db,
    private logger: Logger
  ) {}

  async create(data: CreatePersonData) {
    this.logger.info({ action: 'create_person' });

    const [person] = await this.db
      .insert(persons)
      .values(data)
      .returning();

    return person;
  }

  async findById(id: string) {
    const [person] = await this.db
      .select()
      .from(persons)
      .where(eq(persons.id, id))
      .limit(1);

    if (!person) {
      throw new Error('Person not found');
    }

    return person;
  }
}
```

**4. Handlers (handlers.ts)**:
```typescript
import { Context } from 'hono';
import { ClientService } from './service';
import { createClientSchema } from './validators';

const service = new ClientService();

export async function createClient(c: Context) {
  const body = createClientSchema.parse(await c.req.json());
  const client = await service.createClient(body);
  return c.json(client, 201);
}

export async function getClient(c: Context) {
  const id = c.req.param('id');
  const client = await service.getClient(id);
  return c.json(client);
}
```

### Consent Management Pattern

All modules with sensitive data include JSONB consent fields:

```typescript
// Database schema
export const clients = pgTable('clients', {
  id: uuid('id').defaultRandom().primaryKey(),
  person_id: uuid('person_id').references(() => persons.id),
  
  // Consent fields (JSONB)
  data_processing_consent: jsonb('data_processing_consent').$type<{
    granted: boolean;
    granted_at: string;
    ip_address: string;
    updated_at: string;
    updated_by: string;
  }>(),
  
  service_provider_access_consent: jsonb('service_provider_access_consent').$type<{
    granted: boolean;
    granted_at: string;
    ip_address: string;
    updated_at: string;
    updated_by: string;
  }>(),
});
```

## Database Workflow

### Schema Changes

1. **Modify Drizzle Schema**:
```typescript
// services/api/src/core/database.schema.ts
export const persons = pgTable('persons', {
  id: uuid('id').defaultRandom().primaryKey(),
  firstName: text('first_name').notNull(),
  lastName: text('last_name').notNull(),
  email: text('email').notNull().unique(),
  // Add new field
  phoneNumber: text('phone_number'),
  created_at: timestamp('created_at').defaultNow(),
});
```

2. **Generate Migration**:
```bash
cd services/api
bun run db:generate
```

3. **Review Generated SQL**:
```bash
cat src/generated/migrations/0001_add_phone_number_field.sql
```

4. **Apply Migration**:
Migrations are applied automatically on server start, or manually:
```bash
bun run db:migrate
```

### Database Inspection

Use Drizzle Studio for visual database exploration:

```bash
cd services/api
bun run db:studio
```

Opens a web interface at `http://localhost:4983`

### Best Practices

- **Migrations**: Never edit generated migrations - modify schema and regenerate
- **Indexes**: Add indexes for frequently queried columns
- **Foreign Keys**: Always define relationships
- **JSONB**: Use for flexible consent and configuration data
- **Timestamps**: Include `created_at` and `updated_at` on all tables

## Test File Organization and Naming

### Naming Conventions

**Two test types, two conventions:**
- **Unit Tests**: `.test.ts` (Bun test runner)
- **E2E Tests**: `.spec.ts` (Playwright)

#### Unit Tests - `.test.ts`

All unit tests use `.test.ts` suffix:

- ✅ `billing.test.ts` - Correct
- ❌ `billing.spec.ts` - Wrong

#### E2E Tests - `.spec.ts`

All Playwright E2E tests use `.spec.ts` suffix:

- ✅ `booking.spec.ts` - Correct
- ❌ `booking.test.ts` - Wrong

### Test Organization Patterns

#### Unit Tests - Colocated with Source

**Unit tests are placed next to the files they test:**

```
src/api/billing.ts          # Source file
src/api/billing.test.ts     # Unit test (colocated)

src/hooks/use-billing.ts    # Source file
src/hooks/use-billing.test.ts  # Unit test (colocated)

src/utils/formatters.ts     # Source file
src/utils/formatters.test.ts   # Unit test (colocated)
```

**Benefits of colocation:**
- Easier to find tests when editing source files
- Simpler to move/rename files (test moves with source)
- Clear what does and doesn't have tests
- Modern build tools automatically exclude test files

#### E2E Tests - Dedicated Directory (Playwright)

**E2E tests use Playwright's recommended structure:**

```
tests/e2e/
├── *.spec.ts                # E2E test files
├── pages/                   # Page Object Model classes
│   ├── login.page.ts
│   └── billing.page.ts
├── fixtures/                # Test data & custom fixtures
│   └── test-data.ts
└── helpers/                 # Utility functions
    └── auth-helpers.ts
```

**Playwright structure requirements:**
- **Page Objects** (`pages/`): Encapsulate page interactions
- **Fixtures** (`fixtures/`): Test data factories and custom fixtures
- **Helpers** (`helpers/`): Shared utilities (auth, data creation)

**Benefits of separation:**
- E2E tests require supporting files (page objects, fixtures)
- Clear distinction from unit tests
- Easier to run E2E tests independently
- Better organization for complex test scenarios

### Examples

**✅ Good:**
```
# Unit test - colocated
src/api/billing.ts
src/api/billing.test.ts

# E2E test - dedicated directory
tests/e2e/billing.spec.ts
tests/e2e/pages/billing.page.ts
tests/e2e/fixtures/billing-data.ts
tests/e2e/helpers/billing-helpers.ts
```

**❌ Bad:**
```
# Don't use .spec.ts for unit tests
src/api/billing.spec.ts

# Don't use .test.ts for E2E tests
tests/e2e/billing.test.ts

# Don't nest unit tests in __tests__
src/api/__tests__/billing.test.ts

# Don't colocate E2E tests with source
src/routes/billing.e2e.spec.ts
```

### Test Runner Configuration

Frontend apps use **two separate test runners**:

#### Frontend Apps (Using Both Bun + Playwright)

Frontend apps (account, website) use:
- **Bun test runner** for unit tests (colocated in `src/`)
- **Playwright** for E2E tests (in `tests/e2e/`)

**1. Update `package.json` scripts:**
```json
{
  "scripts": {
    "test": "bun test src/",           // Unit tests only
    "test:watch": "bun test src/ --watch",
    "test:e2e": "playwright test",     // E2E tests only
    "test:e2e:ui": "playwright test --ui"
  }
}
```

**2. Configure `playwright.config.ts`:**
```typescript
import { defineConfig } from '@playwright/test';

export default defineConfig({
  testDir: './tests/e2e',
  testMatch: '**/*.spec.ts',  // Only .spec.ts files
  // ... other config
});
```

**3. Running tests:**
```bash
# Unit tests (src/**/*.test.ts)
bun test
bun test --watch

# E2E tests (tests/e2e/**/*.spec.ts)
bun run test:e2e
bun run test:e2e:ui
```

**Why this works:**
- `bun test src/` explicitly targets only the src directory
- Playwright config (`testDir: './tests/e2e'`, `testMatch: '**/*.spec.ts'`) targets only E2E tests
- Using `.spec.ts` for E2E and `.test.ts` for unit tests provides visual separation
- Test runners stay separate, no conflicts

#### Backend Services (Using Only Bun)

Backend services (API) use only Bun test runner:

```bash
# All tests run with Bun
bun test                    # Runs all *.test.ts files
```

No special configuration needed since there's only one test runner.

### Playwright Patterns

#### Page Object Model

Encapsulate page interactions in classes:

```typescript
// tests/e2e/pages/login.page.ts
import { Page, Locator } from '@playwright/test';

export class LoginPage {
  readonly page: Page;
  readonly emailInput: Locator;
  readonly passwordInput: Locator;
  readonly loginButton: Locator;

  constructor(page: Page) {
    this.page = page;
    this.emailInput = page.getByLabel('Email');
    this.passwordInput = page.getByLabel('Password');
    this.loginButton = page.getByRole('button', { name: 'Login' });
  }

  async goto() {
    await this.page.goto('/auth/sign-in');
  }

  async signIn(email: string, password: string) {
    await this.emailInput.fill(email);
    await this.passwordInput.fill(password);
    await this.loginButton.click();
  }
}
```

#### Test Data Fixtures

Create reusable test data factories:

```typescript
// tests/e2e/fixtures/test-data.ts
import { faker } from '@faker-js/faker';

export function makeTestUser() {
  return {
    email: faker.internet.email(),
    password: 'TestPassword123!',
    name: faker.person.fullName(),
  };
}

export function makeTestServiceProvider(overrides = {}) {
  return {
    firstName: faker.person.firstName(),
    lastName: faker.person.lastName(),
    specialization: 'Business Consulting',
    ...overrides,
  };
}
```

#### Helper Functions

Create shared utilities for common operations:

```typescript
// tests/e2e/helpers/auth-helpers.ts
import { Page } from '@playwright/test';

export async function createTestUser(page: Page, userData: any) {
  const response = await page.request.post('/api/auth/signup', {
    data: userData,
  });
  return response.json();
}

export async function signInAsUser(page: Page, email: string, password: string) {
  await page.goto('/auth/sign-in');
  await page.fill('[name="email"]', email);
  await page.fill('[name="password"]', password);
  await page.click('button[type="submit"]');
  await page.waitForURL('/dashboard');
}
```

### Migration Notes

**Current state:**
- API service uses `.test.ts` ✅
- Website app E2E uses `.spec.ts` ✅
- Account app E2E uses `.spec.ts` ✅

**For new code:**
- Unit tests: Use `.test.ts`, colocate with source files
- E2E tests: Use `.spec.ts`, place in `tests/e2e/` directory
- Page objects: Place in `tests/e2e/pages/`
- Fixtures: Place in `tests/e2e/fixtures/`
- Helpers: Place in `tests/e2e/helpers/`

## Testing Requirements

### Unit Tests (API Service)

```bash
cd services/api
cargo test
```

Write tests for:
- Auth (bcrypt hashing, HMAC session verification)
- Dialect abstraction (PG vs SQLite SQL generation)
- Business logic in handlers and repos

**Example Test** (already in codebase):
```rust
// services/api/src/auth/password.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_and_verify() {
        let password = "test-password-123";
        let hash = hash_password(password).unwrap();
        assert!(verify_password(password, &hash).unwrap());
        assert!(!verify_password("wrong-password", &hash).unwrap());
    }
}
```

### E2E Tests (Frontend Apps)

```bash
cd apps/patient
bun run test:e2e
```

Write E2E tests for:
- Critical user flows
- Authentication
- Form submissions
- API integrations
- Error handling

**Example E2E Test (Playwright)**:
```typescript
// apps/account/e2e/booking.spec.ts
import { test, expect } from '@playwright/test';

test('client can book appointment', async ({ page }) => {
  await page.goto('/login');
  await page.fill('[name="email"]', 'client@example.com');
  await page.fill('[name="password"]', 'password');
  await page.click('button[type="submit"]');
  
  await page.goto('/service-providers');
  await page.click('text=John Smith Consulting');
  await page.click('text=Book Appointment');
  await page.click('[data-testid="time-slot-9am"]');
  await page.click('button:has-text("Confirm Booking")');
  
  await expect(page.locator('text=Appointment confirmed')).toBeVisible();
});
```

### Type Checking

Always run type checking before committing:

```bash
# Check API service
cd services/api && bun run typecheck

# Check account app
cd apps/account && bun run typecheck
```

### Test Coverage Requirements

- **Critical Paths**: 100% coverage for payment, booking, authentication
- **Business Logic**: 80%+ coverage for service layer
- **Handlers**: Test happy path and error cases
- **E2E**: Cover primary user workflows

## Git Workflow

### Branching Strategy

```bash
main                    # Production-ready code
├── develop            # Integration branch (if using)
├── feature/add-video consultations
├── fix/booking-timezone-bug
└── chore/update-dependencies
```

### Branch Naming Conventions

- `feature/` - New features (e.g., `feature/video-consultations`)
- `fix/` - Bug fixes (e.g., `fix/appointment-reminder-timing`)
- `chore/` - Maintenance tasks (e.g., `chore/upgrade-react-19`)
- `docs/` - Documentation updates (e.g., `docs/add-api-examples`)
- `refactor/` - Code refactoring (e.g., `refactor/extract-consent-logic`)

### Commit Message Format

Follow Conventional Commits specification:

```
<type>(<scope>): <subject>

<body>

<footer>
```

**Types**:
- `feat` - New feature
- `fix` - Bug fix
- `docs` - Documentation changes
- `style` - Code style changes (formatting, no logic change)
- `refactor` - Code refactoring
- `test` - Adding or updating tests
- `chore` - Maintenance tasks

**Examples**:
```bash
feat(booking): add service provider availability search filters

Add specialty, language, and service tier filters to provider search.
Includes backend API updates and frontend UI components.

Closes #123

---

fix(client): resolve appointment timezone display issue

Client appointments were showing in UTC instead of local timezone.
Updated date formatting logic to use client's timezone preference.

Fixes #456

---

chore(deps): upgrade TanStack Router to v1.80.0

Update TanStack Router across all apps. No breaking changes.
```

### Before Committing

Run pre-commit checklist:

```bash
# 1. Type checking
bun run typecheck

# 2. Run tests
bun test

# 3. Build check
bun run build

# 4. Lint (if configured)
bun run lint
```

## Pull Request Process

### 1. Create Pull Request

- **Title**: Clear, descriptive (follows commit convention)
- **Description**: What, why, and how
- **Linked Issues**: Reference related issues
- **Screenshots**: For UI changes
- **Testing**: Describe how to test changes

**PR Template**:
```markdown
## Description
Brief description of changes

## Type of Change
- [ ] Bug fix
- [ ] New feature
- [ ] Breaking change
- [ ] Documentation update

## Changes Made
- Change 1
- Change 2
- Change 3

## Testing
1. Step-by-step testing instructions
2. Expected behavior

## Screenshots (if applicable)
[Add screenshots]

## Checklist
- [ ] TypeScript types are correct
- [ ] Tests added/updated
- [ ] Documentation updated
- [ ] No console errors
- [ ] Tested locally
```

### 2. PR Review Requirements

- **Minimum**: 1 approval from code owner
- **Breaking Changes**: 2 approvals required
- **Database Changes**: Database owner approval
- **Security Changes**: Security review required

### 3. Automated Checks

PRs must pass:
- ✅ TypeScript type checking
- ✅ Unit tests
- ✅ E2E tests (if applicable)
- ✅ Build succeeds

### 4. Addressing Review Comments

```bash
# Make requested changes
git add .
git commit -m "fix: address PR review comments"
git push origin feature/your-feature
```

### 5. Merging

- **Squash and Merge**: For feature branches (keeps history clean)
- **Merge Commit**: For release branches
- **Delete Branch**: After merging

## Code Review Guidelines

### For Reviewers

#### What to Look For

**Functionality**:
- Does the code do what it's supposed to?
- Are edge cases handled?
- Is error handling appropriate?

**Code Quality**:
- Is the code readable and maintainable?
- Are names descriptive and consistent?
- Is there unnecessary complexity?
- Are there code smells?

**TypeScript**:
- Are types correct and specific (not `any`)?
- Are interfaces/types properly defined?
- Is type safety maintained?

**Security**:
- Are inputs validated?
- Is sensitive data protected?
- Are SQL injection risks prevented (using ORM)?
- Is authentication/authorization correct?

**Enterprise Compliance**:
- Are audit logs present for sensitive operations?
- Is consent validated before data access?
- Is PII handled appropriately?

**Testing**:
- Are tests included?
- Do tests cover edge cases?
- Are tests meaningful (not just for coverage)?

**Documentation**:
- Are complex sections commented?
- Is API documentation updated?
- Are breaking changes documented?

#### How to Give Feedback

**Be Constructive**:
```
❌ This is wrong
✅ Consider using a Map instead of an array for O(1) lookups
```

**Be Specific**:
```
❌ This needs work
✅ The error handling here should catch ZodError separately to return 400 instead of 500
```

**Explain Why**:
```
❌ Don't use any
✅ Using `any` here loses type safety. Consider `unknown` and use a type guard, or define a proper interface
```

**Acknowledge Good Work**:
```
✅ Nice refactoring! This is much more readable
✅ Great test coverage on edge cases
```

### For Authors

#### Responding to Reviews

- **Be Receptive**: Reviews improve code quality
- **Ask Questions**: If feedback is unclear, ask for clarification
- **Discuss Alternatives**: If you disagree, discuss respectfully
- **Resolve Conversations**: Mark as resolved after addressing

**Good Response**:
```
Thanks for catching that! I've updated the error handling to use a type guard.
Fixed in commit abc1234.
```

#### Self-Review Checklist

Before requesting review:
- [ ] Read your own diff
- [ ] Remove debugging code
- [ ] Update documentation
- [ ] Add tests
- [ ] Run type checking
- [ ] Test locally
- [ ] Check for console warnings

## Debugging Tips

### API Service Debugging

**Enable Debug Logging**:
```typescript
// services/api/src/utils/logger.ts
export const logger = pino({
  level: process.env.LOG_LEVEL || 'debug', // Set to 'debug'
});
```

**View Request/Response**:
```bash
# Add logging middleware
app.use('*', async (c, next) => {
  console.log('Request:', c.req.method, c.req.url);
  await next();
  console.log('Response:', c.res.status);
});
```

**Database Query Logging**:
```typescript
// Enable Drizzle query logging
import { drizzle } from 'drizzle-orm/postgres-js';

const db = drizzle(client, { logger: true });
```

**Common Issues**:

1. **Port Already in Use**:
```bash
# Find process using port 4000
lsof -i :4000
# Kill the process
kill -9 <PID>
```

2. **Database Connection Failed**:
```bash
# Verify PostgreSQL is running
pg_isready

# Check connection string
echo $DATABASE_URL
```

3. **Module Not Found**:
```bash
# Reinstall dependencies
rm -rf node_modules
bun install
```

### Frontend Debugging

**React DevTools**:
- Install React DevTools browser extension
- Inspect component tree and props
- Profile rendering performance

**Network Tab**:
- Monitor API requests
- Check request/response payloads
- Verify authentication headers

**Common Issues**:

1. **API Connection Failed**:
```typescript
// Check API URL configuration
console.log(import.meta.env.VITE_API_URL);
```

2. **TypeScript Errors After API Changes**:
```bash
# Regenerate API types
cd specs/api
bun run build

# Restart dev server
cd apps/account
bun dev
```

3. **Stale Cache**:
```bash
# Clear Vite cache
rm -rf node_modules/.vite
bun dev
```

### Database Debugging

**Drizzle Studio**:
```bash
cd services/api
bun run db:studio
# Opens http://localhost:4983
```

**Manual SQL**:
```bash
# Connect to database
psql $DATABASE_URL

# List tables
\dt

# Describe table
\d clients

# Query data
SELECT * FROM clients LIMIT 10;
```

**Migration Issues**:
```bash
# Reset database (CAUTION: deletes all data)
dropdb monobase
createdb monobase
cd services/api
bun run db:generate
```

### TypeSpec Debugging

**View Generated OpenAPI**:
```bash
cd specs/api
bun run build
cat dist/openapi/openapi.json | jq
```

**Validate TypeSpec Syntax**:
```bash
cd specs/api
bun run build  # Errors will show TypeSpec compilation issues
```

**Common TypeSpec Errors**:
- Missing imports: Add `import "@typespec/http";`
- Type not found: Check namespace imports
- Circular references: Restructure type dependencies

## Frontend Development Patterns

This section covers shared patterns for all frontend applications (account, website, etc.). These patterns apply to apps built with TanStack Router, React 19, and Bun runtime.

**Note**: For app-specific details (domain modules, routes, features), see each app's individual CONTRIBUTING.md file.

### ⚠️ Critical Frontend Rules

#### 1. Always Check OpenAPI Spec First

**Before implementing ANY API feature, check the OpenAPI specification.**

**Location**: `specs/api/dist/openapi/openapi.json`

**Essential Commands**:
```bash
# View schema
cat specs/api/dist/openapi/openapi.json | jq '.components.schemas.PersonUpdateRequest'

# Find nullable fields
cat specs/api/dist/openapi/openapi.json | jq '.components.schemas.PersonUpdateRequest.properties | to_entries[] | select(.value.nullable == true) | .key'
```

#### 2. shadcn/ui Components - CLI ONLY

**NEVER manually create or edit files in `src/components/ui/`**

**Always use the CLI**:
```bash
cd apps/[app-name]
bunx shadcn@latest add button form input textarea select
```

**Custom components** go in domain directories (NOT in `src/components/ui/`).

#### 3. Data Sanitization for Updates

**All UPDATE operations must use `sanitizeObject()` utility.** Check OpenAPI spec for nullable fields, then configure sanitization accordingly.

### Module Architecture (4-File Pattern)

Each domain module follows this structure:

**1. Schema** (`components/[module]/schema.ts`) - Zod schemas and TypeScript types

**2. Forms** (`components/[module]/*-form.tsx`) - Reusable form components
- Standard Props: `defaultValues`, `onSubmit`, `mode`, `showButtons`, `onCancel`
- Use React Hook Form + Zod resolver
- Add useEffect to update when defaultValues change

**3. API Functions** (`api/[module].ts`) - API client functions with sanitization
- Use `sanitizeObject()` for updates
- Check OpenAPI spec for nullable fields

**4. Query Hooks** (`hooks/use-[module].ts`) - TanStack Query hooks
- Query hooks for data fetching
- Mutation hooks for create/update/delete
- Invalidate queries on success
- Show toasts for feedback

### API Integration

**Base HTTP Client** (`src/api/client.ts`):
```typescript
apiGet<T>(url: string, params?: Record<string, any>): Promise<T>
apiPost<T>(url: string, data?: any): Promise<T>
apiPatch<T>(url: string, data?: any): Promise<T>
apiDelete<T>(url: string): Promise<T>
```

**Data Sanitization** (`src/utils/api.ts`):
- Fields in `nullable` array: empty string/null → send `null`
- Fields NOT in `nullable` array: empty string/null → omit from payload

**Query Keys Pattern** (`src/api/query.ts`):
```typescript
export const queryKeys = {
  person: () => ['person'] as const,
  personProfile: (id: string) => [...queryKeys.person(), 'profile', id] as const,
}
```

### Routing (TanStack Router)

**File Naming Conventions**:
- `filename.tsx` - Regular route
- `_filename.tsx` - Layout route
- `$param.tsx` - Dynamic parameter
- `index.tsx` - Index route
- `__root.tsx` - Root layout

**Route Guards** (`src/services/guards.ts`):
- `requireAuth()` - Authentication required
- `requireAuthWithProfile()` - Authentication + complete profile
- `requireAuthWithoutProfile()` - Onboarding only
- `requireGuest()` - Guest only

### Type Safety Rules

1. Never use `any` - use proper types
2. Import types from schemas
3. Separate API and frontend types

### Development Workflow

**Commands**:
```bash
cd apps/[app-name]
bun install          # Install dependencies
bun dev             # Start dev server
bun run build       # Build for production
bun run typecheck   # TypeScript checking
```

**Before Implementation**:
1. Check OpenAPI spec for schema/endpoints
2. Identify nullable fields
3. Verify request/response structures

**Common Patterns**:
```typescript
// Loading states
const { data, isLoading, error } = useQuery(...)
if (isLoading) return <Skeleton />
if (error) return <ErrorAlert />
if (!data) return <EmptyState />

// Path aliases - always use @/
import { Button } from '@/components/ui/button'
```

**For complete details and code examples**, see individual app documentation:
- Account App: `apps/account/CONTRIBUTING.md`

### Hook Architecture Patterns

#### Domain-Based Hook Organization

**When to Use**: This pattern applies specifically to **domain/business logic hooks** that interact with APIs and manage domain-specific state.

**Pattern**: `use-[domain].ts` exports multiple related domain functions

Organize domain-related hooks by **business context** rather than individual operations. This groups related functionality together and provides a clean API.

**Domain Hooks** (Group Together) ✅:
```typescript
// File: src/hooks/use-storage.ts
export function useFileUpload() { ... }
export function useFileDownload() { ... }
export function useFileDelete() { ... }

// File: src/hooks/use-billing.ts
export function usePay() { ... }
export function useRefund() { ... }
export function useInvoice() { ... }

// File: src/hooks/use-client.ts
export function useClientProfile() { ... }
export function useUpdateClient() { ... }
export function useClientHistory() { ... }
```

**Utility Hooks** (Keep Individual) ✅:
```typescript
// File: src/hooks/use-debounce.ts
export function useDebounce() { ... }

// File: src/hooks/use-media-query.ts
export function useMediaQuery() { ... }

// File: src/hooks/use-local-storage.ts
export function useLocalStorage() { ... }

// File: src/hooks/use-click-outside.ts
export function useClickOutside() { ... }
```

**Key Distinction**:
- **Domain hooks** (API/business logic) → Group by business domain
- **Utility hooks** (UI helpers, general utilities) → Keep as individual files
- Don't force hooks into domains where it doesn't make sense

**Benefits**:
- Related domain functionality grouped together
- Single import for all domain operations
- Clear file organization by business context
- Encapsulates domain API integration

**Usage**:
```typescript
// Domain hooks - single import for related operations
import { useFileUpload, useFileDownload } from '@/hooks/use-storage'

// Utility hooks - individual imports
import { useDebounce } from '@/hooks/use-debounce'
import { useMediaQuery } from '@/hooks/use-media-query'

function MyComponent() {
  const { upload } = useFileUpload()
  const { download } = useFileDownload()
  const debouncedValue = useDebounce(value, 500)
  const isMobile = useMediaQuery('(max-width: 768px)')
  // ...
}
```

#### API-Agnostic Routes Pattern

**Core Principle**: Routes should NEVER import API clients directly. Routes are UI layer, hooks are data layer.

**Architecture**:
```
┌─────────────┐
│   Routes    │  UI Layer (components, pages)
│  (UI Only)  │
└─────┬───────┘
      │ uses hooks
      ▼
┌─────────────┐
│    Hooks    │  Data Layer (queries, mutations)
│ (Data Logic)│
└─────┬───────┘
      │ uses API
      ▼
┌─────────────┐
│     API     │  HTTP Layer (fetch, axios)
│ (HTTP Client)│
└─────────────┘
```

**Rules**:
1. Routes only import from `@/hooks/*`
2. Hooks encapsulate API integration internally
3. API clients (`@/api/*`) only imported by hooks
4. No API imports in route components

**Good Pattern** ✅:
```typescript
// src/routes/_dashboard/settings/account.tsx
import { useFileUpload } from '@/hooks/use-storage'

function AccountSettingsPage() {
  const { upload } = useFileUpload()  // Clean API, no implementation details
  // ...
}
```

**Bad Pattern** ❌:
```typescript
// src/routes/_dashboard/settings/account.tsx
import { useFileUpload } from '@/hooks/use-storage'
import * as storageApi from '@/api/storage'  // ❌ Route shouldn't know about API

function AccountSettingsPage() {
  // ❌ Route exposes API implementation details
  const { upload } = useFileUpload({
    apiHandlers: {
      requestFileUpload: storageApi.requestFileUpload,
      uploadToPresignedUrl: storageApi.uploadToPresignedUrl,
      // ...
    }
  })
}
```

**Hook Implementation** (Internal API Integration):
```typescript
// src/hooks/use-storage.ts
import * as storageApi from '@/api/storage'  // ✅ Hook handles API

export function useFileUpload(options?: { maxFileSize?: number }) {
  // API integration is internal to the hook
  const upload = async (file: File) => {
    const uploadResponse = await storageApi.requestFileUpload({ ... })
    await storageApi.uploadToPresignedUrl(uploadResponse.uploadUrl, file)
    // ...
  }

  return { upload, isUploading, progress, error }
}
```

**Why This Matters**:
- Routes stay focused on UI concerns
- API implementation can change without touching routes
- Easier testing (mock hooks, not API calls)
- Clear separation of concerns
- Better maintainability

---

## Getting Help

### Resources

- **CLAUDE.md**: Comprehensive project guide for AI assistants
- **README.md**: Project overview and quick start
- **Module Docs**: Individual module documentation (in progress)
- **TypeSpec Docs**: https://typespec.io/docs
- **Drizzle ORM Docs**: https://orm.drizzle.team/docs/overview
- **Hono Docs**: https://hono.dev/docs
- **TanStack Docs**: https://tanstack.com/router/latest

### Communication Channels

- **Issues**: Report bugs and request features
- **Discussions**: Ask questions and share ideas
- **Pull Requests**: Code review and collaboration

### Questions?

If you're stuck:
1. Check existing documentation (CLAUDE.md, README.md)
2. Search closed issues for similar problems
3. Ask in discussions
4. Create a new issue with:
   - Clear description of the problem
   - Steps to reproduce
   - Expected vs actual behavior
   - Environment details (OS, Bun version, etc.)

## Enterprise Development Best Practices

### Enterprise Compliance

**Audit Logging**:
```typescript
// Always log sensitive data access
logger.info({
  user_id: c.get('user').id,
  action: 'view_client_records',
  client_id: clientId,
  timestamp: new Date().toISOString(),
});
```

**Consent Validation**:
```typescript
// Check consent before accessing data
if (!client.data_processing_consent?.granted) {
  return c.json({ error: 'Consent not granted' }, 403);
}
```

**Data Encryption**:
- Use TLS 1.3 for data in transit
- Encrypt sensitive fields at rest (planned)
- Never log sensitive PII (SSN, payment information, credentials)

### Person-Centric Design

Remember: **Person is the PII safeguard**
- Client and ServiceProvider extend Person
- Users can have multiple roles
- Never duplicate PII across tables
- Reference `person_id` for relationships

```typescript
// Good: Reference person_id
const client = await db
  .select()
  .from(clients)
  .innerJoin(persons, eq(clients.person_id, persons.id))
  .where(eq(clients.id, clientId));

// Bad: Duplicating person data in client table
```

---

## Thank You!

Thank you for contributing to Monobase Application Platform. Your work helps improve healthcare access and user management.

For questions or clarification, don't hesitate to reach out through issues or discussions.

Happy coding! 🚀
