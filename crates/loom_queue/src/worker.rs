use std::sync::Arc;
use std::time::Duration;

use sqlx::PgPool;

use crate::queue;

/// Background job worker — polls a named queue and executes Rhai scripts.
pub struct Worker {
    pool: Arc<PgPool>,
    registry: Arc<loom_core::doctype::DocTypeRegistry>,
    queue_name: String,
    poll_interval: Duration,
}

impl Worker {
    pub fn new(
        pool: Arc<PgPool>,
        registry: Arc<loom_core::doctype::DocTypeRegistry>,
        queue_name: &str,
    ) -> Self {
        Self {
            pool,
            registry,
            queue_name: queue_name.to_string(),
            poll_interval: Duration::from_secs(1),
        }
    }

    /// Start the worker loop. Runs until the task is cancelled.
    pub async fn run(&self) {
        tracing::info!("Worker for queue '{}' started", self.queue_name);

        loop {
            match queue::dequeue(&self.pool, &self.queue_name).await {
                Ok(Some(job)) => {
                    tracing::info!(
                        "[{}] Processing job {} — {} (priority {})",
                        self.queue_name,
                        job.id,
                        job.method,
                        job.priority
                    );
                    self.execute_job(&job).await;
                }
                Ok(None) => {
                    tokio::time::sleep(self.poll_interval).await;
                }
                Err(e) => {
                    tracing::error!("[{}] Dequeue error: {}", self.queue_name, e);
                    tokio::time::sleep(self.poll_interval).await;
                }
            }
        }
    }

    async fn execute_job(&self, job: &queue::Job) {
        let (app_name, method_name) = match job.method.split_once('.') {
            Some((a, m)) => (a, m),
            None => {
                let err = format!("Invalid method path: {}", job.method);
                tracing::error!("{}", err);
                let _ = queue::mark_failed(&self.pool, job.id, &err, false).await;
                return;
            }
        };

        let script_path = format!("apps/{}/api/{}.rhai", app_name, method_name);
        let source = match std::fs::read_to_string(&script_path) {
            Ok(s) => s,
            Err(_) => {
                let err = format!("Script not found: {}", script_path);
                tracing::error!("{}", err);
                let _ = queue::mark_failed(&self.pool, job.id, &err, false).await;
                return;
            }
        };

        // Use shared registry instead of creating a new one per job
        let mut engine = loom_core::script::create_engine();
        loom_core::script::api::register_loom_api(&mut engine);
        loom_core::script::register_db_api(
            &mut engine,
            self.pool.clone(),
            self.registry.clone(),
            "Administrator".to_string(),
            vec!["Administrator".to_string(), "All".to_string()],
        );

        let ast = match engine.compile(&source) {
            Ok(a) => a,
            Err(e) => {
                let err = format!("Script compile error: {}", e);
                tracing::error!("{}", err);
                let _ = queue::mark_failed(&self.pool, job.id, &err, false).await;
                return;
            }
        };

        let has_main = ast.iter_functions().any(|f| f.name == "main");
        if !has_main {
            let err = "Script has no main() function";
            tracing::error!("{}", err);
            let _ = queue::mark_failed(&self.pool, job.id, err, false).await;
            return;
        }

        let params = rhai::serde::to_dynamic(&job.args).unwrap_or(rhai::Dynamic::UNIT);
        let loom_map = rhai::Dynamic::from(rhai::Map::new());
        let mut scope = rhai::Scope::new();

        let result: Result<rhai::Dynamic, Box<rhai::EvalAltResult>> =
            tokio::task::block_in_place(|| {
                engine.call_fn::<rhai::Dynamic>(&mut scope, &ast, "main", (params, loom_map))
            });

        match result {
            Ok(_) => {
                tracing::info!("[{}] Job {} completed", self.queue_name, job.id);
                let _ = queue::mark_completed(&self.pool, job.id).await;
            }
            Err(e) => {
                let err = e.to_string();
                let can_retry = job.attempts < job.max_retries;
                tracing::error!(
                    "[{}] Job {} failed (attempt {}/{}): {}",
                    self.queue_name,
                    job.id,
                    job.attempts,
                    job.max_retries,
                    err
                );
                let _ = queue::mark_failed(&self.pool, job.id, &err, can_retry).await;
            }
        }
    }
}
