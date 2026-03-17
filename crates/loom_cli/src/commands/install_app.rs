use std::path::PathBuf;

use clap::Args;
use serde::Deserialize;
use sqlx::postgres::PgPoolOptions;

use loom_core::db::migrate::migrate_system_tables;

#[derive(Debug, Args)]
pub struct InstallAppArgs {
    /// App name or path to the app directory
    pub app: String,

    /// Database URL (overrides site config)
    #[arg(long, env = "DATABASE_URL")]
    pub db_url: Option<String>,

    /// Site name
    #[arg(long)]
    pub site: Option<String>,
}

/// App metadata from loom_app.toml
#[derive(Debug, Deserialize)]
struct AppConfig {
    app: AppMeta,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct AppMeta {
    name: String,
    #[serde(default)]
    version: Option<String>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    icon: Option<String>,
    #[serde(default)]
    color: Option<String>,
    #[serde(default)]
    modules: Vec<String>,
}

pub async fn run(args: InstallAppArgs) -> anyhow::Result<()> {
    // 1. Resolve and validate app path
    let app_path = resolve_app_path(&args.app)?;
    tracing::info!("Installing app from {:?}", app_path);

    let config_path = app_path.join("loom_app.toml");
    if !config_path.exists() {
        anyhow::bail!("No loom_app.toml found at {:?}. Is this a valid Loom app?", config_path);
    }
    let config_str = std::fs::read_to_string(&config_path)?;
    let config: AppConfig = toml::from_str(&config_str)?;
    tracing::info!(
        "App: {} v{}",
        config.app.name,
        config.app.version.as_deref().unwrap_or("0.0.0")
    );

    // 2. Connect to database
    let db_url = crate::site_config::resolve_db_url(args.db_url, args.site.as_deref());
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await?;

    migrate_system_tables(&pool).await?;

    // 3. Run full migrate (handles DocTypes, schemas, configs, scripts, client scripts)
    let apps_dir = std::path::Path::new("apps");
    super::sync::run_full_migrate(&pool, apps_dir).await?;

    // 4. Load fixtures (install-only — not run on every migrate)
    let fixture_count = super::sync::load_fixtures(&pool, apps_dir).await?;
    if fixture_count > 0 {
        tracing::info!("Loaded {} fixture record(s)", fixture_count);
    }

    tracing::info!("App '{}' installed successfully!", config.app.name);
    Ok(())
}

/// Resolve the app directory path from a name or path.
fn resolve_app_path(app: &str) -> anyhow::Result<PathBuf> {
    let direct = PathBuf::from(app);
    if direct.is_dir() {
        return Ok(direct);
    }
    let under_apps = PathBuf::from("apps").join(app);
    if under_apps.is_dir() {
        return Ok(under_apps);
    }
    anyhow::bail!(
        "App '{}' not found. Checked '{}' and '{}'",
        app,
        direct.display(),
        under_apps.display()
    )
}
