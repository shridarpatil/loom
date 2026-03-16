use std::sync::Arc;

use clap::Args;
use sqlx::postgres::PgPoolOptions;

use loom_core::db::migrate::migrate_system_tables;
use loom_queue::Worker;

#[derive(Debug, Args)]
pub struct WorkerArgs {
    /// Queue name to process
    #[arg(short, long, default_value = "default")]
    pub queue: String,

    /// Number of concurrent workers for this queue
    #[arg(short, long, default_value = "1")]
    pub concurrency: u32,

    /// Database URL (overrides site config)
    #[arg(long, env = "DATABASE_URL")]
    pub db_url: Option<String>,

    /// Site name
    #[arg(long)]
    pub site: Option<String>,
}

pub async fn run(args: WorkerArgs) -> anyhow::Result<()> {
    let db_url = crate::site_config::resolve_db_url(args.db_url, args.site.as_deref());

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await?;

    migrate_system_tables(&pool).await?;

    // Load shared registry
    let registry = std::sync::Arc::new(loom_core::doctype::DocTypeRegistry::new());
    registry.register(loom_core::doctype::Meta::doctype_meta()).await;
    let _ = registry.load_from_database(&pool).await;

    let pool = Arc::new(pool);

    tracing::info!(
        "Starting {} worker(s) for queue '{}'",
        args.concurrency,
        args.queue
    );

    let mut handles = Vec::new();
    for i in 0..args.concurrency {
        let worker = Worker::new(pool.clone(), registry.clone(), &args.queue);
        let handle = tokio::spawn(async move {
            tracing::info!("Worker {}/{} started", i + 1, args.concurrency);
            worker.run().await;
        });
        handles.push(handle);
    }

    // Wait forever (until Ctrl-C)
    futures_util::future::join_all(handles).await;

    Ok(())
}
