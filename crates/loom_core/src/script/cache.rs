use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use rhai::AST;

/// Cache for compiled Rhai ASTs, keyed by script path or identifier.
#[derive(Debug, Clone)]
pub struct ScriptCache {
    cache: Arc<RwLock<HashMap<String, AST>>>,
}

impl ScriptCache {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get a cached AST, or compile and cache it.
    pub async fn get_or_compile(
        &self,
        key: &str,
        engine: &rhai::Engine,
        source: &str,
    ) -> Result<AST, String> {
        // Fast path
        {
            let cache = self.cache.read().await;
            if let Some(ast) = cache.get(key) {
                return Ok(ast.clone());
            }
        }

        // Compile and cache
        let ast = engine
            .compile(source)
            .map_err(|e| format!("Script compile error for '{}': {}", key, e))?;

        self.cache
            .write()
            .await
            .insert(key.to_string(), ast.clone());

        Ok(ast)
    }

    /// Invalidate a cached script (e.g., on hot-reload).
    pub async fn invalidate(&self, key: &str) {
        self.cache.write().await.remove(key);
    }

    /// Clear all cached scripts.
    pub async fn clear(&self) {
        self.cache.write().await.clear();
    }
}

impl Default for ScriptCache {
    fn default() -> Self {
        Self::new()
    }
}
