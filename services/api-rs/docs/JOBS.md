# Background Jobs Documentation

## Overview

The API uses **pg-boss** for background job processing. Jobs are registered at startup and run on schedule or manually triggered.

**For pg-boss details**, see [pg-boss documentation](https://github.com/timgit/pg-boss/blob/master/docs/readme.md).

---

## Job Scheduler Interface

Location: `src/core/jobs.ts`

**Purpose**: Thin abstraction over pg-boss that uses the existing Drizzle database connection.

### Key Features

- **Shared Database Connection**: Reuses Drizzle's pg.Pool (no extra connections)
- **Provider-Agnostic**: Interface can be swapped without changing handler code
- **Automatic Retries**: Exponential backoff for failed jobs (configurable)
- **Cron Support**: Schedule jobs with cron syntax
- **Interval Support**: Run jobs at regular intervals
- **Manual Triggering**: Start jobs on-demand via API

---

## Job Types

### 1. Cron Jobs

Run on a schedule using cron syntax:

```typescript
scheduler.registerCron('job-name', '0 3 * * *', async (context) => {
  // Runs daily at 3 AM
});
```

**Cron patterns**:
- `'0 3 * * *'` - Daily at 3 AM
- `'*/15 * * * *'` - Every 15 minutes
- `'0 0 * * 0'` - Weekly on Sunday at midnight

See [crontab.guru](https://crontab.guru/) for pattern reference.

### 2. Interval Jobs

Run at regular intervals:

```typescript
scheduler.registerInterval('job-name', 60000, async (context) => {
  // Runs every 60 seconds
});
```

**Note**: Intervals convert to cron patterns internally (minimum 1 minute).

### 3. Delayed Jobs

Run once after a delay:

```typescript
scheduler.registerDelayed('job-name', 5000, async (context) => {
  // Handler logic
});

// Trigger with delay
await scheduler.trigger('job-name', { data: 'value' });
```

---

## Registration Pattern

### Module Structure

```
src/handlers/[module]/
├── jobs/
│   ├── index.ts          # Register all module jobs
│   └── [specific-job].ts # Optional: Complex job logic
├── repos/
└── handlers...
```

### Registration File Template

```typescript
// src/handlers/[module]/jobs/index.ts
import type { JobScheduler, JobContext } from '@/core/jobs';

export function register[Module]Jobs(scheduler: JobScheduler): void {
  // Cron job example
  scheduler.registerCron('module.task', '0 3 * * *', async (context: JobContext) => {
    const { db, logger, jobId } = context;
    logger.debug('Starting job', { jobId });
    
    try {
      // Lazy import repository
      const { ModuleRepository } = await import('../repos/module.repo');
      const repo = new ModuleRepository(db, logger);
      
      // Perform work
      const result = await repo.performTask();
      
      logger.info('Job completed', { jobId, result });
    } catch (error) {
      logger.error({ error, jobId }, 'Job failed');
      throw error; // pg-boss handles retry
    }
  });
  
  // Interval job example
  scheduler.registerInterval('module.cleanup', 300000, async (context) => {
    // Runs every 5 minutes
  });
}
```

### Registration in Main

```typescript
// src/index.ts
import { registerAuditJobs } from '@/handlers/audit/jobs';
import { registerEmailJobs } from '@/handlers/email/jobs';

// After job scheduler is created
registerAuditJobs(jobScheduler);
registerEmailJobs(jobScheduler);

await jobScheduler.start();
```

---

## Job Context

Every job handler receives a context object:

```typescript
interface JobContext {
  db: DatabaseInstance;     // Drizzle database instance
  logger: Logger;            // Pino logger with job metadata
  jobId: string;             // Unique job execution ID
  jobName: string;           // Registered job name
  data?: any;                // Data passed to job
}
```

**Usage**:
```typescript
async (context: JobContext) => {
  const { db, logger, jobId, jobName, data } = context;
  
  logger.debug('Processing job', { jobId, jobName });
  
  // Use db for database operations
  const result = await db.query.table.findMany();
  
  // Use logger with automatic context
  logger.info('Job completed', { result });
}
```

---

## Error Handling

### Automatic Retries

pg-boss automatically retries failed jobs with exponential backoff:

```typescript
// Default retry configuration (in jobs.ts)
{
  retryLimit: 2,          // Retry up to 2 times
  retryDelay: 5,          // 5 seconds initial delay
  retryBackoff: true,     // Exponential: 5s, 10s, 20s...
}
```

### Error Logging

```typescript
scheduler.registerCron('job-name', '0 3 * * *', async (context) => {
  const { logger, jobId } = context;
  
  try {
    await performWork();
    logger.info('Job succeeded', { jobId });
  } catch (error) {
    logger.error({ error, jobId }, 'Job failed - will retry');
    throw error; // Let pg-boss handle retry
  }
});
```

**Important**: Always throw errors to trigger pg-boss retry logic.

---

## Manual Job Triggering

Trigger jobs manually via API or code:

```typescript
// In a handler or service
const jobScheduler = ctx.get('jobScheduler');

// Trigger without data
const jobId = await jobScheduler.trigger('module.task');

// Trigger with data
const jobId = await jobScheduler.trigger('module.process', {
  entityId: '123',
  action: 'update'
});

logger.info('Job triggered', { jobId });
```

---

## Real-World Examples

### Example 1: Audit Log Retention (Cron)

```typescript
// src/handlers/audit/jobs/index.ts
scheduler.registerCron('audit.retention', '0 3 * * *', async (context) => {
  const { db, logger } = context;
  
  const { AuditRepository } = await import('../repos/audit.repo');
  const auditRepo = new AuditRepository(db, logger);
  
  // Archive logs older than 1 year
  const archived = await auditRepo.archiveOldLogs(365);
  
  // Purge logs older than 7 years (HIPAA)
  const purged = await auditRepo.purgeArchivedLogs(2555);
  
  logger.info('Audit retention complete', { archived, purged });
});
```

### Example 2: Email Queue Processing (Interval)

```typescript
// src/handlers/email/jobs/index.ts
scheduler.registerInterval('email.process-queue', 30000, async (context) => {
  const { db, logger } = context;
  
  const emailService = ctx.get('emailService');
  await emailService.processPendingEmails();
  
  logger.debug('Email queue processed');
});
```

### Example 3: Notification Cleanup (Interval)

```typescript
scheduler.registerInterval('notifs.cleanup', 3600000, async (context) => {
  const { db, logger } = context;
  
  const { NotificationRepository } = await import('../repos/notification.repo');
  const notifRepo = new NotificationRepository(db, logger);
  
  // Clean up notifications older than 90 days
  const cleaned = await notifRepo.cleanupExpiredNotifications(90);
  
  logger.info('Notification cleanup complete', { cleaned });
});
```

---

## Monitoring

### Health Check

```typescript
const health = await jobScheduler.getHealth();
// Returns: { healthy: boolean, queueSize?: number }
```

### Queue Size

```typescript
const size = await jobScheduler.getQueueSize('module.task');
// Returns number of pending jobs
```

### Logging

All job operations are logged with structured data:

```json
{
  "job": "module.task",
  "jobId": "abc-123",
  "level": "info",
  "msg": "Job completed successfully"
}
```

---

## Best Practices

1. **Lazy Import Repositories**: Import repositories inside job handlers to avoid circular dependencies
2. **Always Throw Errors**: Let pg-boss handle retries automatically
3. **Use Structured Logging**: Include `jobId` and relevant context in logs
4. **Keep Jobs Idempotent**: Jobs may be retried, ensure they can run multiple times safely
5. **Set Appropriate Intervals**: Minimum 1 minute for interval jobs
6. **Use Cron for Schedules**: Prefer cron syntax over intervals for specific times
7. **Monitor Job Health**: Check queue sizes and health status in production
8. **Reference pg-boss Docs**: For advanced features (priorities, deadlines, etc.)

---

## Testing

Jobs are automatically started in test environments. Test by triggering manually:

```typescript
// In test file
const jobScheduler = app.get('jobScheduler');

// Trigger job
const jobId = await jobScheduler.trigger('module.task', { test: true });

// Wait for completion (in tests)
await new Promise(resolve => setTimeout(resolve, 1000));

// Verify results in database
```

---

## Configuration

Job scheduler configuration in `src/core/jobs.ts`:

```typescript
{
  schema: 'pgboss',                        // Isolated pg-boss tables
  deleteAfterDays: 1,                      // Cleanup completed jobs
  archiveCompletedAfterSeconds: 300,       // Archive after 5 minutes
  retryLimit: 2,                           // Default retry count
  retryDelay: 5,                           // Initial retry delay (seconds)
  expireInMinutes: 5,                      // Job expiration
  maintenanceIntervalSeconds: 10,          // Maintenance frequency
}
```

---

## Troubleshooting

### Job Not Running

1. Check if job is registered: `jobScheduler.getQueueSize('job-name')`
2. Verify cron pattern: Use [crontab.guru](https://crontab.guru/)
3. Check logs for errors during registration
4. Ensure job scheduler is started: `await jobScheduler.start()`

### Jobs Stuck in Queue

1. Check worker is registered: Review startup logs
2. Verify database connection is active
3. Check for deadlocks: Review pg-boss schema logs
4. Restart job scheduler: `await jobScheduler.shutdown()` then `start()`

### High Retry Rate

1. Fix underlying error in job handler
2. Adjust retry configuration if needed
3. Add more detailed error logging
4. Consider implementing circuit breaker pattern

---

**For complete pg-boss features**, see [pg-boss documentation](https://github.com/timgit/pg-boss/blob/master/docs/readme.md).
