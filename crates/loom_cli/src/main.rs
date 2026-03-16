use clap::{Parser, Subcommand};

mod commands;
mod file_watcher;
mod site_config;

#[derive(Parser)]
#[command(name = "loom", about = "Loom — Metadata-driven application framework")]
struct Cli {
    /// Site name for site-specific commands
    #[arg(long, global = true)]
    site: Option<String>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Start the Loom web server
    Serve(commands::serve::ServeArgs),

    /// Create a new site
    NewSite(commands::new_site::NewSiteArgs),

    /// Scaffold a new app
    NewApp(commands::new_app::NewAppArgs),

    /// Install an app into a site
    InstallApp(commands::install_app::InstallAppArgs),

    /// Run database migrations
    Migrate(commands::migrate::MigrateArgs),

    /// Get an app from git, tarball URL, or local archive
    GetApp(commands::get_app::GetAppArgs),

    /// Open an interactive Rhai console
    Console(commands::console::ConsoleArgs),

    /// Start a background job worker
    Worker(commands::worker::WorkerArgs),

    /// Export document records as fixture JSON files
    ExportFixtures(commands::export_fixtures::ExportFixturesArgs),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,loom=debug".into()),
        )
        .init();

    let cli = Cli::parse();

    if let Some(ref site) = cli.site {
        tracing::debug!("Operating on site: {}", site);
    }

    match cli.command {
        Command::Serve(args) => commands::serve::run(args).await,
        Command::NewSite(args) => commands::new_site::run(args).await,
        Command::NewApp(args) => commands::new_app::run(args).await,
        Command::InstallApp(args) => commands::install_app::run(args).await,
        Command::GetApp(args) => commands::get_app::run(args).await,
        Command::Migrate(args) => commands::migrate::run(args).await,
        Command::Console(args) => commands::console::run(args).await,
        Command::Worker(args) => commands::worker::run(args).await,
        Command::ExportFixtures(args) => commands::export_fixtures::run(args).await,
    }
}
