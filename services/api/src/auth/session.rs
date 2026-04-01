use sea_orm::{ConnectionTrait, DatabaseConnection};

/// Generate a random session token (compatible with Better Auth format)
pub fn generate_token() -> String {
    use nanoid::nanoid;
    nanoid!(64)
}

/// Create session tables if they don't exist (for test environments)
pub async fn ensure_auth_tables(db: &DatabaseConnection) -> anyhow::Result<()> {
    // Create auth tables compatible with Better Auth schema (snake_case columns)
    let sql = r#"
        CREATE TABLE IF NOT EXISTS "user" (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            email TEXT NOT NULL UNIQUE,
            email_verified BOOLEAN DEFAULT FALSE NOT NULL,
            image TEXT,
            role TEXT,
            two_factor_enabled BOOLEAN DEFAULT FALSE,
            banned BOOLEAN DEFAULT FALSE,
            ban_reason TEXT,
            ban_expires TIMESTAMPTZ,
            created_at TIMESTAMPTZ DEFAULT NOW() NOT NULL,
            updated_at TIMESTAMPTZ DEFAULT NOW() NOT NULL
        );

        CREATE TABLE IF NOT EXISTS "account" (
            id TEXT PRIMARY KEY,
            account_id TEXT NOT NULL,
            provider_id TEXT NOT NULL,
            user_id TEXT NOT NULL REFERENCES "user"(id) ON DELETE CASCADE,
            password TEXT,
            access_token TEXT,
            refresh_token TEXT,
            id_token TEXT,
            access_token_expires_at TIMESTAMPTZ,
            refresh_token_expires_at TIMESTAMPTZ,
            scope TEXT,
            created_at TIMESTAMPTZ DEFAULT NOW() NOT NULL,
            updated_at TIMESTAMPTZ DEFAULT NOW() NOT NULL
        );

        CREATE INDEX IF NOT EXISTS account_userId_idx ON "account"(user_id);

        CREATE TABLE IF NOT EXISTS "session" (
            id TEXT PRIMARY KEY,
            token TEXT NOT NULL UNIQUE,
            expires_at TIMESTAMPTZ NOT NULL,
            user_id TEXT NOT NULL REFERENCES "user"(id) ON DELETE CASCADE,
            ip_address TEXT,
            user_agent TEXT,
            active_organization_id TEXT,
            created_at TIMESTAMPTZ DEFAULT NOW() NOT NULL,
            updated_at TIMESTAMPTZ DEFAULT NOW() NOT NULL
        );

        CREATE INDEX IF NOT EXISTS session_token_idx ON "session"(token);
        CREATE INDEX IF NOT EXISTS session_userId_idx ON "session"(user_id);

        CREATE TABLE IF NOT EXISTS "verification" (
            id TEXT PRIMARY KEY,
            identifier TEXT NOT NULL,
            value TEXT NOT NULL,
            expires_at TIMESTAMPTZ NOT NULL,
            created_at TIMESTAMPTZ DEFAULT NOW() NOT NULL,
            updated_at TIMESTAMPTZ DEFAULT NOW() NOT NULL
        );

        CREATE INDEX IF NOT EXISTS verification_identifier_idx ON "verification"(identifier);
    "#;

    db.execute_unprepared(sql).await?;
    Ok(())
}
