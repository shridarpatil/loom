use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

use crate::error::{LoomError, LoomResult};

/// Multi-tenant connection pool manager.
/// Each site gets its own database connection pool, lazily initialized.
#[derive(Debug, Clone)]
pub struct PoolManager {
    pools: Arc<RwLock<HashMap<String, PgPool>>>,
    max_connections: u32,
}

impl PoolManager {
    pub fn new(max_connections: u32) -> Self {
        Self {
            pools: Arc::new(RwLock::new(HashMap::new())),
            max_connections,
        }
    }

    /// Get or create a connection pool for a given site.
    pub async fn get_pool(&self, site: &str, database_url: &str) -> LoomResult<PgPool> {
        // Fast path: pool already exists
        {
            let pools = self.pools.read().await;
            if let Some(pool) = pools.get(site) {
                return Ok(pool.clone());
            }
        }

        // Slow path: create pool
        let pool = PgPoolOptions::new()
            .max_connections(self.max_connections)
            .connect(database_url)
            .await
            .map_err(|e| {
                LoomError::Internal(format!(
                    "Failed to connect to DB for site '{}': {}",
                    site, e
                ))
            })?;

        tracing::info!("Created connection pool for site '{}'", site);

        self.pools
            .write()
            .await
            .insert(site.to_string(), pool.clone());
        Ok(pool)
    }

    /// Close and remove the pool for a site.
    pub async fn close(&self, site: &str) {
        if let Some(pool) = self.pools.write().await.remove(site) {
            pool.close().await;
            tracing::info!("Closed connection pool for site '{}'", site);
        }
    }

    /// Close all pools.
    pub async fn close_all(&self) {
        let mut pools = self.pools.write().await;
        for (site, pool) in pools.drain() {
            pool.close().await;
            tracing::info!("Closed connection pool for site '{}'", site);
        }
    }
}

impl Default for PoolManager {
    fn default() -> Self {
        Self::new(10)
    }
}
