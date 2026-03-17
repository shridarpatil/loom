use sha2::{Sha256, Digest};
use sqlx::PgPool;

use crate::doctype::meta::{Meta, STANDARD_FIELDS};
use crate::error::{LoomError, LoomResult};

/// Hash a password with SHA-256 and a fixed salt prefix.
pub fn hash_password(password: &str) -> String {
    let salted = format!("loom_salt_{}", password);
    let mut hasher = Sha256::new();
    hasher.update(salted.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Verify a password against a stored hash.
pub fn verify_password(password: &str, stored_hash: &str) -> bool {
    hash_password(password) == stored_hash
}

/// Create system tables required by the framework.
/// These are internal tables prefixed with `__`.
pub async fn migrate_system_tables(pool: &PgPool) -> LoomResult<()> {
    // __doctype: stores DocType metadata as JSONB
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS \"__doctype\" (
            name VARCHAR(140) PRIMARY KEY,
            module VARCHAR(140),
            meta JSONB NOT NULL,
            modified TIMESTAMP DEFAULT NOW()
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| LoomError::Internal(format!("Failed to create __doctype table: {}", e)))?;

    // __naming_series: counters for series-based naming
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS \"__naming_series\" (
            name VARCHAR(140) PRIMARY KEY,
            current BIGINT NOT NULL DEFAULT 0
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| {
        LoomError::Internal(format!("Failed to create __naming_series table: {}", e))
    })?;

    // __script: stores Rhai scripts for DocTypes
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS \"__script\" (
            name VARCHAR(140) PRIMARY KEY,
            doctype VARCHAR(140),
            script TEXT NOT NULL,
            modified TIMESTAMP DEFAULT NOW()
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| LoomError::Internal(format!("Failed to create __script table: {}", e)))?;

    // __user_api_key: stores API keys for token-based auth
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS \"__user_api_key\" (
            api_key VARCHAR(140) PRIMARY KEY,
            api_secret VARCHAR(140) NOT NULL,
            user_name VARCHAR(140) NOT NULL
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| {
        LoomError::Internal(format!("Failed to create __user_api_key table: {}", e))
    })?;

    // __customization: per-site DocType customizations (field overrides + client scripts)
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS \"__customization\" (
            doctype VARCHAR(140) PRIMARY KEY,
            overrides JSONB NOT NULL DEFAULT '{}',
            client_script TEXT DEFAULT '',
            modified TIMESTAMP DEFAULT NOW()
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| {
        LoomError::Internal(format!("Failed to create __customization table: {}", e))
    })?;

    // __user: stores user accounts
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS \"__user\" (
            email VARCHAR(140) PRIMARY KEY,
            password_hash VARCHAR(256) NOT NULL,
            full_name VARCHAR(140) DEFAULT '',
            enabled BOOLEAN DEFAULT TRUE,
            roles JSONB DEFAULT '[]',
            creation TIMESTAMP DEFAULT NOW(),
            modified TIMESTAMP DEFAULT NOW()
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| LoomError::Internal(format!("Failed to create __user table: {}", e)))?;

    // __session: stores active login sessions
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS \"__session\" (
            sid VARCHAR(64) PRIMARY KEY,
            user_email VARCHAR(140) NOT NULL,
            created TIMESTAMP DEFAULT NOW(),
            expires TIMESTAMP NOT NULL
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| LoomError::Internal(format!("Failed to create __session table: {}", e)))?;

    // __site_config: stores site-level configuration (theme, branding, etc.)
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS \"__site_config\" (
            key VARCHAR(140) PRIMARY KEY,
            value JSONB NOT NULL,
            modified TIMESTAMP DEFAULT NOW()
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| {
        LoomError::Internal(format!("Failed to create __site_config table: {}", e))
    })?;

    // __user_settings: stores per-user settings (sidebar, list views, etc.)
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS \"__user_settings\" (
            user_email VARCHAR(140) NOT NULL,
            key VARCHAR(140) NOT NULL,
            value JSONB NOT NULL,
            modified TIMESTAMP DEFAULT NOW(),
            PRIMARY KEY (user_email, key)
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| {
        LoomError::Internal(format!("Failed to create __user_settings table: {}", e))
    })?;

    // __user_permission: link-based row filtering per user
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS \"__user_permission\" (
            id BIGSERIAL PRIMARY KEY,
            user_email VARCHAR(140) NOT NULL,
            allow VARCHAR(140) NOT NULL,
            for_value VARCHAR(140) NOT NULL,
            applicable_for VARCHAR(140),
            is_default BOOLEAN DEFAULT FALSE
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| {
        LoomError::Internal(format!("Failed to create __user_permission table: {}", e))
    })?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_user_perm_user ON \"__user_permission\" (user_email)",
    )
    .execute(pool)
    .await
    .ok();

    // __job_queue: background job queue with named queues and priority
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS \"__job_queue\" (
            id BIGSERIAL PRIMARY KEY,
            method VARCHAR(280) NOT NULL,
            args JSONB DEFAULT '{}',
            queue VARCHAR(60) NOT NULL DEFAULT 'default',
            priority INTEGER NOT NULL DEFAULT 0,
            status VARCHAR(20) NOT NULL DEFAULT 'queued',
            attempts INTEGER DEFAULT 0,
            max_retries INTEGER DEFAULT 3,
            error TEXT,
            created TIMESTAMP DEFAULT NOW(),
            started TIMESTAMP,
            finished TIMESTAMP
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| LoomError::Internal(format!("Failed to create __job_queue table: {}", e)))?;

    // Index for fast dequeue: queue + status + priority
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_job_queue_dequeue ON \"__job_queue\" (queue, status, priority DESC, id)",
    )
    .execute(pool)
    .await
    .ok();

    // Add columns if table already existed (migration for existing installs)
    sqlx::query("ALTER TABLE \"__job_queue\" ADD COLUMN IF NOT EXISTS queue VARCHAR(60) NOT NULL DEFAULT 'default'")
        .execute(pool).await.ok();
    sqlx::query("ALTER TABLE \"__job_queue\" ADD COLUMN IF NOT EXISTS priority INTEGER NOT NULL DEFAULT 0")
        .execute(pool).await.ok();

    // __activity: stores document activity / audit trail
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS \"__activity\" (
            id BIGSERIAL PRIMARY KEY,
            doctype VARCHAR(140) NOT NULL,
            docname VARCHAR(140) NOT NULL,
            action VARCHAR(40) NOT NULL,
            user_email VARCHAR(140) NOT NULL,
            timestamp TIMESTAMP DEFAULT NOW(),
            data JSONB DEFAULT '{}'
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| LoomError::Internal(format!("Failed to create __activity table: {}", e)))?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_activity_doc ON \"__activity\" (doctype, docname)",
    )
    .execute(pool)
    .await
    .ok();

    // Seed Administrator user if not exists
    let admin_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM \"__user\" WHERE email = 'Administrator')",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(false);

    if !admin_exists {
        // Default password: "admin" — user should change on first login
        let hash = hash_password("admin");
        sqlx::query(
            "INSERT INTO \"__user\" (email, password_hash, full_name, roles) VALUES ('Administrator', $1, 'Administrator', '[\"Administrator\", \"System Manager\", \"All\"]')"
        )
        .bind(&hash)
        .execute(pool)
        .await
        .map_err(|e| LoomError::Internal(format!("Failed to seed admin user: {}", e)))?;
        tracing::info!("Seeded default Administrator user (password: admin)");
    }

    tracing::info!("System tables created/verified");
    Ok(())
}

/// Seed the Administrator user into the User DocType table if it exists but admin is missing.
/// Call this after core DocTypes are loaded and migrated.
pub async fn seed_admin_to_doctype_table(pool: &PgPool) -> LoomResult<()> {
    let user_table = crate::doctype::meta::doctype_table_name("User");

    // Check if User table exists
    let table_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT FROM information_schema.tables WHERE table_name = $1)",
    )
    .bind(&user_table)
    .fetch_one(pool)
    .await
    .unwrap_or(false);

    if !table_exists {
        return Ok(());
    }

    // Check if Administrator already exists
    let sql = format!("SELECT EXISTS (SELECT 1 FROM \"{}\" WHERE id = 'Administrator')", user_table);
    let admin_exists: bool = sqlx::query_scalar(&sql)
        .fetch_one(pool)
        .await
        .unwrap_or(false);

    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.6f").to_string();

    if !admin_exists {
    // Copy from __user if it exists there
    let legacy_admin: Option<(String, String)> = sqlx::query_as(
        "SELECT password_hash, full_name FROM \"__user\" WHERE email = 'Administrator'",
    )
    .fetch_optional(pool)
    .await
    .unwrap_or(None);

    let (password_hash, full_name) = legacy_admin
        .unwrap_or_else(|| (hash_password("admin"), "Administrator".to_string()));

    let sql = format!(
        "INSERT INTO \"{}\" (id, email, full_name, password_hash, enabled, roles_json, owner, creation, modified, modified_by, docstatus) \
         VALUES ('Administrator', 'Administrator', $1, $2, 'true', '[\"Administrator\", \"System Manager\", \"All\"]', 'Administrator', $3::TIMESTAMP, $3::TIMESTAMP, 'Administrator', 0) \
         ON CONFLICT (id) DO NOTHING",
        user_table
    );
    sqlx::query(&sql)
        .bind(&full_name)
        .bind(&password_hash)
        .bind(&now)
        .execute(pool)
        .await
        .map_err(|e| LoomError::Internal(format!("Failed to seed admin to {}: {}", user_table, e)))?;

    tracing::info!("Seeded Administrator into {}", user_table);
    } // end if !admin_exists

    // Seed default roles into Role DocType table
    let role_table = crate::doctype::meta::doctype_table_name("Role");
    let role_table_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT FROM information_schema.tables WHERE table_name = $1)",
    )
    .bind(&role_table)
    .fetch_one(pool)
    .await
    .unwrap_or(false);

    if role_table_exists {
        let default_roles = [
            ("Administrator", "Full system access", "Core", false),
            ("System Manager", "Can manage users, roles, and settings", "Core", false),
            ("All", "Default role assigned to every user", "Core", false),
            ("Guest", "Unauthenticated users", "Core", false),
        ];

        for (role_name, description, module, is_custom) in &default_roles {
            let check_sql = format!("SELECT EXISTS (SELECT 1 FROM \"{}\" WHERE id = $1)", role_table);
            let exists: bool = sqlx::query_scalar(&check_sql)
                .bind(role_name)
                .fetch_one(pool)
                .await
                .unwrap_or(false);

            if !exists {
                let insert_sql = format!(
                    "INSERT INTO \"{}\" (id, role_name, module, description, is_custom, owner, creation, modified, modified_by, docstatus) \
                     VALUES ($1, $1, $2, $3, $4, 'Administrator', $5::TIMESTAMP, $5::TIMESTAMP, 'Administrator', 0)",
                    role_table
                );
                sqlx::query(&insert_sql)
                    .bind(role_name)
                    .bind(module)
                    .bind(description)
                    .bind(is_custom.to_string())
                    .bind(&now)
                    .execute(pool)
                    .await
                    .ok();
            }
        }
        tracing::info!("Seeded default roles into {}", role_table);
    }

    // Seed System app in installed_apps if not present
    let existing_apps: serde_json::Value = sqlx::query_scalar(
        "SELECT value FROM \"__site_config\" WHERE key = 'installed_apps'",
    )
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
    .unwrap_or(serde_json::json!([]));

    let mut apps = existing_apps.as_array().cloned().unwrap_or_default();
    let has_system = apps.iter().any(|a| a.get("name").and_then(|v| v.as_str()) == Some("system"));

    if !has_system {
        apps.insert(0, serde_json::json!({
            "name": "system",
            "title": "System",
            "icon": "settings",
            "color": "#6366f1",
            "modules": ["Core"],
            "workspace": [
                { "label": "DocType", "route": "/app/DocType", "icon": "document" },
                { "label": "User", "route": "/app/User", "icon": "users" },
                { "label": "Role", "route": "/app/Role", "icon": "shield" },
                { "label": "Report Builder", "route": "/app/report-builder", "icon": "chart" },
                { "label": "Permissions", "route": "/app/role-permission-manager", "icon": "lock" }
            ]
        }));

        sqlx::query(
            "INSERT INTO \"__site_config\" (key, value, modified) VALUES ('installed_apps', $1, NOW()) \
             ON CONFLICT (key) DO UPDATE SET value = $1, modified = NOW()",
        )
        .bind(&serde_json::json!(apps))
        .execute(pool)
        .await
        .ok();

        tracing::info!("Seeded System app in installed_apps");
    }

    Ok(())
}

/// Auto-migrate database schema to match DocType metadata.
/// Creates tables if they don't exist, adds missing columns.
pub async fn migrate_doctype(pool: &PgPool, meta: &Meta) -> LoomResult<()> {
    let table = meta.table_name();

    if !table_exists(pool, &table).await? {
        create_table(pool, meta).await?;
        tracing::info!("Created table '{}'", table);
    } else {
        alter_table(pool, meta).await?;
    }

    Ok(())
}

/// Check if a table exists in the database.
async fn table_exists(pool: &PgPool, table_name: &str) -> LoomResult<bool> {
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT FROM information_schema.tables WHERE table_name = $1)",
    )
    .bind(table_name)
    .fetch_one(pool)
    .await?;

    Ok(exists)
}

/// Create a new table for a DocType.
async fn create_table(pool: &PgPool, meta: &Meta) -> LoomResult<()> {
    let table = meta.table_name();
    let mut columns = Vec::new();

    // Standard fields
    for (name, sql_type) in STANDARD_FIELDS {
        columns.push(format!("\"{}\" {}", name, sql_type));
    }

    // DocType-specific fields
    for field in meta.data_fields() {
        let sql_type = field.fieldtype.sql_type();
        if sql_type.is_empty() {
            continue;
        }

        let mut col_def = format!("\"{}\" {}", field.fieldname, sql_type);

        if field.unique {
            col_def.push_str(" UNIQUE");
        }

        if let Some(ref default) = field.default {
            match default {
                serde_json::Value::String(s) => {
                    col_def.push_str(&format!(" DEFAULT '{}'", s.replace('\'', "''")));
                }
                serde_json::Value::Number(n) => {
                    col_def.push_str(&format!(" DEFAULT {}", n));
                }
                serde_json::Value::Bool(b) => {
                    col_def.push_str(&format!(" DEFAULT {}", b));
                }
                _ => {}
            }
        }

        columns.push(col_def);
    }

    let sql = format!("CREATE TABLE \"{}\" ({})", table, columns.join(", "));
    tracing::debug!("CREATE TABLE SQL: {}", sql);

    sqlx::query(&sql)
        .execute(pool)
        .await
        .map_err(|e| LoomError::Internal(format!("Failed to create table '{}': {}", table, e)))?;

    // Create indexes on standard filter fields
    for field in meta.fields.iter().filter(|f| f.in_standard_filter) {
        let idx_name = format!("idx_{}_{}", table, field.fieldname);
        let idx_sql = format!(
            "CREATE INDEX IF NOT EXISTS \"{}\" ON \"{}\" (\"{}\")",
            idx_name, table, field.fieldname
        );
        sqlx::query(&idx_sql).execute(pool).await.ok();
    }

    Ok(())
}

/// Alter an existing table to add missing columns.
async fn alter_table(pool: &PgPool, meta: &Meta) -> LoomResult<()> {
    let table = meta.table_name();
    let existing_columns = get_existing_columns(pool, &table).await?;

    for field in meta.data_fields() {
        let sql_type = field.fieldtype.sql_type();
        if sql_type.is_empty() {
            continue;
        }

        if !existing_columns.contains(&field.fieldname) {
            let sql = format!(
                "ALTER TABLE \"{}\" ADD COLUMN \"{}\" {}",
                table, field.fieldname, sql_type
            );
            tracing::info!("Adding column '{}' to table '{}'", field.fieldname, table);

            sqlx::query(&sql).execute(pool).await.map_err(|e| {
                LoomError::Internal(format!(
                    "Failed to add column '{}' to '{}': {}",
                    field.fieldname, table, e
                ))
            })?;
        }
    }

    Ok(())
}

/// Get existing column names for a table.
async fn get_existing_columns(pool: &PgPool, table_name: &str) -> LoomResult<Vec<String>> {
    let columns: Vec<String> = sqlx::query_scalar(
        "SELECT column_name::TEXT FROM information_schema.columns WHERE table_name = $1",
    )
    .bind(table_name)
    .fetch_all(pool)
    .await?;

    Ok(columns)
}

/// Run migrations for all registered DocTypes.
pub async fn migrate_all(
    pool: &PgPool,
    registry: &crate::doctype::DocTypeRegistry,
) -> LoomResult<()> {
    let doctypes = registry.all_doctypes().await;
    for doctype_name in &doctypes {
        let meta = registry.get_meta(doctype_name).await?;
        migrate_doctype(pool, &meta).await?;
    }
    tracing::info!("Migrated {} DocTypes", doctypes.len());
    Ok(())
}
