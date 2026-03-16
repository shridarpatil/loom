use clap::Args;

#[derive(Debug, Args)]
pub struct NewSiteArgs {
    /// Site name (e.g., mysite.localhost)
    pub site_name: String,

    /// Database name
    #[arg(long)]
    pub db_name: Option<String>,

    /// Admin password
    #[arg(long, default_value = "admin")]
    pub admin_password: String,

    /// Database URL
    #[arg(long, env = "DATABASE_URL")]
    pub db_url: Option<String>,
}

pub async fn run(args: NewSiteArgs) -> anyhow::Result<()> {
    let db_name = args.db_name.unwrap_or_else(|| {
        args.site_name.replace('.', "_").replace('-', "_")
    });

    tracing::info!(
        "Creating new site '{}' with database '{}'",
        args.site_name,
        db_name
    );

    // Create site directory
    let site_dir = format!("sites/{}", args.site_name);
    std::fs::create_dir_all(&site_dir)?;

    // Write site_config.json
    let config = serde_json::json!({
        "db_name": db_name,
        "db_type": "postgres",
        "admin_password": args.admin_password,
    });

    let config_path = format!("{}/site_config.json", site_dir);
    std::fs::write(&config_path, serde_json::to_string_pretty(&config)?)?;

    tracing::info!("Site config written to {}", config_path);

    // TODO: Create database, run initial migrations, create admin user

    tracing::info!("Site '{}' created successfully", args.site_name);
    Ok(())
}
