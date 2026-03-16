use std::sync::Arc;

use clap::Args;
use sqlx::postgres::PgPoolOptions;

use loom_core::db::migrate::migrate_system_tables;
use loom_core::doctype::{DocTypeRegistry, Meta};
use loom_core::script::{create_engine, register_db_api, register_loom_api};

#[derive(Debug, Args)]
pub struct ConsoleArgs {
    /// Database URL (overrides site config)
    #[arg(long, env = "DATABASE_URL")]
    pub db_url: Option<String>,

    /// Site name
    #[arg(long)]
    pub site: Option<String>,
}

pub async fn run(args: ConsoleArgs) -> anyhow::Result<()> {
    let db_url = crate::site_config::resolve_db_url(args.db_url, args.site.as_deref());

    // Connect to DB
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await?;

    migrate_system_tables(&pool).await?;

    // Load registry
    let registry = DocTypeRegistry::new();
    registry.register(Meta::doctype_meta()).await;
    let db_count = registry.load_from_database(&pool).await?;

    // Set up Rhai engine with full API (running as Administrator)
    let registry = Arc::new(registry);
    let mut engine = create_engine();
    register_loom_api(&mut engine);
    register_db_api(
        &mut engine,
        Arc::new(pool),
        registry.clone(),
        "Administrator".to_string(),
        vec!["Administrator".to_string(), "All".to_string()],
    );

    println!("Loom Console — Rhai REPL");
    println!("Loaded {} DocType(s) from database", db_count);
    println!("Type expressions to evaluate. Ctrl-D to exit.\n");

    let mut rl = rustyline::DefaultEditor::new()?;
    let mut scope = rhai::Scope::new();

    // Add some useful variables to scope
    let dt_names = registry.all_doctypes().await;
    scope.push_constant("doctypes", dt_names.clone());

    loop {
        match rl.readline("loom> ") {
            Ok(line) => {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }

                let _ = rl.add_history_entry(line);

                match engine.eval_with_scope::<rhai::Dynamic>(&mut scope, line) {
                    Ok(result) => {
                        if !result.is_unit() {
                            println!("{}", result);
                        }
                    }
                    Err(e) => {
                        eprintln!("Error: {}", e);
                    }
                }
            }
            Err(rustyline::error::ReadlineError::Interrupted) => {
                // Ctrl-C: clear line, continue
                println!("(interrupted)");
                continue;
            }
            Err(rustyline::error::ReadlineError::Eof) => {
                // Ctrl-D: exit
                println!("Bye!");
                break;
            }
            Err(e) => {
                eprintln!("Error: {}", e);
                break;
            }
        }
    }

    Ok(())
}
