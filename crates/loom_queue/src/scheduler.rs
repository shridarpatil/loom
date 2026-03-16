use std::sync::Arc;
use std::time::Duration;

use chrono::Timelike;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::queue;

/// A scheduled task definition (from app hooks.toml).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledTask {
    pub cron: String,
    pub method: String,
}

/// Scheduler — runs scheduled tasks based on simple interval matching.
/// Checks every minute which tasks should run.
pub struct Scheduler {
    pool: Arc<PgPool>,
    tasks: Vec<ScheduledTask>,
}

impl Scheduler {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self {
            pool,
            tasks: Vec::new(),
        }
    }

    /// Load scheduled tasks from all apps' hooks.toml files.
    pub fn load_from_apps(&mut self, apps_dir: &std::path::Path) {
        if !apps_dir.exists() {
            return;
        }

        let entries = match std::fs::read_dir(apps_dir) {
            Ok(e) => e,
            Err(_) => return,
        };

        for entry in entries.flatten() {
            let hooks_path = entry.path().join("hooks.toml");
            if !hooks_path.exists() {
                continue;
            }

            let content = match std::fs::read_to_string(&hooks_path) {
                Ok(c) => c,
                Err(_) => continue,
            };

            let hooks: toml::Value = match content.parse::<toml::Value>() {
                Ok(v) => v,
                Err(_) => continue,
            };

            if let Some(schedulers) = hooks.get("scheduler").and_then(|v: &toml::Value| v.as_array()) {
                for entry in schedulers {
                    if let Some(table) = entry.as_table() {
                        let cron = table.get("cron").and_then(|v: &toml::Value| v.as_str()).unwrap_or("").to_string();
                        let method = table.get("method").and_then(|v: &toml::Value| v.as_str()).unwrap_or("").to_string();
                        if !cron.is_empty() && !method.is_empty() {
                            tracing::info!("Registered scheduled task: {} ({})", method, cron);
                            self.tasks.push(ScheduledTask { cron, method });
                        }
                    }
                }
            }
        }
    }

    /// Run the scheduler loop. Checks every minute for tasks to execute.
    pub async fn run(&self) {
        if self.tasks.is_empty() {
            tracing::info!("No scheduled tasks configured");
            return;
        }

        tracing::info!("Scheduler started with {} task(s)", self.tasks.len());

        loop {
            let now = chrono::Utc::now();

            for task in &self.tasks {
                if should_run(&task.cron, &now) {
                    tracing::info!("Scheduling task: {}", task.method);
                    if let Err(e) = queue::enqueue(&self.pool, &task.method, &serde_json::json!({}), Default::default()).await {
                        tracing::error!("Failed to enqueue scheduled task '{}': {}", task.method, e);
                    }
                }
            }

            // Sleep until the next minute boundary
            let next_minute = (now + chrono::Duration::seconds(60))
                .with_second(0)
                .unwrap_or(now);
            let sleep_duration = (next_minute - now).to_std().unwrap_or(Duration::from_secs(60));
            tokio::time::sleep(sleep_duration).await;
        }
    }
}

/// Simple cron matching — supports: "* * * * *" (min hour day month weekday)
/// Each field can be: *, a number, or */N (every N).
fn should_run(cron: &str, now: &chrono::DateTime<chrono::Utc>) -> bool {
    use chrono::Datelike;
    use chrono::Timelike;

    let parts: Vec<&str> = cron.split_whitespace().collect();
    if parts.len() != 5 {
        return false;
    }

    let checks = [
        (parts[0], now.minute()),
        (parts[1], now.hour()),
        (parts[2], now.day()),
        (parts[3], now.month()),
        (parts[4], now.weekday().num_days_from_sunday()),
    ];

    for (pattern, value) in &checks {
        if !matches_cron_field(pattern, *value) {
            return false;
        }
    }

    true
}

fn matches_cron_field(pattern: &str, value: u32) -> bool {
    if pattern == "*" {
        return true;
    }
    if let Some(step) = pattern.strip_prefix("*/") {
        if let Ok(n) = step.parse::<u32>() {
            return n > 0 && value % n == 0;
        }
    }
    if let Ok(n) = pattern.parse::<u32>() {
        return value == n;
    }
    // Comma-separated values
    if pattern.contains(',') {
        return pattern.split(',').any(|p| p.trim().parse::<u32>().ok() == Some(value));
    }
    false
}
