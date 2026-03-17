use loom_core::perms::roles::{Role, RoleProfile, SYSTEM_ROLES};

// ===========================================================================
// SYSTEM_ROLES tests
// ===========================================================================

#[test]
fn test_system_roles_defined() {
    assert!(
        SYSTEM_ROLES.contains(&"Administrator"),
        "SYSTEM_ROLES should contain Administrator"
    );
    assert!(
        SYSTEM_ROLES.contains(&"System Manager"),
        "SYSTEM_ROLES should contain System Manager"
    );
    assert!(
        SYSTEM_ROLES.contains(&"All"),
        "SYSTEM_ROLES should contain All"
    );
    assert!(
        SYSTEM_ROLES.contains(&"Guest"),
        "SYSTEM_ROLES should contain Guest"
    );
}

#[test]
fn test_system_roles_count() {
    assert_eq!(
        SYSTEM_ROLES.len(),
        4,
        "SYSTEM_ROLES should have exactly 4 entries"
    );
}

// ===========================================================================
// Role struct tests
// ===========================================================================

#[test]
fn test_role_struct() {
    let role = Role {
        name: "HR Manager".to_string(),
        is_custom: true,
        disabled: false,
    };
    assert_eq!(role.name, "HR Manager");
    assert!(role.is_custom);
    assert!(!role.disabled);
}

#[test]
fn test_role_serde_roundtrip() {
    let role = Role {
        name: "Accounts User".to_string(),
        is_custom: false,
        disabled: true,
    };
    let json = serde_json::to_string(&role).expect("serialize Role");
    let deserialized: Role = serde_json::from_str(&json).expect("deserialize Role");
    assert_eq!(deserialized.name, "Accounts User");
    assert!(!deserialized.is_custom);
    assert!(deserialized.disabled);
}

#[test]
fn test_role_serde_defaults() {
    // is_custom and disabled should default to false when absent
    let json = r#"{"name": "Guest"}"#;
    let role: Role = serde_json::from_str(json).expect("deserialize with defaults");
    assert_eq!(role.name, "Guest");
    assert!(!role.is_custom);
    assert!(!role.disabled);
}

// ===========================================================================
// RoleProfile struct tests
// ===========================================================================

#[test]
fn test_role_profile_struct() {
    let profile = RoleProfile {
        name: "HR Full Access".to_string(),
        roles: vec![
            "HR Manager".to_string(),
            "HR User".to_string(),
            "Employee".to_string(),
        ],
    };
    assert_eq!(profile.name, "HR Full Access");
    assert_eq!(profile.roles.len(), 3);
    assert!(profile.roles.contains(&"HR Manager".to_string()));
    assert!(profile.roles.contains(&"HR User".to_string()));
    assert!(profile.roles.contains(&"Employee".to_string()));
}

#[test]
fn test_role_profile_serde_roundtrip() {
    let profile = RoleProfile {
        name: "Minimal".to_string(),
        roles: vec!["All".to_string()],
    };
    let json = serde_json::to_string(&profile).expect("serialize RoleProfile");
    let deserialized: RoleProfile = serde_json::from_str(&json).expect("deserialize RoleProfile");
    assert_eq!(deserialized.name, "Minimal");
    assert_eq!(deserialized.roles, vec!["All".to_string()]);
}
