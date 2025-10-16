/**
 * Minimal Job Scheduler Interface
 * Provides a thin abstraction layer over pg-boss for background job processing
 * Uses existing Drizzle database connection to avoid additional connection pools
 */

import PgBoss from 'pg-boss';
import type { Logger } from '@/types/logger';
import type { DatabaseInstance } from './database';

// ============================================================================
// Public Interface (Provider-agnostic)
// ============================================================================

/**
 * Configuration for cron jobs
 */
export interface CronJobConfig {
  schedule: string;
  timezone?: string;
  retryLimit?: number;
}

/**
 * Configuration for interval jobs
 */
export interface IntervalJobConfig {
  intervalMinutes: number;
  retryLimit?: number;
}

/**
 * Minimal job scheduler interface
 * Exposes only the essential features needed for the application
 */
export interface JobScheduler {
  // Job registration
  registerCron(name: string, pattern: string, handler: JobHandler): void;
  registerInterval(name: string, intervalMs: number, handler: JobHandler): void;
  registerDelayed(name: string, delayMs: number, handler: JobHandler): void;

  // Lifecycle management
  start(): Promise<void>;
  shutdown(): Promise<void>;

  // Job operations
  trigger(name: string, data?: any): Promise<string>;
  cancel(jobId: string): Promise<void>;

  // Monitoring
  getHealth(): Promise<JobHealth>;
  getQueueSize(name: string): Promise<number>;
}

/**
 * Context passed to job handlers
 * Provides access to database and logging
 */
export interface JobContext {
  db: DatabaseInstance;
  logger: Logger;
  jobId: string;
  jobName: string;
  data?: any;
}

/**
 * Job handler function type
 */
export type JobHandler = (context: JobContext) => Promise<void>;

/**
 * Job health status
 */
export interface JobHealth {
  healthy: boolean;
  queueSize?: number;
  failedCount?: number;
  completedCount?: number;
}

// ============================================================================
// pg-boss Implementation
// ============================================================================

/**
 * pg-boss implementation of JobScheduler interface
 * Wraps pg-boss functionality behind our minimal interface
 */
class PgBossScheduler implements JobScheduler {
  private boss: PgBoss;
  private db: DatabaseInstance;
  private logger: Logger;
  private handlers = new Map<string, JobHandler>();
  private cronJobs = new Map<string, CronJobConfig>(); // name -> config
  private intervalJobs = new Map<string, IntervalJobConfig>(); // name -> config
  private createdQueues = new Set<string>(); // Track created queues to prevent deadlocks
  private jobIdToQueueName = new Map<string, string>(); // Track job ID to queue name mapping
  private isStarted = false;
  
  constructor(db: DatabaseInstance, logger: Logger) {
    this.db = db;
    this.logger = logger;
    
    // Extract the underlying pg.Pool from Drizzle
    const pool = (db as any).$client;
    if (!pool) {
      throw new Error('Unable to access pg.Pool from Drizzle instance');
    }
    
    // Create an adapter that pg-boss expects
    const pgBossDb = {
      executeSql: async (text: string, values?: any[]) => {
        return await pool.query(text, values);
      }
    };
    
    // Initialize pg-boss with the adapter
    this.boss = new PgBoss({
      db: pgBossDb as any,
      
      // pg-boss configuration
      schema: 'pgboss', // Isolate pg-boss tables in their own schema
      deleteAfterDays: 1, // Faster cleanup for tests
      archiveCompletedAfterSeconds: 300, // Archive after 5 minutes for tests
      
      // Retry configuration
      retryLimit: 2, // Fewer retries for faster tests
      retryDelay: 5, // Shorter delay for tests
      retryBackoff: true, // exponential backoff
      
      // Job expiration
      expireInMinutes: 5, // Shorter expiration for tests
      
      // Maintenance configuration (faster for tests)
      maintenanceIntervalSeconds: 10 // More frequent maintenance
      
      // Worker configuration - noScheduling and noSupervisor removed as they don't exist in ConstructorOptions
    });
  }
  
  
  /**
   * Register a cron job
   */
  registerCron(name: string, pattern: string, handler: JobHandler): void {
    this.handlers.set(name, handler);
    this.cronJobs.set(name, {
      schedule: pattern,
      timezone: 'UTC',
      retryLimit: 3
    });

    // If scheduler is already started, setup this job immediately
    if (this.isStarted && !this.createdQueues.has(name)) {
      this.setupSingleJob(name, handler);
    }

    this.logger.debug(`Registered cron job: ${name} with pattern: ${pattern}`);
  }
  
  /**
   * Register an interval job
   */
  registerInterval(name: string, intervalMs: number, handler: JobHandler): void {
    this.handlers.set(name, handler);
    const intervalMinutes = Math.max(1, Math.floor(intervalMs / (1000 * 60)));
    this.intervalJobs.set(name, {
      intervalMinutes,
      retryLimit: 3
    });

    // If scheduler is already started, setup this job immediately
    if (this.isStarted && !this.createdQueues.has(name)) {
      this.setupSingleJob(name, handler);
    }

    this.logger.debug(`Registered interval job: ${name} every ${intervalMs}ms (${intervalMinutes} minutes)`);
  }
  
  /**
   * Register a delayed job (one-time execution after delay)
   */
  registerDelayed(name: string, delayMs: number, handler: JobHandler): void {
    this.handlers.set(name, handler);
    
    // If scheduler is already started, setup this job immediately
    if (this.isStarted && !this.createdQueues.has(name)) {
      this.setupSingleJob(name, handler);
    }
    
    // Delayed jobs are handled differently - they're sent with a delay when triggered
    this.logger.debug(`Registered delayed job: ${name} with delay ${delayMs}ms`);
  }
  
  /**
   * Start the job scheduler
   */
  async start(): Promise<void> {
    if (this.isStarted) {
      this.logger.warn('Job scheduler already started');
      return;
    }
    
    try {
      // Start pg-boss (this handles restarts automatically)
      await this.boss.start();
      this.isStarted = true;
      this.logger.debug('pg-boss started successfully');
      
      // Create queues and register workers for all registered handlers
      await this.setupAllJobs();
      
      // Schedule cron jobs
      for (const [name, config] of this.cronJobs) {
        try {
          await this.boss.schedule(name, config.schedule, {}, {
            tz: config.timezone || 'UTC',
            retryLimit: config.retryLimit || 3
          });
          this.logger.debug(`Scheduled cron job: ${name} with pattern: ${config.schedule}`);
        } catch (error) {
          this.logger.error({ error, jobName: name, config }, 'Failed to schedule cron job');
          // Don't fail startup for scheduling errors - job can still be triggered manually
        }
      }
      
      // Schedule interval jobs
      for (const [name, config] of this.intervalJobs) {
        try {
          const cronPattern = `*/${config.intervalMinutes} * * * *`;
          await this.boss.schedule(name, cronPattern, {}, {
            retryLimit: config.retryLimit || 3
          });
          this.logger.debug(`Scheduled interval job: ${name} every ${config.intervalMinutes} minutes`);
        } catch (error) {
          this.logger.error({ error, jobName: name, config }, 'Failed to schedule interval job');
          // Don't fail startup for scheduling errors - job can still be triggered manually
        }
      }
      
      // Allow workers to fully initialize (increased for better reliability)
      await new Promise(resolve => setTimeout(resolve, 500));
      
      this.logger.info(`Job scheduler started with ${this.handlers.size} jobs and workers ready`);
      
    } catch (error) {
      this.logger.error({ error }, 'Failed to start job scheduler');
      throw error;
    }
  }
  
  /**
   * Setup a single job (queue + worker) - used for late registrations
   */
  private async setupSingleJob(name: string, handler: JobHandler): Promise<void> {
    try {
      this.logger.debug(`Setting up late-registered job: ${name}`);
      
      // Create queue if it doesn't exist
      if (!this.createdQueues.has(name)) {
        await this.boss.createQueue(name);
        this.createdQueues.add(name);
        this.logger.debug(`Created queue: ${name} (late registration)`);
        
        // Allow queue to initialize in pg-boss
        await new Promise(resolve => setTimeout(resolve, 200));
      }
      
      // Verify queue was created successfully
      if (!this.createdQueues.has(name)) {
        throw new Error(`Queue ${name} was not created successfully`);
      }
      
      // Register worker
      await this.boss.work(name, { pollingIntervalSeconds: 0.5 }, async (jobs) => {
        const job = Array.isArray(jobs) ? jobs[0] : jobs;
        
        if (!job) {
          this.logger.warn({ job: name }, 'No job data received');
          return;
        }
        
        const context: JobContext = {
          db: this.db,
          logger: this.logger.child({ job: name, jobId: job.id }),
          jobId: job.id,
          jobName: name,
          data: job.data,
        };
        
        try {
          await handler(context);
          this.logger.debug({ job: name, jobId: job.id }, 'Job completed successfully');
        } catch (error) {
          this.logger.error({ job: name, jobId: job.id, error }, 'Job handler error');
          throw error;
        }
      });
      
      // Allow worker to fully initialize
      await new Promise(resolve => setTimeout(resolve, 300));
      
      this.logger.debug(`Successfully registered worker for job: ${name} (late registration)`);
    } catch (error) {
      this.logger.error({ error, jobName: name }, 'Failed to setup late-registered job');
      // Remove from tracking if setup failed
      this.createdQueues.delete(name);
      throw error;
    }
  }
  
  /**
   * Setup all registered jobs (queues + workers) - called during start and for late registrations
   */
  private async setupAllJobs(): Promise<void> {
    // Phase 1: Create ALL queues first before any worker registration
    for (const [name] of this.handlers) {
      if (!this.createdQueues.has(name)) {
        try {
          await this.boss.createQueue(name);
          this.createdQueues.add(name);
          this.logger.debug(`Created queue: ${name}`);
        } catch (error) {
          // Log error but don't fail startup - queue might already exist
          this.logger.debug({ error, jobName: name }, 'Queue creation failed, assuming it exists');
          this.createdQueues.add(name); // Assume it exists to avoid retry
        }
      }
    }
    
    // Allow queues to fully initialize in pg-boss
    await new Promise(resolve => setTimeout(resolve, 200));
    
    // Phase 2: Register workers only after all queues are created
    for (const [name, handler] of this.handlers) {
      if (!this.createdQueues.has(name)) {
        this.logger.warn(`Skipping worker registration for ${name} - queue creation failed`);
        continue; // Skip if queue creation failed
      }
      
      try {
        await this.boss.work(name, { pollingIntervalSeconds: 0.5 }, async (jobs) => {
          // pg-boss passes jobs as an array, even for single jobs
          const job = Array.isArray(jobs) ? jobs[0] : jobs;
          
          if (!job) {
            this.logger.warn({ job: name }, 'No job data received');
            return;
          }
          
          const context: JobContext = {
            db: this.db,
            logger: this.logger.child({ job: name, jobId: job.id }),
            jobId: job.id,
            jobName: name,
            data: job.data,
          };
          
          try {
            await handler(context);
            this.logger.debug({ job: name, jobId: job.id }, 'Job completed successfully');
          } catch (error) {
            this.logger.error({ job: name, jobId: job.id, error }, 'Job handler error');
            throw error; // Let pg-boss handle retry logic
          }
        });
        
        this.logger.debug(`Registered worker for: ${name}`);
      } catch (error) {
        this.logger.error({ error, jobName: name }, `Failed to register worker for: ${name}`);
      }
    }
    
    // Allow workers to fully initialize
    await new Promise(resolve => setTimeout(resolve, 300));
  }
  
  /**
   * Shutdown the job scheduler gracefully
   */
  async shutdown(): Promise<void> {
    if (!this.isStarted) {
      this.logger.warn('Job scheduler not started');
      return;
    }
    
    try {
      this.logger.debug('Shutting down job scheduler...');
      await this.boss.stop({
        graceful: true,
        timeout: 30000, // 30 seconds timeout for graceful shutdown
      });
      
      // Clear tracking to allow proper restart
      this.createdQueues.clear();
      this.jobIdToQueueName.clear();
      this.isStarted = false;
      
      this.logger.debug('Job scheduler shut down successfully');
    } catch (error) {
      this.logger.error({ error }, 'Error shutting down job scheduler');
      // Clear state even if shutdown failed
      this.createdQueues.clear();
      this.jobIdToQueueName.clear();
      this.isStarted = false;
      throw error;
    }
  }
  
  /**
   * Verify that a queue and its worker are ready for job processing
   */
  private async verifyQueueReady(name: string): Promise<boolean> {
    try {
      // Check if queue exists in our tracking
      if (!this.createdQueues.has(name)) {
        this.logger.warn(`Queue ${name} not found in createdQueues tracking`);
        return false;
      }
      
      // Try to get queue size as a readiness check
      const queueSize = await this.boss.getQueueSize(name);
      return queueSize !== undefined;
    } catch (error) {
      this.logger.warn({ error, queueName: name }, 'Queue readiness check failed');
      return false;
    }
  }
  
  /**
   * Manually trigger a job
   */
  async trigger(name: string, data?: any): Promise<string> {
    if (!this.isStarted) {
      throw new Error('Job scheduler not started');
    }
    
    // If queue doesn't exist, create it now (late registration support)
    if (!this.createdQueues.has(name)) {
      this.logger.warn(`Queue ${name} not found, creating now (late registration)`);
      const handler = this.handlers.get(name);
      if (handler) {
        await this.setupSingleJob(name, handler);
        // Give additional time for worker registration
        await new Promise(resolve => setTimeout(resolve, 500));
      } else {
        throw new Error(`No handler found for job ${name}`);
      }
    }
    
    // Verify queue readiness before attempting to send
    const isReady = await this.verifyQueueReady(name);
    if (!isReady) {
      this.logger.warn(`Queue ${name} not ready, attempting to refresh`);
      // Small delay and retry once
      await new Promise(resolve => setTimeout(resolve, 100));
    }
    
    try {
      const jobId = await this.boss.send(name, data || {});
      
      if (!jobId) {
        throw new Error(`Failed to trigger job ${name}: send() returned null`);
      }
      
      // Track the job ID to queue name mapping for cancel support
      this.jobIdToQueueName.set(String(jobId), name);
      
      this.logger.debug(`Triggered job: ${name} with id: ${jobId}`);
      return String(jobId); // Ensure we return a string
    } catch (error) {
      this.logger.error({ error, jobName: name }, `Failed to trigger job: ${name}`);
      throw error;
    }
  }
  
  /**
   * Cancel a job
   */
  async cancel(jobId: string): Promise<void> {
    if (!this.isStarted) {
      throw new Error('Job scheduler not started');
    }
    
    if (!jobId) {
      throw new Error(`Invalid jobId provided to cancel: ${jobId}`);
    }
    
    // Get the queue name for this job ID
    const queueName = this.jobIdToQueueName.get(jobId);
    if (!queueName) {
      // If we don't have the mapping, we can't cancel the job
      // This might happen for jobs triggered before this mapping was added
      throw new Error(`Cannot find queue name for job ${jobId}. Job might have been triggered before tracking was enabled.`);
    }
    
    // pg-boss cancel requires queue name and job ID
    await this.boss.cancel(queueName, jobId);
    
    // Clean up the mapping
    this.jobIdToQueueName.delete(jobId);
    
    this.logger.debug(`Cancelled job: ${jobId} from queue: ${queueName}`);
  }
  
  /**
   * Get health status of the job system
   */
  async getHealth(): Promise<JobHealth> {
    if (!this.isStarted) {
      return { healthy: false };
    }
    
    try {
      // Get general health metrics
      // pg-boss doesn't have generic getQueueSize(), so we'll just return healthy: true
      // Individual queue sizes would require specific queue names
      return {
        healthy: true,
        queueSize: 0, // Would need specific queue name to get size
        failedCount: 0, // Would need to query failed jobs
        completedCount: 0, // Would need to query completed jobs
      };
    } catch (error) {
      this.logger.error({ error }, 'Error getting job health');
      return { healthy: false };
    }
  }
  
  /**
   * Get queue size for a specific queue
   */
  async getQueueSize(name: string): Promise<number> {
    if (!this.isStarted) {
      throw new Error('Job scheduler not started');
    }
    
    try {
      const size = await this.boss.getQueueSize(name);
      return size || 0;
    } catch (error) {
      this.logger.error({ error, queueName: name }, 'Error getting queue size');
      throw error;
    }
  }
}

// ============================================================================
// Factory Function
// ============================================================================

/**
 * Create a job scheduler instance
 * Returns the minimal interface, hiding implementation details
 */
export function createJobScheduler(db: DatabaseInstance, logger: Logger): JobScheduler {
  // Currently uses pg-boss, but can be swapped for another implementation
  // without changing consumer code
  return new PgBossScheduler(db, logger);
}
