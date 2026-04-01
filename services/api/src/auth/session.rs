use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use hmac::{Hmac, Mac};
use sea_orm::{ConnectionTrait, DatabaseConnection, FromQueryResult, JsonValue, Statement};
use sha2::Sha256;

use super::AuthUser;
use crate::error::ApiError;

type HmacSha256 = Hmac<Sha256>;

/// Verify a Better-Auth signed cookie token.
///
/// Format: `{raw_token}.{base64_hmac_signature}`
/// The cookie value may be URL-encoded.
pub fn verify_signed_token(cookie_value: &str, secret: &str) -> Result<String, ApiError> {
    let decoded = urlencoding::decode(cookie_value)
        .map_err(|_| ApiError::unauthorized("Invalid token encoding"))?;

    // Split on last '.' to get raw_token and signature
    let last_dot = decoded
        .rfind('.')
        .ok_or_else(|| ApiError::unauthorized("Invalid token format"))?;

    let raw_token = &decoded[..last_dot];
    let signature = &decoded[last_dot + 1..];

    // Verify HMAC-SHA256
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|_| ApiError::internal("HMAC key error"))?;
    mac.update(raw_token.as_bytes());
    let expected = BASE64.encode(mac.finalize().into_bytes());

    if signature != expected {
        return Err(ApiError::unauthorized("Invalid token signature"));
    }

    Ok(raw_token.to_string())
}

/// Extract bearer token from Authorization header.
/// Returns the raw token after signature verification.
pub fn extract_bearer_token(auth_header: &str, secret: &str) -> Result<String, ApiError> {
    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or_else(|| ApiError::unauthorized("Invalid authorization header"))?;

    // If token contains a '.', it's a signed token — verify it
    if token.contains('.') {
        verify_signed_token(token, secret)
    } else {
        // Raw unsigned token
        Ok(token.to_string())
    }
}

/// Look up a session + user by raw token.
pub async fn get_user_by_token(
    db: &DatabaseConnection,
    token: &str,
) -> Result<Option<AuthUser>, ApiError> {
    let sql = r#"
        SELECT
            u.id, u.name, u.email, u.email_verified, u.image,
            u.role, u.banned, u.ban_reason, u.ban_expires,
            u.two_factor_enabled,
            s.expires_at
        FROM "session" s
        JOIN "user" u ON u.id = s.user_id
        WHERE s.token = $1
    "#;

    let result = JsonValue::find_by_statement(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        sql,
        vec![token.into()],
    ))
    .one(db)
    .await
    .map_err(ApiError::Database)?;

    let row = match result {
        Some(row) => row,
        None => return Ok(None),
    };

    // Check session expiry
    if let Some(expires_at) = row.get("expires_at").and_then(|v| v.as_str()) {
        if let Ok(expires) = chrono::DateTime::parse_from_rfc3339(expires_at) {
            if expires < chrono::Utc::now() {
                return Ok(None);
            }
        }
    }

    // Check banned status
    let banned = row.get("banned").and_then(|v| v.as_bool()).unwrap_or(false);
    if banned {
        // Check if ban has expired
        if let Some(ban_expires) = row.get("ban_expires").and_then(|v| v.as_str()) {
            if let Ok(expires) = chrono::DateTime::parse_from_rfc3339(ban_expires) {
                if expires > chrono::Utc::now() {
                    return Err(ApiError::forbidden("Account is banned"));
                }
            }
        } else {
            return Err(ApiError::forbidden("Account is banned"));
        }
    }

    let role_str = row.get("role").and_then(|v| v.as_str());
    let roles = AuthUser::parse_roles(role_str);

    Ok(Some(AuthUser {
        id: row
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        name: row
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        email: row
            .get("email")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        email_verified: row
            .get("email_verified")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        image: row.get("image").and_then(|v| v.as_str()).map(String::from),
        roles,
        banned: false, // Already checked above
        two_factor_enabled: row
            .get("two_factor_enabled")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
    }))
}

/// Create a new session for a user.
pub async fn create_session(
    db: &DatabaseConnection,
    user_id: &str,
    expires_secs: u64,
    ip_address: Option<&str>,
    user_agent: Option<&str>,
) -> Result<String, ApiError> {
    let session_id = uuid::Uuid::new_v4().to_string();
    let token = uuid::Uuid::new_v4().to_string();
    let expires_at = chrono::Utc::now()
        + chrono::Duration::seconds(expires_secs as i64);

    let sql = r#"
        INSERT INTO "session" (id, token, user_id, expires_at, ip_address, user_agent, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, NOW(), NOW())
    "#;

    db.execute(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        sql,
        vec![
            session_id.into(),
            token.clone().into(),
            user_id.into(),
            expires_at.to_rfc3339().into(),
            ip_address.unwrap_or("").into(),
            user_agent.unwrap_or("").into(),
        ],
    ))
    .await
    .map_err(ApiError::Database)?;

    Ok(token)
}

/// Delete a session by token.
pub async fn delete_session(db: &DatabaseConnection, token: &str) -> Result<(), ApiError> {
    let sql = r#"DELETE FROM "session" WHERE token = $1"#;
    db.execute(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        sql,
        vec![token.into()],
    ))
    .await
    .map_err(ApiError::Database)?;

    Ok(())
}

/// Sign a raw token to produce a Better-Auth compatible signed cookie value.
pub fn sign_token(raw_token: &str, secret: &str) -> Result<String, ApiError> {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|_| ApiError::internal("HMAC key error"))?;
    mac.update(raw_token.as_bytes());
    let signature = BASE64.encode(mac.finalize().into_bytes());
    Ok(format!("{}.{}", raw_token, signature))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sign_and_verify() {
        let secret = "test-secret-key";
        let raw_token = "550e8400-e29b-41d4-a716-446655440000";

        let signed = sign_token(raw_token, secret).unwrap();
        let verified = verify_signed_token(&signed, secret).unwrap();
        assert_eq!(verified, raw_token);
    }

    #[test]
    fn test_verify_bad_signature() {
        let secret = "test-secret-key";
        let bad_token = "some-token.badsignature";
        assert!(verify_signed_token(bad_token, secret).is_err());
    }

    #[test]
    fn test_verify_url_encoded() {
        let secret = "test-secret-key";
        let raw_token = "test-token";

        let signed = sign_token(raw_token, secret).unwrap();
        let encoded = urlencoding::encode(&signed).to_string();
        let verified = verify_signed_token(&encoded, secret).unwrap();
        assert_eq!(verified, raw_token);
    }
}
