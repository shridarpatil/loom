use std::sync::Arc;
use std::time::Duration;

use sqlx::PgPool;

use crate::queue;

/// Maximum backoff interval when queue is idle (30 seconds).
const MAX_BACKOFF: Duration = Duration::from_secs(30);
/// Base poll interval.
const BASE_INTERVAL: Duration = Duration::from_secs(1);

/// Background job worker — uses LISTEN/NOTIFY to wake instantly when jobs arrive,
/// with exponential backoff as fallback for reliability.
pub struct Worker {
    pool: Arc<PgPool>,
    registry: Arc<loom_core::doctype::DocTypeRegistry>,
    queue_name: String,
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
        }
    }

    /// Start the worker loop. Runs until the task is cancelled.
    ///
    /// Uses PostgreSQL LISTEN/NOTIFY for instant wakeup when jobs are enqueued.
    /// Falls back to exponential backoff polling if LISTEN fails.
    pub async fn run(&self) {
        tracing::info!("Worker for queue '{}' started", self.queue_name);

        // Try to set up LISTEN on a dedicated connection
        let channel = format!("loom_queue_{}", self.queue_name);
        let mut listener = self.setup_listener(&channel).await;

        if listener.is_some() {
            tracing::info!(
                "[{}] Using LISTEN/NOTIFY for job notifications",
                self.queue_name
            );
        } else {
            tracing::warn!(
                "[{}] LISTEN/NOTIFY unavailable, falling back to polling",
                self.queue_name
            );
        }

        let mut backoff = BASE_INTERVAL;

        loop {
            // First, drain all available jobs before sleeping
            loop {
                match self.try_dequeue_and_execute().await {
                    Ok(true) => {
                        // Job found and processed — reset backoff, check for more
                        backoff = BASE_INTERVAL;
                        continue;
                    }
                    Ok(false) => {
                        // No jobs available — break to sleep
                        break;
                    }
                    Err(e) => {
                        tracing::error!("[{}] Dequeue error: {}", self.queue_name, e);
                        break;
                    }
                }
            }

            // Wait for notification or backoff timeout
            if let Some(ref mut listener) = listener {
                // Use LISTEN with a timeout — so we still periodically check even if
                // a notification is missed (belt and suspenders)
                let timeout = backoff.min(MAX_BACKOFF);
                match tokio::time::timeout(timeout, listener.recv()).await {
                    Ok(Ok(_notification)) => {
                        // Notified — job available, reset backoff
                        backoff = BASE_INTERVAL;
                    }
                    Ok(Err(e)) => {
                        // Listener error — fall back to polling
                        tracing::warn!(
                            "[{}] LISTEN error: {}, falling back to polling",
                            self.queue_name,
                            e
                        );
                        tokio::time::sleep(backoff).await;
                        backoff = (backoff * 2).min(MAX_BACKOFF);
                    }
                    Err(_timeout) => {
                        // Timeout — do a lightweight check before heavy dequeue
                        match queue::has_queued_jobs(&self.pool, &self.queue_name).await {
                            Ok(true) => {
                                backoff = BASE_INTERVAL;
                            }
                            Ok(false) => {
                                backoff = (backoff * 2).min(MAX_BACKOFF);
                            }
                            Err(_) => {
                                backoff = (backoff * 2).min(MAX_BACKOFF);
                            }
                        }
                    }
                }
            } else {
                // No listener — pure polling with exponential backoff
                tokio::time::sleep(backoff).await;

                // Lightweight check before expensive dequeue
                match queue::has_queued_jobs(&self.pool, &self.queue_name).await {
                    Ok(true) => {
                        backoff = BASE_INTERVAL;
                    }
                    Ok(false) => {
                        backoff = (backoff * 2).min(MAX_BACKOFF);
                    }
                    Err(_) => {
                        backoff = (backoff * 2).min(MAX_BACKOFF);
                    }
                }
            }
        }
    }

    /// Try to dequeue and execute one job. Returns Ok(true) if a job was processed.
    async fn try_dequeue_and_execute(&self) -> Result<bool, crate::error::QueueError> {
        match queue::dequeue(&self.pool, &self.queue_name).await? {
            Some(job) => {
                tracing::info!(
                    "[{}] Processing job {} — {} (priority {})",
                    self.queue_name,
                    job.id,
                    job.method,
                    job.priority
                );
                self.execute_job(&job).await;
                Ok(true)
            }
            None => Ok(false),
        }
    }

    /// Set up a PostgreSQL LISTEN on a dedicated connection.
    /// Returns None if the connection cannot be established.
    async fn setup_listener(&self, channel: &str) -> Option<sqlx::postgres::PgListener> {
        // PgListener gets its own connection (not from the pool)
        match sqlx::postgres::PgListener::connect_with(&*self.pool).await {
            Ok(mut listener) => {
                if listener.listen(channel).await.is_ok() {
                    Some(listener)
                } else {
                    tracing::warn!("Failed to LISTEN on channel '{}'", channel);
                    None
                }
            }
            Err(e) => {
                tracing::warn!("Failed to create PgListener: {}", e);
                None
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
