use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::doctype::meta::{FieldType, Meta};
use crate::error::LoomResult;

/// User Permission: link-based filtering.
/// e.g., "User X can only see documents where company = 'Acme'"
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPermission {
    pub user: String,
    /// The DocType this permission applies to (e.g., "Company")
    pub allow: String,
    /// The specific value (e.g., "Acme Corp")
    pub for_value: String,
    /// If set, this permission only applies to this specific DocType's documents
    #[serde(default)]
    pub applicable_for: Option<String>,
    #[serde(default)]
    pub is_default: bool,
}

/// Load all User Permissions for a given user from the `__user_permission` table.
pub async fn get_user_permissions(pool: &PgPool, user: &str) -> LoomResult<Vec<UserPermission>> {
    let rows: Vec<(String, String, String, Option<String>, bool)> = sqlx::query_as(
        "SELECT user_email, allow, for_value, applicable_for, is_default \
         FROM \"__user_permission\" WHERE user_email = $1",
    )
    .bind(user)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    Ok(rows
        .into_iter()
        .map(
            |(user, allow, for_value, applicable_for, is_default)| UserPermission {
                user,
                allow,
                for_value,
                applicable_for,
                is_default,
            },
        )
        .collect())
}

/// Build additional WHERE clauses that enforce User Permissions on a get_list query.
///
/// For each Link field in the DocType that points to a restricted DocType,
/// adds a filter: `link_field IN ('allowed_val_1', 'allowed_val_2', ...)`
///
/// Returns (clauses, bind_values) to be appended to the query.
pub fn build_user_perm_filters(
    meta: &Meta,
    user_perms: &[UserPermission],
    doctype: &str,
) -> (Vec<String>, Vec<String>) {
    let mut clauses = Vec::new();
    let mut bind_values = Vec::new();

    // Group user permissions by the DocType they restrict (the `allow` field)
    // Only include permissions that apply globally or specifically to this doctype
    let mut perm_map: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();

    for perm in user_perms {
        let applies = match &perm.applicable_for {
            Some(dt) => dt == doctype,
            None => true, // Global — applies to all DocTypes with matching Link fields
        };
        if applies {
            perm_map
                .entry(perm.allow.clone())
                .or_default()
                .push(perm.for_value.clone());
        }
    }

    if perm_map.is_empty() {
        return (clauses, bind_values);
    }

    // For each restricted DocType, find Link fields in this meta that point to it
    for (restricted_doctype, allowed_values) in &perm_map {
        // Check if this DocType itself is the restricted one (filter on "id" / "name")
        if restricted_doctype == doctype {
            let placeholders: Vec<String> = allowed_values
                .iter()
                .map(|v| {
                    bind_values.push(v.clone());
                    format!("${}", bind_values.len())
                })
                .collect();
            clauses.push(format!("\"id\" IN ({})", placeholders.join(", ")));
            continue;
        }

        // Find Link fields pointing to the restricted DocType
        for field in meta.fields.iter() {
            if field.fieldtype == FieldType::Link {
                let target = field.options.as_deref().unwrap_or("");
                if target == restricted_doctype {
                    let placeholders: Vec<String> = allowed_values
                        .iter()
                        .map(|v| {
                            bind_values.push(v.clone());
                            format!("${}", bind_values.len())
                        })
                        .collect();
                    clauses.push(format!(
                        "\"{}\" IN ({})",
                        field.fieldname,
                        placeholders.join(", ")
                    ));
                }
            }
        }
    }

    (clauses, bind_values)
}
