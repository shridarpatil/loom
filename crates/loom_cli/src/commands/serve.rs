use std::sync::Arc;

use clap::Args;
use sqlx::postgres::PgPoolOptions;

use loom_api::{build_router, AppCache, AppState};
use loom_api::realtime::RealtimeHub;
use loom_core::doctype::{DocTypeRegistry, Meta, RhaiHookRunner};
use loom_core::script::{ScriptCache, create_engine};
use loom_core::db::migrate::{self, migrate_system_tables, migrate_doctype};
use loom_queue::{Worker, Scheduler};

#[derive(Debug, Args)]
pub struct ServeArgs {
    /// Port to listen on
    #[arg(short, long, default_value = "8000")]
    pub port: u16,

    /// Host to bind to
    #[arg(long, default_value = "0.0.0.0")]
    pub host: String,

    /// Database URL (overrides site config)
    #[arg(long, env = "DATABASE_URL")]
    pub db_url: Option<String>,

    /// Site name
    #[arg(long)]
    pub site: Option<String>,

    /// Path to apps directory
    #[arg(long, default_value = "apps")]
    pub apps_dir: String,

    /// Path to built frontend directory (default: frontend/dist)
    #[arg(long)]
    pub frontend_dir: Option<String>,
}

pub async fn run(args: ServeArgs) -> anyhow::Result<()> {
    let db_url = crate::site_config::resolve_db_url(args.db_url, args.site.as_deref());

    tracing::info!("Connecting to database...");
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&db_url)
        .await?;

    // Ensure system tables exist
    migrate_system_tables(&pool).await?;

    // Load DocType registry
    let registry = Arc::new(DocTypeRegistry::new());

    // Bootstrap the "DocType" DocType (meta-circular)
    let doctype_meta = Meta::doctype_meta();
    migrate_doctype(&pool, &doctype_meta).await?;
    registry.register(doctype_meta).await;
    tracing::info!("Bootstrapped 'DocType' meta");

    // Load from database first (existing state)
    let db_count = registry.load_from_database(&pool).await?;
    tracing::info!("Loaded {} DocTypes from database", db_count);

    // Load core DocTypes (overrides DB)
    let core_dir = std::path::Path::new("core_doctypes");
    if core_dir.exists() {
        let count = registry.load_from_directory(core_dir).await?;
        if count > 0 {
            tracing::info!("Loaded {} core DocTypes", count);
        }
    }

    // Load from apps directory (overrides DB — filesystem is authoritative)
    let apps_dir = std::path::Path::new(&args.apps_dir);
    if apps_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(apps_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let doctypes_dir = path.join("doctypes");
                    if doctypes_dir.exists() {
                        let count = registry.load_from_directory(&doctypes_dir).await?;
                        tracing::info!(
                            "Loaded {} DocTypes from app '{}'",
                            count,
                            path.file_name().unwrap_or_default().to_string_lossy()
                        );
                    }
                }
            }
        }
    }

    // Auto-migrate all loaded DocTypes
    for dt_name in registry.all_doctypes().await {
        if dt_name == "DocType" { continue; }
        if let Ok(meta) = registry.get_meta(&dt_name).await {
            migrate_doctype(&pool, &meta).await?;
        }
    }
    tracing::info!("All DocType tables migrated");

    // Seed Administrator into tabUser (if core User DocType was loaded)
    migrate::seed_admin_to_doctype_table(&pool).await?;

    // Set up hook runner and load scripts
    // The engine is built per-call inside run_hook with the user's context,
    // so we just pass a placeholder engine for the constructor.
    let engine = Arc::new(create_engine());
    let cache = ScriptCache::new();
    let hook_runner = Arc::new(RhaiHookRunner::new(engine, cache));

    // Load scripts from database (installed via install-app)
    let db_script_count = hook_runner.load_scripts_from_database(&pool).await?;
    if db_script_count > 0 {
        tracing::info!("Loaded {} scripts from database", db_script_count);
    }

    // Load scripts from apps directory (overrides DB versions)
    if apps_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(apps_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let doctypes_dir = path.join("doctypes");
                    if doctypes_dir.exists() {
                        let count = hook_runner.load_scripts_from_directory(&doctypes_dir).await?;
                        if count > 0 {
                            tracing::info!(
                                "Loaded {} scripts from app '{}'",
                                count,
                                path.file_name().unwrap_or_default().to_string_lossy()
                            );
                        }
                    }
                }
            }
        }
    }

    // Discover queues from app hooks.toml, always include "default"
    let mut queues: Vec<String> = vec!["default".to_string()];
    if apps_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(apps_dir) {
            for entry in entries.flatten() {
                let hooks_path = entry.path().join("hooks.toml");
                if hooks_path.exists() {
                    if let Ok(content) = std::fs::read_to_string(&hooks_path) {
                        if let Ok(hooks) = content.parse::<toml::Value>() {
                            if let Some(names) = hooks
                                .get("queues")
                                .and_then(|v| v.get("names"))
                                .and_then(|v| v.as_array())
                            {
                                for name in names {
                                    if let Some(s) = name.as_str() {
                                        if !queues.contains(&s.to_string()) {
                                            queues.push(s.to_string());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Start one worker per queue
    for queue_name in &queues {
        let worker = Worker::new(Arc::new(pool.clone()), queue_name);
        tokio::spawn(async move { worker.run().await });
    }
    tracing::info!("Started workers for queues: {}", queues.join(", "));

    // Start scheduler
    let mut scheduler = Scheduler::new(Arc::new(pool.clone()));
    scheduler.load_from_apps(apps_dir);
    tokio::spawn(async move { scheduler.run().await });

    // Start file watcher in developer mode
    if crate::site_config::load_site_config(args.site.as_deref())
        .and_then(|c| c.developer_mode)
        .unwrap_or(false)
    {
        crate::file_watcher::start_watcher(
            Arc::new(pool.clone()),
            registry.clone(),
            hook_runner.clone(),
        );
    }

    let state = AppState {
        pool,
        registry,
        hook_runner,
        realtime: RealtimeHub::new(),
        cache: AppCache::new(),
    };

    // Resolve frontend directory: explicit flag > frontend/dist default
    let frontend_dir = args.frontend_dir.or_else(|| {
        let default = std::path::Path::new("frontend/dist");
        if default.exists() {
            Some(default.to_string_lossy().to_string())
        } else {
            None
        }
    });
    if let Some(ref dir) = frontend_dir {
        tracing::info!("Serving frontend from {}", dir);
    }

    let app = build_router(state, frontend_dir);

    let addr = format!("{}:{}", args.host, args.port);
    tracing::info!("Loom server starting on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
