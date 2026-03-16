use clap::Args;

use loom_core::db::migrate::{migrate_all, migrate_system_tables, migrate_doctype, seed_admin_to_doctype_table};
use loom_core::doctype::{DocTypeRegistry, Meta};

use crate::site_config::resolve_db_url;

#[derive(Debug, Args)]
pub struct MigrateArgs {
    /// Database URL (overrides site config)
    #[arg(long, env = "DATABASE_URL")]
    pub db_url: Option<String>,

    /// Site name
    #[arg(long)]
    pub site: Option<String>,

    /// Path to apps directory
    #[arg(long, default_value = "apps")]
    pub apps_dir: String,
}

pub async fn run(args: MigrateArgs) -> anyhow::Result<()> {
    let db_url = resolve_db_url(args.db_url, args.site.as_deref());

    tracing::info!("Running migrations...");

    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(2)
        .connect(&db_url)
        .await?;

    // Create system tables first
    migrate_system_tables(&pool).await?;
    tracing::info!("System tables ready");

    // Load DocTypes into registry
    let registry = DocTypeRegistry::new();

    // Bootstrap DocType
    let doctype_meta = Meta::doctype_meta();
    migrate_doctype(&pool, &doctype_meta).await?;
    registry.register(doctype_meta).await;

    // Load from database first (existing state)
    let db_count = registry.load_from_database(&pool).await?;
    if db_count > 0 {
        tracing::info!("Loaded {} DocTypes from database", db_count);
    }

    // Load core doctypes (overrides DB — core is authoritative)
    let core_dir = std::path::Path::new("core_doctypes");
    if core_dir.exists() {
        let count = registry.load_from_directory(core_dir).await?;
        if count > 0 {
            tracing::info!("Loaded {} core DocTypes", count);
        }
    }

    // Load from apps directory (overrides DB — filesystem is authoritative in developer mode)
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

    // Run DDL migrations
    migrate_all(&pool, &registry).await?;

    // Seed admin + default roles
    seed_admin_to_doctype_table(&pool).await?;

    // Sync to __doctype table
    registry.sync_to_database(&pool).await?;

    tracing::info!("Migrations complete");
    Ok(())
}
