use crate::site_config::resolve_db_url;
use clap::Args;

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

    let apps_dir = std::path::Path::new(&args.apps_dir);

    super::sync::run_full_migrate(&pool, apps_dir).await?;

    tracing::info!("Migrations complete");
    Ok(())
}
