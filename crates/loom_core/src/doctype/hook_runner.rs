use std::collections::HashMap;
use std::sync::Arc;

use rhai::{Dynamic, Engine, Scope};
use serde_json::Value;
use sqlx::PgPool;
use tokio::sync::RwLock;

use crate::context::RequestContext;
use crate::doctype::DocTypeRegistry;
use crate::error::{LoomError, LoomResult};
use crate::script::ScriptCache;

use super::controller::HookRunner;
use super::hooks::HookEvent;

/// Rhai-based hook runner that executes DocType scripts during document lifecycle events.
pub struct RhaiHookRunner {
    cache: ScriptCache,
    /// Map of doctype name → script source code
    scripts: Arc<RwLock<HashMap<String, String>>>,
}

impl std::fmt::Debug for RhaiHookRunner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RhaiHookRunner")
            .field("scripts_count", &"<async>")
            .finish()
    }
}

impl RhaiHookRunner {
    pub fn new(_engine: Arc<Engine>, cache: ScriptCache) -> Self {
        Self {
            cache,
            scripts: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Build a fresh Rhai engine with the full loom API registered for the given context.
    fn build_engine(
        pool: &PgPool,
        registry: &Arc<DocTypeRegistry>,
        user: &str,
        roles: &[String],
    ) -> Engine {
        let mut engine = crate::script::create_engine();
        crate::script::api::register_loom_api(&mut engine);
        crate::script::register_db_api(
            &mut engine,
            Arc::new(pool.clone()),
            registry.clone(),
            user.to_string(),
            roles.to_vec(),
        );
        engine
    }

    /// Register a script for a DocType.
    pub async fn load_script(&self, doctype: &str, source: String) {
        tracing::debug!("Loading script for DocType '{}'", doctype);
        self.scripts
            .write()
            .await
            .insert(doctype.to_string(), source);
        // Invalidate cached AST so it gets recompiled
        self.cache.invalidate(doctype).await;
    }

    /// Load Rhai scripts from the `__script` table in the database.
    pub async fn load_scripts_from_database(&self, pool: &sqlx::PgPool) -> LoomResult<usize> {
        let table_exists: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT FROM information_schema.tables WHERE table_name = '__script')",
        )
        .fetch_one(pool)
        .await
        .unwrap_or(false);

        if !table_exists {
            return Ok(0);
        }

        let rows: Vec<(String, String)> =
            sqlx::query_as("SELECT doctype, script FROM \"__script\"")
                .fetch_all(pool)
                .await?;

        let count = rows.len();
        for (doctype, script) in rows {
            tracing::debug!("Loaded script for '{}' from database", doctype);
            self.load_script(&doctype, script).await;
        }

        Ok(count)
    }

    /// Load all `.rhai` scripts from a directory tree.
    /// Expects the convention: `{dir}/{doctype_dir}/{doctype}.rhai`
    pub async fn load_scripts_from_directory(&self, dir: &std::path::Path) -> LoomResult<usize> {
        if !dir.exists() {
            return Ok(0);
        }

        // Collect scripts from filesystem (sync), then load them (async)
        let found = collect_rhai_scripts(dir)?;
        let count = found.len();

        for (doctype, source) in found {
            self.load_script(&doctype, source).await;
        }

        Ok(count)
    }
}

/// Recursively collect all `.rhai` scripts from a directory (sync filesystem walk).
fn collect_rhai_scripts(dir: &std::path::Path) -> LoomResult<Vec<(String, String)>> {
    let mut results = Vec::new();
    let mut stack = vec![dir.to_path_buf()];

    while let Some(current) = stack.pop() {
        let entries =
            std::fs::read_dir(&current).map_err(|e| LoomError::Internal(e.to_string()))?;

        for entry in entries {
            let entry = entry.map_err(|e| LoomError::Internal(e.to_string()))?;
            let path = entry.path();

            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|ext| ext == "rhai") {
                let source = std::fs::read_to_string(&path)
                    .map_err(|e| LoomError::Internal(e.to_string()))?;

                let doctype = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_string();

                if !doctype.is_empty() {
                    tracing::info!("Found script for '{}' at {:?}", doctype, path);
                    results.push((doctype, source));
                }
            }
        }
    }

    Ok(results)
}

#[async_trait::async_trait]
impl HookRunner for RhaiHookRunner {
    async fn run_hook(
        &self,
        event: HookEvent,
        doctype: &str,
        doc: &mut Value,
        ctx: &RequestContext,
    ) -> LoomResult<()> {
        // Check if we have a script for this doctype
        let script_source = {
            let scripts = self.scripts.read().await;
            // Try exact match first, then lowercase
            scripts
                .get(doctype)
                .or_else(|| scripts.get(&doctype.to_lowercase().replace(' ', "_")))
                .cloned()
        };

        let source = match script_source {
            Some(s) => s,
            None => return Ok(()), // No script for this doctype, nothing to do
        };

        let fn_name = event.as_str();

        // Build a per-call engine with full loom API for this user's context
        let engine = Self::build_engine(ctx.pool(), &ctx.registry, &ctx.user, &ctx.roles);

        // Compile or get cached AST
        let ast = self
            .cache
            .get_or_compile(doctype, &engine, &source)
            .await
            .map_err(|e| LoomError::Script(e))?;

        // Check if the function exists in the AST
        let has_fn = ast.iter_functions().any(|f| f.name == fn_name);

        if !has_fn {
            return Ok(()); // Function not defined, skip
        }

        // Build scope with context variables
        let mut scope = Scope::new();
        scope.push("__loom_user", ctx.user.clone());
        scope.push("__loom_roles", ctx.roles.clone());

        // Convert doc to Dynamic (clone the Value since to_dynamic takes ownership)
        let doc_dynamic = rhai::serde::to_dynamic(&*doc)
            .map_err(|e| LoomError::Script(format!("Failed to convert doc to Dynamic: {}", e)))?;

        // Call the function with just the doc parameter.
        // All loom APIs are registered as global functions on the engine.
        let result = tokio::task::block_in_place(move || {
            engine.call_fn::<Dynamic>(&mut scope, &ast, fn_name, (doc_dynamic,))
        });

        match result {
            Ok(returned) => {
                // If the hook returned a map, merge it back into the doc
                if let Ok(updated_val) = rhai::serde::from_dynamic::<Value>(&returned) {
                    if updated_val.is_object() {
                        if let Some(update_obj) = updated_val.as_object() {
                            if let Some(doc_obj) = doc.as_object_mut() {
                                for (k, v) in update_obj {
                                    doc_obj.insert(k.clone(), v.clone());
                                }
                            }
                        }
                    }
                }
                Ok(())
            }
            Err(e) => {
                let err_str = e.to_string();
                // Check if this is a validation error thrown via loom.throw()
                if err_str.contains("ErrorRuntime") || err_str.contains("Runtime error") {
                    Err(LoomError::Validation(err_str))
                } else {
                    Err(LoomError::Script(format!(
                        "Error in {}.{}(): {}",
                        doctype, fn_name, err_str
                    )))
                }
            }
        }
    }
}
