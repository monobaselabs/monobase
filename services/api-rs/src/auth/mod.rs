pub mod backend;
pub mod password;
pub mod session;

use std::collections::HashSet;

/// Authenticated user extracted from session.
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub id: String,
    pub name: String,
    pub email: String,
    pub email_verified: bool,
    pub image: Option<String>,
    pub roles: HashSet<String>,
    pub banned: bool,
    pub two_factor_enabled: bool,
}

impl AuthUser {
    /// Check if the user has a specific role.
    pub fn has_role(&self, role: &str) -> bool {
        self.roles.contains(role)
    }

    /// Check if the user has any of the given roles.
    pub fn has_any_role(&self, roles: &[&str]) -> bool {
        roles.iter().any(|r| self.roles.contains(*r))
    }

    /// Parse comma-separated role string into a HashSet.
    pub fn parse_roles(role_str: Option<&str>) -> HashSet<String> {
        role_str
            .unwrap_or("")
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    }
}
