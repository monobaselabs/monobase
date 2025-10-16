/**
 * Main server setup with Hono, Better-Auth, and generated routes
 * Uses factory pattern for dependency injection with proper cleanup support
 */

import { Hono } from 'hono';
import type { Variables, App } from '@/types/app';
import type { Config } from '@/core/config';

// Core dependencies
import { createAuth } from '@/core/auth';
import { ensureAdminUsers } from '@/utils/auth';
import { createDatabase, checkDatabaseConnection, closeDatabaseConnection, runMigrations, type DatabaseInstance } from '@/core/database';
import { createJobScheduler } from '@/core/jobs';
import { createLogger } from '@/core/logger';
import { createStorageProvider } from '@/core/storage';
import { createNotificationService } from '@/core/notifs';
import { createEmailService } from '@/core/email';
import { createAuditService } from '@/core/audit';
import { createWebSocketService } from '@/core/ws';
import { createBillingService } from '@/core/billing';
import { registerEmailJobs } from '@/handlers/email/jobs';
import { registerNotifsJobs } from '@/handlers/notifs/jobs';
import { registerAuditJobs } from '@/handlers/audit/jobs';
import { registerBookingJobs } from '@/handlers/booking/jobs';
import { registerAuthTemplates } from '@/handlers/auth/templates/email/loader';

// Routes
import { registerRoutes as registerOpenAPIRoutes } from '@/generated/openapi/routes';
import { registerRoutes as registerHealthRoutes } from '@/core/health';
import { registerRoutes as registerAuthRoutes } from '@/core/auth';
import { registerRoutes as registerDocsRoutes } from '@/core/openapi';
import { registerRoutes as registerWebSocketRoutes } from '@/generated/websocket/registry';
import { registerHandlers as registerErrorHandlers } from '@/core/errors';

// OpenAPI Specifications
import typespecOpenapi from '@monobase/api-spec/openapi.json';
import betterAuthOpenapi from '@/generated/better-auth/openapi.json';
import healthOpenapi from '@/core/health.openapi.json';

// Middleware
import { createRequestId, createRequestLogger } from '@/middleware/request';
import { createDependencyInjection } from '@/middleware/dependency';
import { createSecurityHeaders, createCorsMiddleware } from '@/middleware/security';


/**
 * Create and configure the Hono application with proper dependency injection
 * Returns the Hono app instance with database, logger, auth, and storage attached
 */
export function createApp(config: Config): App {
  const app = new Hono<{ Variables: Variables }>();

  // Generate internal service token for secure service-to-service communication
  // Used for expand requests and future microservice communication
  // TODO: Move to config/env for production deployments
  const internalServiceToken = crypto.randomUUID();

  // Create core dependencies with config
  const logger = createLogger(config);
  const database = createDatabase(config.database);
  const email = createEmailService(database, config, logger);
  const auth = createAuth(database, config, logger, email);
  const storage = createStorageProvider(config.storage, logger);
  const jobs = createJobScheduler(database, logger);
  const ws = createWebSocketService(logger);

  const notifs = createNotificationService(database, logger, config.notifs, ws);
  const audit = createAuditService(database, logger);
  const billing = createBillingService(config.billing, database, logger);

  // Attach dependencies to the app instance early for access throughout
  Object.assign(app, { database, logger, auth, storage, jobs, notifs, email, audit, ws, billing, internalServiceToken });

  // Global middleware - order matters!

  // Request ID generation - Needed for all logging
  app.use('*', createRequestId(config));

  // Dependency injection - Inject logger, database, storage, auth, jobs early
  app.use('*', createDependencyInjection(app as App, config));

  // Request logger - Log all incoming requests
  app.use('*', createRequestLogger(config));

  // Security headers - Lightweight, security critical
  app.use('*', createSecurityHeaders(config));

  // CORS - Required early for preflight
  app.use('*', createCorsMiddleware(config, logger));

  // Register health check endpoints
  registerHealthRoutes(app as App);

  // Register auth routes
  registerAuthRoutes(app as App);

  // Register API routes
  registerOpenAPIRoutes(app as any);

  // Register WebSocket handlers
  registerWebSocketRoutes(app as App);

  // Register documentation routes with multiple OpenAPI specs
  registerDocsRoutes(app as App, [typespecOpenapi, betterAuthOpenapi, healthOpenapi], config);

  // Register error handlers - must be last!
  registerErrorHandlers(app as App, config);

  return app as App;
}

/**
 * Initialize application components and dependencies
 * Handles database, admin users, and job scheduler initialization
 */
export async function initializeApp(app: App, config: Config): Promise<void> {
  const { database, logger, jobs } = app;

  // Run database migrations
  logger.debug('Running database migrations...');
  await runMigrations(database);
  logger.debug('Database migrations completed successfully');

  // Initialize email templates per module
  logger.debug('Initializing email templates...');
  await registerAuthTemplates(database, logger);
  logger.debug('Email templates initialized successfully');

  // Setup admin users if configured
  if (config.auth.adminEmails && config.auth.adminEmails.length > 0) {
    logger.debug('Setting up admin users...');
    const promotedEmails = await ensureAdminUsers(database, config.auth.adminEmails);
    if (promotedEmails.length > 0) {
      logger.info({ promotedEmails }, `Promoted ${promotedEmails.length} users to admin role`);
    } else {
      logger.debug('No existing users found to promote to admin role');
    }
    logger.debug('Admin user setup completed successfully');
  }

  // Initialize and start background job scheduler
  logger.debug('Registering jobs...');
  registerEmailJobs(jobs, app.email);
  registerNotifsJobs(jobs, app.notifs);
  registerAuditJobs(jobs);
  registerBookingJobs(jobs, app.notifs);
  logger.debug('Jobs registered successfully');
  
  logger.debug('Starting background job scheduler...');
  await jobs.start();
  logger.debug('Background job scheduler started successfully');
}

/**
 * Cleanup helper function for graceful shutdown
 * Extracts database, logger, auth, and storage from the app instance and performs cleanup
 */
export async function cleanupApp(app: App): Promise<void> {
  const { database, logger, jobs } = app;
  
  logger.debug('Cleaning up application resources...');
  
  // Shutdown job scheduler first
  if (jobs) {
    logger.debug('Shutting down job scheduler...');
    await jobs.shutdown();
    logger.debug('Job scheduler shutdown successfully');
  }
  
  // Gracefully close db conn 
  await closeDatabaseConnection(database);
  logger.debug('Database connection closed successfully');
}
