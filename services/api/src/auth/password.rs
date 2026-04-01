use crate::error::ApiError;

/// Hash a password using bcrypt (compatible with Better-Auth).
pub fn hash_password(password: &str) -> Result<String, ApiError> {
    bcrypt::hash(password, bcrypt::DEFAULT_COST).map_err(|e| {
        ApiError::internal(format!("Password hashing failed: {}", e))
    })
}

/// Verify a password against a bcrypt hash.
/// Supports Better-Auth's bcrypt format ($2b$, $2a$, $2y$).
pub fn verify_password(password: &str, hash: &str) -> Result<bool, ApiError> {
    bcrypt::verify(password, hash).map_err(|e| {
        ApiError::internal(format!("Password verification failed: {}", e))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_and_verify() {
        let password = "test-password-123";
        let hash = hash_password(password).unwrap();
        assert!(verify_password(password, &hash).unwrap());
        assert!(!verify_password("wrong-password", &hash).unwrap());
    }

    #[test]
    fn test_bcrypt_format() {
        let hash = hash_password("test").unwrap();
        // bcrypt hashes start with $2b$ (or $2a$, $2y$)
        assert!(hash.starts_with("$2b$") || hash.starts_with("$2a$") || hash.starts_with("$2y$"));
    }
}
