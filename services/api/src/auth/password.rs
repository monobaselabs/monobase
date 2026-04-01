use anyhow::Result;

/// Verify a password against a hash.
/// Supports both bcrypt (legacy Better Auth) and argon2 (new users).
pub fn verify_password(password: &str, hash: &str) -> Result<bool> {
    // Try bcrypt first (legacy hashes start with $2b$ or $2a$)
    if hash.starts_with("$2b$") || hash.starts_with("$2a$") || hash.starts_with("$2y$") {
        return Ok(bcrypt::verify(password, hash)?);
    }

    // Try argon2
    if hash.starts_with("$argon2") {
        use argon2::{Argon2, PasswordHash, PasswordVerifier};
        let parsed_hash = PasswordHash::new(hash)
            .map_err(|e| anyhow::anyhow!("Invalid argon2 hash: {}", e))?;
        return Ok(Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok());
    }

    // Unknown hash format
    Ok(false)
}

/// Hash a password using bcrypt (compatible with Better Auth)
pub fn hash_password(password: &str) -> Result<String> {
    let hash = bcrypt::hash(password, 10)?;
    Ok(hash)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bcrypt_hash_and_verify() {
        let password = "testpassword123";
        let hash = hash_password(password).unwrap();
        assert!(verify_password(password, &hash).unwrap());
        assert!(!verify_password("wrongpassword", &hash).unwrap());
    }
}
