use std::collections::HashSet;

use serde_json::Value;

use crate::doctype::meta::Meta;
use crate::error::{LoomError, LoomResult};

use super::docperm::{perm_grants, PermType};

/// Check if a user has the given permission for a DocType (and optionally a specific document).
pub fn has_permission(
    meta: &Meta,
    doc: Option<&Value>,
    ptype: PermType,
    user: &str,
    user_roles: &[String],
) -> bool {
    // Administrator has all permissions
    if user == "Administrator" || user_roles.iter().any(|r| r == "Administrator") {
        return true;
    }

    for perm in &meta.permissions {
        // Check if user has this role
        if !user_roles.iter().any(|r| r == &perm.role) {
            continue;
        }

        // Check if this perm rule grants the requested permission type
        if !perm_grants(perm, ptype) {
            continue;
        }

        // Check if_owner: user can only access docs they own
        if perm.if_owner {
            if let Some(doc) = doc {
                let owner = doc.get("owner").and_then(|v| v.as_str()).unwrap_or("");
                if owner != user {
                    continue;
                }
            }
            // If no doc provided, if_owner is satisfied (for create checks)
        }

        return true;
    }

    false
}

/// Check permission and return an error if denied.
pub fn check_permission(
    meta: &Meta,
    doc: Option<&Value>,
    ptype: PermType,
    user: &str,
    user_roles: &[String],
) -> LoomResult<()> {
    if has_permission(meta, doc, ptype, user, user_roles) {
        Ok(())
    } else {
        Err(LoomError::PermissionDenied(format!(
            "No {} permission for {}",
            ptype.as_str(),
            meta.name
        )))
    }
}

/// Standard fields that are always visible regardless of permlevel.
const ALWAYS_VISIBLE_FIELDS: &[&str] = &[
    "id",
    "name",
    "owner",
    "creation",
    "modified",
    "modified_by",
    "docstatus",
    "idx",
    "parent",
    "parentfield",
    "parenttype",
];

/// Compute which permlevels a user's roles grant for a given permission type.
/// For Read: higher levels imply level 0 (level 1 read = see level 0 + level 1 fields).
/// For Write: each level is independent (level 1 write only applies to level 1 fields).
pub fn allowed_permlevels(meta: &Meta, ptype: PermType, user_roles: &[String]) -> HashSet<u8> {
    let mut levels = HashSet::new();
    for perm in &meta.permissions {
        if !user_roles.iter().any(|r| r == &perm.role) {
            continue;
        }
        if !perm_grants(perm, ptype) {
            continue;
        }
        levels.insert(perm.permlevel);
    }
    // For read access, having any level implies level 0 visibility
    if ptype == PermType::Read && !levels.is_empty() {
        levels.insert(0);
    }
    levels
}

/// Remove fields from a document whose permlevel is not in the allowed set.
/// Standard fields (id, owner, creation, etc.) are always kept.
pub fn strip_fields_by_permlevel(doc: &mut Value, meta: &Meta, allowed_levels: &HashSet<u8>) {
    if let Some(obj) = doc.as_object_mut() {
        let keys_to_remove: Vec<String> = obj
            .keys()
            .filter(|key| {
                // Always keep standard fields
                if ALWAYS_VISIBLE_FIELDS.contains(&key.as_str()) {
                    return false;
                }
                // Look up the field in meta to find its permlevel
                if let Some(field_meta) = meta.get_field(key) {
                    !allowed_levels.contains(&field_meta.permlevel)
                } else {
                    // Field not in meta — keep it (might be a computed/virtual field)
                    false
                }
            })
            .cloned()
            .collect();

        for key in keys_to_remove {
            obj.remove(&key);
        }
    }
}

/// Check which incoming fields a user is not allowed to write (by permlevel)
/// and return an error listing the first disallowed field.
pub fn check_write_permlevels(
    incoming: &Value,
    meta: &Meta,
    allowed_write_levels: &HashSet<u8>,
) -> LoomResult<()> {
    if let Some(obj) = incoming.as_object() {
        for key in obj.keys() {
            if ALWAYS_VISIBLE_FIELDS.contains(&key.as_str()) {
                continue;
            }
            if let Some(field_meta) = meta.get_field(key) {
                if !allowed_write_levels.contains(&field_meta.permlevel) {
                    return Err(LoomError::PermissionDenied(format!(
                        "No write permission for field '{}' at permlevel {}",
                        key, field_meta.permlevel
                    )));
                }
            }
        }
    }
    Ok(())
}
