use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::RwLock;

/// A simple in-memory TTL cache.
#[derive(Debug, Clone)]
pub struct TtlCache<V: Clone> {
    inner: Arc<RwLock<HashMap<String, (V, Instant)>>>,
    ttl: Duration,
}

impl<V: Clone + Send + Sync> TtlCache<V> {
    pub fn new(ttl_secs: u64) -> Self {
        Self {
            inner: Arc::new(RwLock::new(HashMap::new())),
            ttl: Duration::from_secs(ttl_secs),
        }
    }

    pub async fn get(&self, key: &str) -> Option<V> {
        let map = self.inner.read().await;
        if let Some((value, inserted)) = map.get(key) {
            if inserted.elapsed() < self.ttl {
                return Some(value.clone());
            }
        }
        None
    }

    pub async fn set(&self, key: String, value: V) {
        self.inner
            .write()
            .await
            .insert(key, (value, Instant::now()));
    }

    pub async fn invalidate(&self, key: &str) {
        self.inner.write().await.remove(key);
    }

    pub async fn clear(&self) {
        self.inner.write().await.clear();
    }
}

/// Shared caches for the application.
#[derive(Debug, Clone)]
pub struct AppCache {
    /// Session cache: sid → (user_email, roles)
    /// TTL: 5 minutes — avoids 2 DB queries per request
    pub sessions: TtlCache<(String, Vec<String>)>,

    /// Customization overrides cache: doctype → overrides JSON
    /// TTL: 5 minutes — avoids 1 DB query per get_effective_meta call
    pub customizations: TtlCache<Option<serde_json::Value>>,

    /// User permissions cache: user_email → Vec<UserPermission-like data>
    /// TTL: 5 minutes — avoids 1 DB query per get_list call
    pub user_permissions: TtlCache<Vec<loom_core::perms::user_perm::UserPermission>>,
}

/// Cached effective meta (base meta + permission overrides merged).
pub type CachedMeta = loom_core::doctype::Meta;

impl AppCache {
    pub fn new() -> Self {
        Self {
            sessions: TtlCache::new(3600),         // 1 hour — invalidated on logout
            customizations: TtlCache::new(86400),  // 24 hours — invalidated on save
            user_permissions: TtlCache::new(3600), // 1 hour — invalidated on role change
        }
    }

    /// Get effective meta with caching on the customization layer.
    /// Caches the __customization query result, not the full merged meta
    /// (since the merge depends on the base meta which may change).
    pub async fn get_effective_meta(
        &self,
        registry: &loom_core::doctype::DocTypeRegistry,
        pool: &sqlx::PgPool,
        doctype: &str,
    ) -> loom_core::LoomResult<CachedMeta> {
        let mut meta = registry.get_meta(doctype).await?;

        // Check customization cache
        let overrides = if let Some(cached) = self.customizations.get(doctype).await {
            cached
        } else {
            // Cache miss — query DB
            let row: Option<(serde_json::Value,)> =
                sqlx::query_as("SELECT overrides FROM \"__customization\" WHERE doctype = $1")
                    .bind(doctype)
                    .fetch_optional(pool)
                    .await
                    .unwrap_or(None);

            let val = row.map(|(v,)| v);
            self.customizations
                .set(doctype.to_string(), val.clone())
                .await;
            val
        };

        // Apply permission overrides (merge logic)
        if let Some(overrides) = overrides {
            if let Some(perm_overrides) = overrides.get("__permissions") {
                if let Ok(override_perms) = serde_json::from_value::<
                    Vec<loom_core::doctype::DocPermMeta>,
                >(perm_overrides.clone())
                {
                    meta.permissions = loom_core::doctype::merge_permission_overrides(
                        &meta.permissions,
                        &override_perms,
                    );
                }
            }
        }

        Ok(meta)
    }

    /// Get user permissions with caching.
    pub async fn get_user_permissions_cached(
        &self,
        pool: &sqlx::PgPool,
        user: &str,
    ) -> loom_core::LoomResult<Vec<loom_core::perms::user_perm::UserPermission>> {
        if let Some(cached) = self.user_permissions.get(user).await {
            return Ok(cached);
        }

        let perms = loom_core::perms::get_user_permissions(pool, user).await?;
        self.user_permissions
            .set(user.to_string(), perms.clone())
            .await;
        Ok(perms)
    }

    /// Invalidate customization cache for a doctype (call after saving customizations).
    pub async fn invalidate_customization(&self, doctype: &str) {
        self.customizations.invalidate(doctype).await;
    }

    /// Invalidate session cache (call on logout).
    pub async fn invalidate_session(&self, sid: &str) {
        self.sessions.invalidate(sid).await;
    }
}
