use sea_orm::{ConnectionTrait, DatabaseConnection};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// User record from the auth database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthUser {
    pub id: String,
    pub name: String,
    pub email: String,
    pub email_verified: bool,
    pub role: Option<String>,
    pub image: Option<String>,
}

/// Session record from the auth database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthSession {
    pub id: String,
    pub token: String,
    pub user_id: String,
    pub expires_at: String,
    pub active_organization_id: Option<String>,
}

/// Credentials for email/password sign-in
#[derive(Debug, Deserialize)]
pub struct Credentials {
    pub email: String,
    pub password: String,
}

/// Sign-up request body
#[derive(Debug, Deserialize)]
pub struct SignUpRequest {
    pub email: String,
    pub password: String,
    pub name: String,
}

/// Auth response returned by sign-up and sign-in
#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub user: AuthUserResponse,
    pub token: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthUserResponse {
    pub id: String,
    pub name: String,
    pub email: String,
    pub email_verified: bool,
    pub image: Option<String>,
    pub role: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

/// Look up a user by bearer token
pub async fn get_user_by_token(db: &DatabaseConnection, token: &str) -> anyhow::Result<Option<AuthUser>> {
    let sql = r#"
        SELECT u.id, u.name, u.email, u.email_verified, u.role, u.image
        FROM "session" s
        JOIN "user" u ON u.id = s.user_id
        WHERE s.token = $1 AND s.expires_at > NOW()
    "#;

    let rows = db.query_all(
        sea_orm::Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            sql,
            vec![token.into()],
        )
    ).await?;

    if let Some(row) = rows.first() {
        use sea_orm::TryGetable;
        Ok(Some(AuthUser {
            id: row.try_get("", "id")?,
            name: row.try_get("", "name")?,
            email: row.try_get("", "email")?,
            email_verified: row.try_get::<bool>("", "email_verified").unwrap_or(false),
            role: row.try_get("", "role").ok(),
            image: row.try_get("", "image").ok(),
        }))
    } else {
        Ok(None)
    }
}

/// Create a new user (sign-up)
pub async fn create_user(
    db: &DatabaseConnection,
    req: &SignUpRequest,
) -> anyhow::Result<AuthResponse> {
    use super::password::hash_password;
    use super::session::generate_token;

    let user_id = nanoid::nanoid!();
    let account_id = nanoid::nanoid!();
    let session_id = nanoid::nanoid!();
    let token = generate_token();
    let password_hash = hash_password(&req.password)?;

    // Create user (snake_case columns matching Better Auth Drizzle schema)
    let create_user_sql = r#"
        INSERT INTO "user" (id, name, email, email_verified, role, created_at, updated_at)
        VALUES ($1, $2, $3, false, 'user', NOW(), NOW())
    "#;
    db.execute(sea_orm::Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        create_user_sql,
        vec![user_id.clone().into(), req.name.clone().into(), req.email.to_lowercase().into()],
    )).await?;

    // Create account (credential provider)
    let create_account_sql = r#"
        INSERT INTO "account" (id, account_id, provider_id, user_id, password, created_at, updated_at)
        VALUES ($1, $2, 'credential', $3, $4, NOW(), NOW())
    "#;
    db.execute(sea_orm::Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        create_account_sql,
        vec![account_id.into(), user_id.clone().into(), user_id.clone().into(), password_hash.into()],
    )).await?;

    // Create session
    let create_session_sql = r#"
        INSERT INTO "session" (id, token, expires_at, user_id, created_at, updated_at)
        VALUES ($1, $2, NOW() + INTERVAL '12 hours', $3, NOW(), NOW())
    "#;
    db.execute(sea_orm::Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        create_session_sql,
        vec![session_id.into(), token.clone().into(), user_id.clone().into()],
    )).await?;

    Ok(AuthResponse {
        user: AuthUserResponse {
            id: user_id,
            name: req.name.clone(),
            email: req.email.to_lowercase(),
            email_verified: false,
            image: None,
            role: Some("user".to_string()),
            created_at: None,
            updated_at: None,
        },
        token,
    })
}

/// Authenticate a user (sign-in)
pub async fn authenticate(
    db: &DatabaseConnection,
    creds: &Credentials,
) -> anyhow::Result<Option<AuthResponse>> {
    use super::password::verify_password;
    use super::session::generate_token;

    // Find user and password hash (snake_case columns)
    let sql = r#"
        SELECT u.id, u.name, u.email, u.email_verified, u.role, u.image, a.password
        FROM "user" u
        JOIN "account" a ON a.user_id = u.id AND a.provider_id = 'credential'
        WHERE LOWER(u.email) = LOWER($1)
        LIMIT 1
    "#;

    let rows = db.query_all(
        sea_orm::Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            sql,
            vec![creds.email.clone().into()],
        )
    ).await?;

    let row = match rows.first() {
        Some(r) => r,
        None => return Ok(None),
    };

    use sea_orm::TryGetable;
    let password_hash: String = row.try_get("", "password")?;

    if !verify_password(&creds.password, &password_hash)? {
        return Ok(None);
    }

    let user_id: String = row.try_get("", "id")?;
    let user_name: String = row.try_get("", "name")?;
    let user_email: String = row.try_get("", "email")?;
    let email_verified: bool = row.try_get::<bool>("", "email_verified").unwrap_or(false);
    let role: Option<String> = row.try_get("", "role").ok();
    let image: Option<String> = row.try_get("", "image").ok();

    // Create new session
    let session_id = nanoid::nanoid!();
    let token = generate_token();

    let create_session_sql = r#"
        INSERT INTO "session" (id, token, expires_at, user_id, created_at, updated_at)
        VALUES ($1, $2, NOW() + INTERVAL '12 hours', $3, NOW(), NOW())
    "#;
    db.execute(sea_orm::Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        create_session_sql,
        vec![session_id.into(), token.clone().into(), user_id.clone().into()],
    )).await?;

    Ok(Some(AuthResponse {
        user: AuthUserResponse {
            id: user_id,
            name: user_name,
            email: user_email,
            email_verified,
            image,
            role,
            created_at: None,
            updated_at: None,
        },
        token,
    }))
}

/// Sign out - invalidate session
pub async fn sign_out(db: &DatabaseConnection, token: &str) -> anyhow::Result<()> {
    let sql = r#"DELETE FROM "session" WHERE token = $1"#;
    db.execute(sea_orm::Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        sql,
        vec![token.into()],
    )).await?;
    Ok(())
}

/// Get session info for a token
pub async fn get_session(db: &DatabaseConnection, token: &str) -> anyhow::Result<Option<Value>> {
    let user = get_user_by_token(db, token).await?;
    match user {
        Some(u) => Ok(Some(serde_json::json!({
            "user": {
                "id": u.id,
                "name": u.name,
                "email": u.email,
                "emailVerified": u.email_verified,
                "role": u.role,
                "image": u.image,
            },
            "session": {
                "userId": u.id,
            }
        }))),
        None => Ok(None),
    }
}
