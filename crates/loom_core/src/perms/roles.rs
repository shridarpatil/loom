use serde::{Deserialize, Serialize};

/// A role that can be assigned to users.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    pub name: String,
    #[serde(default)]
    pub is_custom: bool,
    #[serde(default)]
    pub disabled: bool,
}

/// A role profile groups multiple roles together for easy assignment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleProfile {
    pub name: String,
    pub roles: Vec<String>,
}

/// Built-in system roles.
pub const SYSTEM_ROLES: &[&str] = &[
    "Administrator",
    "System Manager",
    "All",
    "Guest",
];
