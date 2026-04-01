# Testing Guide

## Overview

This document describes testing patterns and best practices for the Monobase Rust API service.

## Test Types

### Unit Tests (Rust)

```bash
cargo test
```

Unit tests live alongside the source code using `#[cfg(test)]` modules. Current coverage:
- `auth::password` — bcrypt hash/verify compatibility
- `auth::session` — HMAC sign/verify, URL-encoded tokens
- `model::dialect` — PostgreSQL vs SQLite SQL generation

### E2E Tests (TypeScript/Bun)

```bash
# Start dependencies
bun run deps:up

# Start the Rust server (separate terminal)
cargo run

# Run E2E tests
./run-tests.sh
```

E2E tests live at `tests/e2e/` and use real HTTP requests — no internal Rust imports. They work as black-box tests against the running server.

## Test Isolation for Parallel Execution

### The Problem

Bun test runner executes tests in parallel by default. Tests that share state or assume they're the only ones accessing the database will fail unpredictably.

**Common symptoms:**
- Tests pass individually but fail when run together
- Flaky tests that sometimes pass, sometimes fail
- Assertion errors like `expect(data.length).toBe(1)` when you get 5 results

### The Solution: Complete Test Isolation

Each test must be completely isolated:

1. **Create its own authenticated clients** — Never share clients between tests
2. **Create its own test data** — via API calls, not direct DB access
3. **Use data-aware assertions** — Verify YOUR data exists, not that ALL data matches

## Data-Aware Assertions Pattern

### Wrong: Assumes ALL data matches your filter

```typescript
test('should filter by owner', async () => {
  const client = await createAuthenticatedClient();
  const person = await createTestPerson(client);

  const { data } = await client.get('/persons');

  // WRONG: Other parallel tests also create persons
  expect(data.length).toBe(1);
});
```

### Correct: Verify YOUR data exists

```typescript
test('should filter by owner', async () => {
  const client = await createAuthenticatedClient();
  const person = await createTestPerson(client);

  const { data } = await client.get('/persons');

  // CORRECT: Find OUR person in results
  expect(data.length).toBeGreaterThan(0);
  const ourPerson = data.find(p => p.id === person.id);
  expect(ourPerson).toBeDefined();
});
```

## Test Structure Pattern

### Template for Isolated Tests

```typescript
import { describe, test, expect } from "bun:test";
import { createAuthenticatedClient } from "../helpers/auth";
import { createTestPerson, createTestPatient } from "../helpers/fixtures";

describe("Patient Management", () => {
  test("create and retrieve patient", async () => {
    // 1. Create your own client (fresh user)
    const client = await createAuthenticatedClient();

    // 2. Create your own test data via API
    const person = await createTestPerson(client);
    const patient = await client.post("/patients", { personId: person.id });

    // 3. Verify via API
    const fetched = await client.get(`/patients/${patient.id}`);

    // 4. Data-aware assertions
    expect(fetched.id).toBe(patient.id);
  });

  test("another test with its own data", async () => {
    // Each test gets its own client and data
    const client = await createAuthenticatedClient();
    // ...
  });
});
```

## Common Pitfalls

### 1. Reusing Clients Between Tests

```typescript
// Wrong
let sharedClient: ApiClient;
beforeAll(async () => { sharedClient = await createAuthenticatedClient(); });

// Correct — create per test
test('test', async () => {
  const client = await createAuthenticatedClient();
});
```

### 2. Asserting on ALL Results

```typescript
// Wrong
const { data } = await client.get('/patients');
expect(data.length).toBe(1);

// Correct
const patient = await createTestPatient(client, personId);
const { data } = await client.get('/patients');
const ours = data.find(p => p.id === patient.id);
expect(ours).toBeDefined();
```

### 3. Expecting Empty Results

```typescript
// Wrong
const { data } = await client.get('/patients');
expect(data.length).toBe(0);

// Correct
const { data } = await client.get('/patients');
const leaked = data.find(p => p.id === someOtherId);
expect(leaked).toBeUndefined();
```

## E2E Test Modules

| Module | Path | Coverage |
|--------|------|----------|
| Health | `tests/e2e/health/` | /health, /livez, /readyz |
| Auth | `tests/e2e/auth/` | sign-up, sign-in, session, sign-out |
| Person | `tests/e2e/person/` | CRUD, auth, 404 |
| Patient | `tests/e2e/patient/` | CRUD, delete, duplicate constraint |
| Provider | `tests/e2e/provider/` | CRUD, list (any auth), delete |
| Booking | `tests/e2e/booking/` | Events, slots, CRUD |
| Billing | `tests/e2e/billing/` | Invoices, finalize, void, merchants, webhook |
| Comms | `tests/e2e/comms/` | Chat rooms, messages, ICE servers |
| EMR | `tests/e2e/emr/` | Consultations, finalize, draft guard |
| Email | `tests/e2e/email/` | Templates, preview (admin) |
| Storage | `tests/e2e/storage/` | Upload, complete, download, delete |
| Notifs | `tests/e2e/notifs/` | List, mark-read |
| Reviews | `tests/e2e/reviews/` | CRUD, NPS validation |
| Audit | `tests/e2e/audit/` | List (admin), 403 for non-admin |

## Testing Checklist

Before committing tests, verify:

- [ ] Each test creates its own authenticated clients
- [ ] No shared state between tests
- [ ] Assertions use `.find()` to locate specific data you created
- [ ] No assertions on `data.length` or `.forEach()` over all results
- [ ] Tests pass when run individually AND in parallel
- [ ] No `beforeAll` that creates shared mutable data
