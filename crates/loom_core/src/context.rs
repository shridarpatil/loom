use std::collections::HashMap;
use std::sync::Arc;

use sqlx::PgPool;

use crate::doctype::DocTypeRegistry;

/// Request context that flows through the entire request lifecycle.
/// Contains user identity, permissions, site info, and database access.
#[derive(Debug, Clone)]
pub struct RequestContext {
    pub user: String,
    pub roles: Vec<String>,
    pub site: String,
    pub flags: HashMap<String, bool>,
    pub registry: Arc<DocTypeRegistry>,
    pool: PgPool,
}

impl RequestContext {
    /// Create a new request context for an authenticated user.
    pub fn new(
        user: String,
        roles: Vec<String>,
        site: String,
        pool: PgPool,
        registry: Arc<DocTypeRegistry>,
    ) -> Self {
        Self {
            user,
            roles,
            site,
            flags: HashMap::new(),
            registry,
            pool,
        }
    }

    /// Create a system context (for CLI, migrations, background jobs).
    /// Runs as Administrator with all permissions.
    pub fn system(pool: PgPool, registry: Arc<DocTypeRegistry>) -> Self {
        Self {
            user: "Administrator".to_string(),
            roles: vec!["Administrator".to_string(), "All".to_string()],
            site: "default".to_string(),
            flags: HashMap::new(),
            registry,
            pool,
        }
    }

    /// Create a guest context (unauthenticated requests).
    pub fn guest(pool: PgPool, registry: Arc<DocTypeRegistry>) -> Self {
        Self {
            user: "Guest".to_string(),
            roles: vec!["Guest".to_string(), "All".to_string()],
            site: "default".to_string(),
            flags: HashMap::new(),
            registry,
            pool,
        }
    }

    /// Check if the current user is the Administrator.
    pub fn is_administrator(&self) -> bool {
        self.user == "Administrator" || self.roles.iter().any(|r| r == "Administrator")
    }

    /// Check if the current user has a specific role.
    pub fn has_role(&self, role: &str) -> bool {
        self.is_administrator() || self.roles.iter().any(|r| r == role)
    }

    /// Get a reference to the database pool.
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Check if permissions should be ignored (e.g., system operations).
    pub fn ignore_permissions(&self) -> bool {
        self.is_administrator()
            || self
                .flags
                .get("ignore_permissions")
                .copied()
                .unwrap_or(false)
    }
}
