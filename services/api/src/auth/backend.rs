use sea_orm::{ConnectionTrait, DatabaseConnection, FromQueryResult, JsonValue, Statement};

use super::{password, session, AuthUser};
use crate::error::ApiError;

/// Sign up with email and password.
/// Creates user + account + session rows (Better-Auth compatible).
pub async fn sign_up_email(
    db: &DatabaseConnection,
    name: &str,
    email: &str,
    password_raw: &str,
    session_expires_secs: u64,
    ip_address: Option<&str>,
    user_agent: Option<&str>,
) -> Result<(AuthUser, String), ApiError> {
    // Check if email already exists
    let existing = JsonValue::find_by_statement(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        r#"SELECT id FROM "user" WHERE email = $1"#,
        vec![email.into()],
    ))
    .one(db)
    .await
    .map_err(ApiError::Database)?;

    if existing.is_some() {
        return Err(ApiError::conflict("Email already registered"));
    }

    let user_id = uuid::Uuid::new_v4().to_string();
    let account_id = uuid::Uuid::new_v4().to_string();
    let password_hash = password::hash_password(password_raw)?;

    // Create user
    db.execute(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        r#"
            INSERT INTO "user" (id, name, email, email_verified, role, created_at, updated_at)
            VALUES ($1, $2, $3, false, 'user', NOW(), NOW())
        "#,
        vec![user_id.clone().into(), name.into(), email.into()],
    ))
    .await
    .map_err(ApiError::Database)?;

    // Create credential account
    db.execute(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        r#"
            INSERT INTO "account" (id, account_id, provider_id, user_id, password, created_at, updated_at)
            VALUES ($1, $2, 'credential', $3, $4, NOW(), NOW())
        "#,
        vec![
            account_id.into(),
            user_id.clone().into(),
            user_id.clone().into(),
            password_hash.into(),
        ],
    ))
    .await
    .map_err(ApiError::Database)?;

    // Create session
    let token = session::create_session(
        db,
        &user_id,
        session_expires_secs,
        ip_address,
        user_agent,
    )
    .await?;

    let user = AuthUser {
        id: user_id,
        name: name.to_string(),
        email: email.to_string(),
        email_verified: false,
        image: None,
        roles: AuthUser::parse_roles(Some("user")),
        banned: false,
        two_factor_enabled: false,
    };

    Ok((user, token))
}

/// Sign in with email and password.
pub async fn sign_in_email(
    db: &DatabaseConnection,
    email: &str,
    password_raw: &str,
    session_expires_secs: u64,
    ip_address: Option<&str>,
    user_agent: Option<&str>,
) -> Result<(AuthUser, String), ApiError> {
    // Look up user + credential account
    let row = JsonValue::find_by_statement(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        r#"
            SELECT u.id, u.name, u.email, u.email_verified, u.image, u.role,
                   u.banned, u.ban_expires, u.two_factor_enabled,
                   a.password
            FROM "user" u
            JOIN "account" a ON a.user_id = u.id AND a.provider_id = 'credential'
            WHERE u.email = $1
        "#,
        vec![email.into()],
    ))
    .one(db)
    .await
    .map_err(ApiError::Database)?
    .ok_or_else(|| ApiError::unauthorized("Invalid email or password"))?;

    // Verify password
    let stored_hash = row
        .get("password")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ApiError::unauthorized("Invalid email or password"))?;

    if !password::verify_password(password_raw, stored_hash)? {
        return Err(ApiError::unauthorized("Invalid email or password"));
    }

    // Check banned
    let banned = row.get("banned").and_then(|v| v.as_bool()).unwrap_or(false);
    if banned {
        return Err(ApiError::forbidden("Account is banned"));
    }

    let user_id = row
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    // Create session
    let token = session::create_session(
        db,
        &user_id,
        session_expires_secs,
        ip_address,
        user_agent,
    )
    .await?;

    let role_str = row.get("role").and_then(|v| v.as_str());
    let user = AuthUser {
        id: user_id,
        name: row
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        email: email.to_string(),
        email_verified: row
            .get("email_verified")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        image: row.get("image").and_then(|v| v.as_str()).map(String::from),
        roles: AuthUser::parse_roles(role_str),
        banned: false,
        two_factor_enabled: row
            .get("two_factor_enabled")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
    };

    Ok((user, token))
}
