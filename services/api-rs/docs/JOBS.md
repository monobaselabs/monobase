# Background Jobs Documentation

## Overview

The API uses a **custom pg-boss table poller** for background job processing. The scheduler polls the `pgboss.job` table directly using SeaORM, with no external job-runner process required. Jobs are registered at startup and dispatched to async Tokio tasks.

**Implementation**: `src/service/jobs.rs`

---

## Job Scheduler

Location: `src/service/jobs.rs`

**Purpose**: Poll the `pgboss.job` PostgreSQL table for pending jobs and dispatch them to registered async handlers.

### Key Features

- **Shared Database Connection**: Reuses the SeaORM `DatabaseConnection` — no extra connections
- **Tokio-native**: Handlers are `tokio::task::JoinHandle<()>`-returning closures
- **SELECT … FOR UPDATE SKIP LOCKED**: Prevents double-processing under concurrent instances
- **Graceful shutdown**: `stop()` sets an atomic flag; the poll loop exits cleanly

### Core Types

```rust
type JobHandler = Arc<dyn Fn(serde_json::Value) -> tokio::task::JoinHandle<()> + Send + Sync>;

pub struct JobScheduler {
    db: DatabaseConnection,
    handlers: Arc<Mutex<HashMap<String, JobHandler>>>,
    running: Arc<Mutex<bool>>,
}
```

---

## Job Registration

### Register a Handler

```rust
pub async fn register<F>(&self, name: &str, handler: F)
where
    F: Fn(serde_json::Value) -> tokio::task::JoinHandle<()> + Send + Sync + 'static,
```

**Example**:

```rust
scheduler.register("audit.retention", |data| {
    tokio::spawn(async move {
        tracing::info!(?data, "Running audit retention job");
        // perform work...
    })
}).await;
```

### Registration in `main.rs`

```rust
// After building AppContext
let scheduler = JobScheduler::new(ctx.db.clone());

scheduler.register("audit.retention", |data| {
    tokio::spawn(async move { /* ... */ })
}).await;

scheduler.register("email.process-queue", |data| {
    tokio::spawn(async move { /* ... */ })
}).await;

scheduler.start().await;
```

---

## Job Triggering

### Trigger a Job Manually

```rust
pub async fn trigger(&self, name: &str, data: serde_json::Value) -> Result<String, String>
```

Inserts a row into `pgboss.job` with `state = 'created'` and returns the generated UUID job ID:

```rust
// Trigger without data
let job_id = scheduler.trigger("audit.retention", serde_json::json!({})).await?;

// Trigger with data
let job_id = scheduler.trigger("billing.retry-payment", serde_json::json!({
    "invoiceId": "123",
    "attempt": 2
})).await?;

tracing::info!(job_id, "Job triggered");
```

---

## Poll Loop

The scheduler spawns a single Tokio task that polls every 5 seconds when no jobs are available:

```rust
pub async fn start(&self) {
    // Uses SELECT … FOR UPDATE SKIP LOCKED to claim a job atomically
    let sql = r#"
        UPDATE pgboss.job
        SET state = 'active', started_on = NOW()
        WHERE id = (
            SELECT id FROM pgboss.job
            WHERE state = 'created'
            ORDER BY created_on ASC
            LIMIT 1
            FOR UPDATE SKIP LOCKED
        )
        RETURNING id, name, data
    "#;
    // ...
}
```

**Job lifecycle states** (in `pgboss.job.state`):

| State       | Meaning                              |
|-------------|--------------------------------------|
| `created`   | Queued, awaiting pickup              |
| `active`    | Claimed and being executed           |
| `completed` | Handler returned successfully        |
| `failed`    | Handler panicked or returned `Err`   |

After the handler's `JoinHandle` resolves, the poller updates the row to `completed` or `failed`.

---

## Error Handling

If the `pgboss` schema does not exist (e.g. before migrations run), the poll errors are logged at `TRACE` level and the loop backs off for 10 seconds:

```rust
Err(e) => {
    // pg-boss tables might not exist yet — that's ok
    tracing::trace!(error = %e, "Job poll error (pgboss tables may not exist)");
    tokio::time::sleep(std::time::Duration::from_secs(10)).await;
}
```

If no registered handler exists for a job name, the job is immediately marked `failed`:

```rust
tracing::warn!(job_name, "No handler registered for job");
// UPDATE pgboss.job SET state = 'failed' ...
```

---

## Health Check

```rust
pub async fn health(&self) -> Result<serde_json::Value, String>
```

Returns a JSON summary of job counts by state:

```json
{
  "pending":   3,
  "active":    1,
  "completed": 847,
  "failed":    2
}
```

---

## Stopping the Scheduler

```rust
pub async fn stop(&self)
```

Sets the internal `running` flag to `false`. The poll loop checks this flag on each iteration and exits cleanly.

---

## Module Structure

Jobs are defined alongside the handlers they belong to:

```
src/handlers/[module]/
├── mod.rs          # Router + handlers
├── repo.rs         # SeaORM queries
└── jobs.rs         # Job registration helpers (optional)
```

Registration is called from `main.rs` or a central `register_jobs` function after the `AppContext` is built.

---

## Real-World Examples

### Audit Log Retention (Cron-style via pg-boss)

```rust
// Triggered once daily by an external cron or pg-boss schedule
scheduler.register("audit.retention", move |_data| {
    let db = db.clone();
    tokio::spawn(async move {
        tracing::info!("Starting audit retention job");

        // Archive logs older than 1 year
        let archived = audit_repo::archive_old_logs(&db, 365).await;

        // Purge logs older than 7 years (HIPAA)
        let purged = audit_repo::purge_archived_logs(&db, 2555).await;

        tracing::info!(?archived, ?purged, "Audit retention complete");
    })
}).await;
```

### Email Queue Processing (Interval)

```rust
scheduler.register("email.process-queue", move |_data| {
    let email_service = email_service.clone();
    tokio::spawn(async move {
        if let Err(e) = email_service.process_pending().await {
            tracing::error!(error = %e, "Email queue processing failed");
        }
    })
}).await;
```

### Notification Cleanup

```rust
scheduler.register("notifs.cleanup", move |_data| {
    let db = db.clone();
    tokio::spawn(async move {
        // Clean up notifications older than 90 days
        match notif_repo::cleanup_expired(&db, 90).await {
            Ok(n) => tracing::info!(cleaned = n, "Notification cleanup complete"),
            Err(e) => tracing::error!(error = %e, "Notification cleanup failed"),
        }
    })
}).await;
```

---

## Monitoring

### Health Endpoint

The `/readyz` endpoint includes job scheduler health via `JobScheduler::health()`.

### Logging

All job operations emit structured `tracing` events:

```
INFO Job triggered    job_id="abc-123" job_name="audit.retention"
INFO Processing job   job_id="abc-123" job_name="audit.retention"
INFO Job completed    job_id="abc-123"
ERROR Job failed      job_id="abc-123" error="..."
```

---

## Best Practices

1. **Keep handlers idempotent**: Jobs may be re-triggered; ensure they can run multiple times safely
2. **Avoid long-running blocking work**: Use `tokio::task::spawn_blocking` for CPU-heavy or synchronous code
3. **Log with job context**: Always include the job ID in structured log fields
4. **Handle errors explicitly**: Return `Err` or panic only when the job truly cannot proceed; the scheduler marks it `failed`
5. **Use `SKIP LOCKED`**: The poller already does this — safe to run multiple API instances
6. **Check pg-boss table existence**: Migrations must create the `pgboss` schema before jobs can run

---

## Commands

```bash
# Run the API (starts job scheduler automatically)
cargo run

# Run with debug logging to see job poll events
RUST_LOG=debug cargo run

# Run tests (job scheduler is not started in unit tests)
cargo test

# Run a specific test
cargo test jobs
```

---

## Troubleshooting

### Jobs Not Running

1. Verify the handler name matches the `name` column in `pgboss.job`
2. Check startup logs for `"Job scheduler started"` — confirms `start()` was called
3. Verify the `pgboss` schema exists: `SELECT * FROM pgboss.job LIMIT 1;`
4. Check for `WARN No handler registered for job` log entries

### Jobs Stuck in `active` State

This means the server crashed while a job was running. Reset manually:

```sql
UPDATE pgboss.job SET state = 'created', started_on = NULL WHERE state = 'active';
```

### High Failure Rate

1. Check `tracing` logs for `ERROR Job failed` entries
2. Review handler logic for panics or unhandled `Err` values
3. Confirm the database connection is stable
