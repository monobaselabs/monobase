/**
 * Direct PostgreSQL access for test cleanup.
 */

import pg from "pg";

const DATABASE_URL =
  process.env.TEST_DATABASE_URL ||
  process.env.DATABASE_URL ||
  "postgresql://postgres:password@127.0.0.1:5432/monobase";

let pool: pg.Pool | null = null;

function getPool(): pg.Pool {
  if (!pool) {
    pool = new pg.Pool({ connectionString: DATABASE_URL, max: 3 });
  }
  return pool;
}

export async function dbQuery(sql: string, params?: any[]): Promise<any[]> {
  const client = await getPool().connect();
  try {
    const result = await client.query(sql, params);
    return result.rows;
  } finally {
    client.release();
  }
}

/** Truncate all application tables (not auth tables, to preserve test users). */
export async function resetDatabase(): Promise<void> {
  const tables = [
    "review",
    "notification",
    "consultation_note",
    "chat_message",
    "chat_room",
    "booking",
    "time_slot",
    "schedule_exception",
    "booking_event",
    "invoice_line_item",
    "invoice",
    "merchant_account",
    "email_queue",
    "email_template",
    "stored_file",
    "audit_log_entry",
    "patient",
    "provider",
    "person",
  ];

  for (const table of tables) {
    try {
      await dbQuery(`TRUNCATE TABLE "${table}" CASCADE`);
    } catch {
      // Table may not exist yet — skip
    }
  }
}

/** Truncate everything including auth tables. Full reset. */
export async function resetDatabaseFull(): Promise<void> {
  await resetDatabase();
  const authTables = ["session", "account", "verification", "passkey", "two_factor", "apikey", "user"];
  for (const table of authTables) {
    try {
      await dbQuery(`TRUNCATE TABLE "${table}" CASCADE`);
    } catch {
      // Skip
    }
  }
}

export async function closePool(): Promise<void> {
  if (pool) {
    await pool.end();
    pool = null;
  }
}
