use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use notify::{Event, EventKind, RecursiveMode, Watcher};
use sqlx::PgPool;
use tokio::sync::mpsc;

use loom_core::db::migrate::migrate_doctype;
use loom_core::doctype::{DocTypeRegistry, Meta, RhaiHookRunner};

/// Start watching `apps/` and `core_doctypes/` for file changes.
/// On `.json` change: reload DocType + migrate table.
/// On `.rhai` change: reload script into hook runner.
pub fn start_watcher(
    pool: Arc<PgPool>,
    registry: Arc<DocTypeRegistry>,
    hook_runner: Arc<RhaiHookRunner>,
) {
    let (tx, mut rx) = mpsc::channel::<Event>(100);

    // Spawn the blocking file watcher thread
    std::thread::spawn(move || {
        let rt_tx = tx;
        let mut watcher =
            match notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
                if let Ok(event) = res {
                    let _ = rt_tx.blocking_send(event);
                }
            }) {
                Ok(w) => w,
                Err(e) => {
                    tracing::error!("Failed to create file watcher: {}", e);
                    return;
                }
            };

        // Watch apps/ and core_doctypes/
        for dir in &["apps", "core_doctypes"] {
            let path = Path::new(dir);
            if path.exists() {
                if let Err(e) = watcher.watch(path, RecursiveMode::Recursive) {
                    tracing::warn!("Failed to watch {}: {}", dir, e);
                }
            }
        }

        tracing::info!("File watcher started (apps/, core_doctypes/)");

        // Block this thread forever — watcher needs to stay alive
        std::thread::park();
    });

    // Spawn async handler for events
    tokio::spawn(async move {
        // Debounce: collect events for 500ms before processing
        let mut pending_json: Vec<std::path::PathBuf> = Vec::new();
        let mut pending_rhai: Vec<std::path::PathBuf> = Vec::new();
        let mut pending_client_js: Vec<std::path::PathBuf> = Vec::new();

        loop {
            tokio::select! {
                Some(event) = rx.recv() => {
                    match event.kind {
                        EventKind::Create(_) | EventKind::Modify(_) => {
                            for path in &event.paths {
                                let path_str = path.to_string_lossy();
                                if path_str.ends_with(".client.js") {
                                    pending_client_js.push(path.clone());
                                } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                                    match ext {
                                        "json" if !path_str.contains(".customize.") => {
                                            pending_json.push(path.clone());
                                        }
                                        "rhai" => {
                                            pending_rhai.push(path.clone());
                                        }
                                        _ => {}
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
                _ = tokio::time::sleep(Duration::from_millis(500)), if !pending_json.is_empty() || !pending_rhai.is_empty() || !pending_client_js.is_empty() => {
                    // Process pending changes
                    for path in pending_json.drain(..) {
                        reload_doctype(&pool, &registry, &path).await;
                    }
                    for path in pending_rhai.drain(..) {
                        reload_script(&hook_runner, &path).await;
                    }
                    for path in pending_client_js.drain(..) {
                        reload_client_script(&pool, &path).await;
                    }
                }
            }
        }
    });
}

async fn reload_doctype(pool: &PgPool, registry: &DocTypeRegistry, path: &Path) {
    let meta = match Meta::from_json_file(path) {
        Ok(m) => m,
        Err(e) => {
            tracing::warn!("Failed to parse DocType from {:?}: {}", path, e);
            return;
        }
    };

    let name = meta.name.clone();

    // Migrate table
    if let Err(e) = migrate_doctype(pool, &meta).await {
        tracing::warn!("Failed to migrate DocType '{}': {}", name, e);
    }

    // Update registry
    registry.register(meta).await;

    // Sync to __doctype table
    if let Ok(meta) = registry.get_meta(&name).await {
        let meta_json = serde_json::to_value(&meta).unwrap_or_default();
        let _ = sqlx::query(
            "INSERT INTO \"__doctype\" (name, module, meta, modified) \
             VALUES ($1, $2, $3, NOW()) \
             ON CONFLICT (name) DO UPDATE SET meta = $3, module = $2, modified = NOW()",
        )
        .bind(&name)
        .bind(&meta.module)
        .bind(&meta_json)
        .execute(pool)
        .await;
    }

    tracing::info!("[hot-reload] Reloaded DocType '{}' from {:?}", name, path);
}

async fn reload_script(hook_runner: &RhaiHookRunner, path: &Path) {
    let source = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!("Failed to read script {:?}: {}", path, e);
            return;
        }
    };

    let doctype = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string();

    if doctype.is_empty() {
        return;
    }

    hook_runner.load_script(&doctype, source).await;
    tracing::info!(
        "[hot-reload] Reloaded script for '{}' from {:?}",
        doctype,
        path
    );
}

async fn reload_client_script(pool: &PgPool, path: &Path) {
    let source = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!("Failed to read client script {:?}: {}", path, e);
            return;
        }
    };

    // Extract DocType name from path: .../doctypes/todo/todo.client.js → "todo"
    let filename = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
    let slug = filename
        .strip_suffix(".client.js")
        .unwrap_or("")
        .to_string();
    if slug.is_empty() {
        return;
    }

    // Find the DocType name — try the slug as-is, or look up from the parent dir JSON
    let doctype_name = if let Some(parent) = path.parent() {
        let json_path = parent.join(format!("{}.json", slug));
        if json_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&json_path) {
                if let Ok(meta) = serde_json::from_str::<serde_json::Value>(&content) {
                    meta.get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or(&slug)
                        .to_string()
                } else {
                    slug.clone()
                }
            } else {
                slug.clone()
            }
        } else {
            slug.clone()
        }
    } else {
        slug.clone()
    };

    // Upsert into __customization
    let _ = sqlx::query(
        "INSERT INTO \"__customization\" (doctype, overrides, client_script, modified) \
         VALUES ($1, '{}', $2, NOW()) \
         ON CONFLICT (doctype) DO UPDATE SET client_script = $2, modified = NOW()",
    )
    .bind(&doctype_name)
    .bind(&source)
    .execute(pool)
    .await;

    tracing::info!(
        "[hot-reload] Reloaded client script for '{}' from {:?}",
        doctype_name,
        path
    );
}
