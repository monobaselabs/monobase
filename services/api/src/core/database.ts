/**
 * Database connection factory functions without global state
 * Uses Drizzle ORM with PostgreSQL via pg.Pool or SQLite for embedded mode
 */

import { drizzle } from 'drizzle-orm/node-postgres';
import { Pool } from 'pg';
import type { NodePgDatabase } from 'drizzle-orm/node-postgres';
import { migrate } from 'drizzle-orm/node-postgres/migrator';
import type { Logger } from '@/types/logger';
import path from 'path';

export type Dialect = 'postgresql' | 'sqlite';

/**
 * Detect the dialect from a DATABASE_URL string.
 *
 * - `postgres://...` or `postgresql://...` → 'postgresql'
 * - `sqlite://...` or `:memory:` or path ending in `.db`/`.sqlite` → 'sqlite'
 * - Defaults to 'postgresql'
 */
export function detectDialect(databaseUrl: string): Dialect {
  if (
    databaseUrl?.startsWith('sqlite://') ||
    databaseUrl === ':memory:' ||
    /\.(db|sqlite|sqlite3)$/i.test(databaseUrl || '')
  ) {
    return 'sqlite';
  }
  return 'postgresql';
}

/**
 * Database configuration
 */
export interface DatabaseConfig {
  url: string;
  poolMin?: number;
  poolMax?: number;
  idleTimeoutMs?: number;
  ssl?: boolean;
  logging?: boolean;
}

/**
 * Database instance type - Drizzle instance with pg.Pool
 */
export type DatabaseInstance = NodePgDatabase;

/**
 * Create a new database instance with the given configuration
 * Returns the Drizzle instance directly for simplified usage
 *
 * Dialect is auto-detected from the URL:
 * - `postgres://` or `postgresql://` → PostgreSQL
 * - `sqlite://` or `:memory:` or `.db`/`.sqlite` → SQLite
 */
export function createDatabase(config: DatabaseConfig): DatabaseInstance {
  const dialect = detectDialect(config.url);

  if (dialect === 'sqlite') {
    // SQLite mode - for embedded Boa/Tauri apps
    // Note: SQLite is handled by Boa's __db native bridge, not here
    // This branch exists for future Bun-native SQLite support
    throw new Error(
      'SQLite is not yet supported in Bun server mode. ' +
      'For embedded apps, use the Boa JS engine with native SQLite bridge.'
    );
  }

  // PostgreSQL mode
  let schemaName: string | null = null;
  let cleanUrl = config.url;

  try {
    const url = new URL(config.url);
    schemaName = url.searchParams.get('schema');

    // Remove schema from connection string (pg doesn't support it as a connection parameter)
    if (schemaName) {
      url.searchParams.delete('schema');
      cleanUrl = url.toString();
    }
  } catch {
    // If URL parsing fails, use original URL
    cleanUrl = config.url;
  }

  // Create PostgreSQL connection pool
  const pool = new Pool({
    connectionString: cleanUrl,
    max: config.poolMax || 20,
    min: config.poolMin || 2,
    idleTimeoutMillis: config.idleTimeoutMs || 30000,
    connectionTimeoutMillis: 5000,
    ssl: config.ssl
      ? {
          rejectUnauthorized: true,
        }
      : false,
  });

  // Set search_path for ALL connections in the pool (critical for test schema isolation)
  if (schemaName) {
    pool.on('connect', async (client) => {
      try {
        await client.query(`SET search_path TO "${schemaName}", public`);
      } catch (error) {
        console.error(`Failed to set search_path to ${schemaName}:`, error);
        throw error;
      }
    });
  }

  // Create and return Drizzle database instance directly
  return drizzle(pool, {
    logger: config.logging || false,
  });
}

/**
 * Check if database connection is healthy
 * Uses Drizzle's execute method for health checks
 */
export async function checkDatabaseConnection(
  dbInstance: DatabaseInstance, 
  logger?: Logger
): Promise<boolean> {
  try {
    // Use Drizzle's execute method for health check
    await dbInstance.execute('SELECT 1 as health_check');
    return true;
  } catch (error) {
    if (logger) {
      logger.error({ error }, 'Database health check failed');
    }
    return false;
  }
}

/**
 * Close database connection and cleanup resources
 * Accesses underlying pg.Pool through $client property
 */
export async function closeDatabaseConnection(dbInstance: DatabaseInstance): Promise<void> {
  // Access the underlying pg.Pool and close it
  const pool = (dbInstance as any).$client;
  if (pool && typeof pool.end === 'function') {
    await pool.end();
  }
}

/**
 * @deprecated Use ctx.get('database') directly instead
 * Helper function to get database instance from Hono context
 * Returns the Drizzle instance directly for database operations
 */
export function getDatabaseFromContext(ctx: any): DatabaseInstance {
  const database = ctx.get('database');
  if (!database) {
    throw new Error('Database instance not found in context. Make sure dependency injection middleware is properly configured.');
  }
  return database;
}

/**
 * Run database migrations
 * Applies all pending migrations from the specified folder
 */
export async function runMigrations(
  dbInstance: DatabaseInstance, 
  migrationsFolder?: string
): Promise<void> {
  // Use absolute path to migrations directory
  const folder = migrationsFolder || path.join(__dirname, '../generated/migrations');

  await migrate(dbInstance, { migrationsFolder: folder });
}

