use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use sqlx::PgPool;

use super::meta::Meta;
use crate::error::{LoomError, LoomResult};

/// Global DocType registry — maps doctype names to their metadata.
/// Loaded from the database at startup, and updated when DocTypes are created/modified.
#[derive(Debug, Clone)]
pub struct DocTypeRegistry {
    inner: Arc<RwLock<HashMap<String, Meta>>>,
}

impl DocTypeRegistry {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a DocType in the registry.
    pub async fn register(&self, meta: Meta) {
        let name = meta.name.clone();
        self.inner.write().await.insert(name, meta);
    }

    /// Get the metadata for a DocType.
    pub async fn get_meta(&self, doctype: &str) -> LoomResult<Meta> {
        self.inner
            .read()
            .await
            .get(doctype)
            .cloned()
            .ok_or_else(|| LoomError::NotFound {
                doctype: "DocType".to_string(),
                name: doctype.to_string(),
            })
    }

    /// Get meta with site-level permission overrides from `__customization` applied.
    /// Use this instead of `get_meta` whenever permission checks are involved.
    pub async fn get_effective_meta(&self, doctype: &str, pool: &PgPool) -> LoomResult<Meta> {
        let mut meta = self.get_meta(doctype).await?;

        let row: Option<(serde_json::Value,)> =
            sqlx::query_as("SELECT overrides FROM \"__customization\" WHERE doctype = $1")
                .bind(doctype)
                .fetch_optional(pool)
                .await
                .unwrap_or(None);

        if let Some((overrides,)) = row {
            if let Some(perm_overrides) = overrides.get("__permissions") {
                if let Ok(override_perms) =
                    serde_json::from_value::<Vec<super::meta::DocPermMeta>>(perm_overrides.clone())
                {
                    meta.permissions =
                        super::meta::merge_permission_overrides(&meta.permissions, &override_perms);
                }
            }
        }

        Ok(meta)
    }

    /// Check if a DocType is registered.
    pub async fn exists(&self, doctype: &str) -> bool {
        self.inner.read().await.contains_key(doctype)
    }

    /// Get all registered DocType names.
    pub async fn all_doctypes(&self) -> Vec<String> {
        self.inner.read().await.keys().cloned().collect()
    }

    /// Remove a DocType from the registry.
    pub async fn unregister(&self, doctype: &str) {
        self.inner.write().await.remove(doctype);
    }

    /// Load all DocType definitions from JSON files in a directory (recursive).
    /// **Bug fix:** Now actually registers the loaded Meta objects into the registry.
    pub async fn load_from_directory(&self, dir: &std::path::Path) -> LoomResult<usize> {
        if !dir.exists() {
            return Ok(0);
        }
        let metas = collect_metas_recursive(dir)?;
        let count = metas.len();
        for meta in metas {
            tracing::info!("Registered DocType '{}' from filesystem", meta.name);
            self.register(meta).await;
        }
        Ok(count)
    }

    /// Load DocTypes from the `__doctype` system table in the database.
    pub async fn load_from_database(&self, pool: &PgPool) -> LoomResult<usize> {
        // Check if the __doctype table exists first
        let table_exists: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT FROM information_schema.tables WHERE table_name = '__doctype')",
        )
        .fetch_one(pool)
        .await
        .unwrap_or(false);

        if !table_exists {
            tracing::debug!("__doctype table does not exist yet, skipping DB load");
            return Ok(0);
        }

        let rows: Vec<(String, serde_json::Value)> =
            sqlx::query_as("SELECT name, meta FROM \"__doctype\"")
                .fetch_all(pool)
                .await?;

        let mut count = 0;
        for (name, meta_json) in rows {
            match serde_json::from_value::<Meta>(meta_json) {
                Ok(meta) => {
                    tracing::debug!("Loaded DocType '{}' from database", name);
                    self.register(meta).await;
                    count += 1;
                }
                Err(e) => {
                    tracing::warn!("Failed to deserialize DocType '{}' from DB: {}", name, e);
                }
            }
        }

        Ok(count)
    }

    /// Sync all registered DocTypes to the `__doctype` system table.
    pub async fn sync_to_database(&self, pool: &PgPool) -> LoomResult<usize> {
        let doctypes = self.inner.read().await.clone();
        let mut count = 0;

        for (name, meta) in &doctypes {
            let meta_json = serde_json::to_value(meta).map_err(|e| {
                LoomError::Internal(format!("Failed to serialize DocType '{}': {}", name, e))
            })?;

            sqlx::query(
                "INSERT INTO \"__doctype\" (name, module, meta, modified) \
                 VALUES ($1, $2, $3, NOW()) \
                 ON CONFLICT (name) DO UPDATE SET meta = $3, module = $2, modified = NOW()",
            )
            .bind(name)
            .bind(&meta.module)
            .bind(&meta_json)
            .execute(pool)
            .await?;

            count += 1;
        }

        tracing::info!("Synced {} DocTypes to database", count);
        Ok(count)
    }
}

/// Recursively collect Meta objects from JSON files (sync filesystem walk).
fn collect_metas_recursive(dir: &std::path::Path) -> LoomResult<Vec<Meta>> {
    let mut metas = Vec::new();
    let mut stack = vec![dir.to_path_buf()];

    while let Some(current) = stack.pop() {
        let entries =
            std::fs::read_dir(&current).map_err(|e| LoomError::Internal(e.to_string()))?;

        for entry in entries {
            let entry = entry.map_err(|e| LoomError::Internal(e.to_string()))?;
            let path = entry.path();

            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|ext| ext == "json")
                && !path.to_string_lossy().ends_with(".customize.json")
            {
                match Meta::from_json_file(&path) {
                    Ok(meta) => {
                        metas.push(meta);
                    }
                    Err(e) => {
                        tracing::warn!("Failed to load DocType from {:?}: {}", path, e);
                    }
                }
            }
        }
    }

    Ok(metas)
}

impl Default for DocTypeRegistry {
    fn default() -> Self {
        Self::new()
    }
}
