use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::PgPool;

use crate::error::{QueueError, QueueResult};

/// A background job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: i64,
    pub method: String,
    pub args: Value,
    pub queue: String,
    pub priority: i32,
    pub status: String,
    pub attempts: i32,
    pub max_retries: i32,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobStatus {
    Queued,
    Running,
    Completed,
    Failed,
}

/// Options for enqueuing a job.
#[derive(Debug, Clone, Default)]
pub struct EnqueueOptions {
    /// Queue name (default: "default")
    pub queue: Option<String>,
    /// Priority — higher runs first (default: 0)
    pub priority: Option<i32>,
    /// Max retry attempts (default: 3)
    pub max_retries: Option<i32>,
}

/// Enqueue a job for background execution.
pub async fn enqueue(
    pool: &PgPool,
    method: &str,
    args: &Value,
    opts: EnqueueOptions,
) -> QueueResult<i64> {
    let queue = opts.queue.as_deref().unwrap_or("default");
    let priority = opts.priority.unwrap_or(0);
    let max_retries = opts.max_retries.unwrap_or(3);

    let id: i64 = sqlx::query_scalar(
        "INSERT INTO \"__job_queue\" (method, args, queue, priority, status, attempts, max_retries, created) \
         VALUES ($1, $2, $3, $4, 'queued', 0, $5, NOW()) RETURNING id",
    )
    .bind(method)
    .bind(args)
    .bind(queue)
    .bind(priority)
    .bind(max_retries)
    .fetch_one(pool)
    .await
    .map_err(|e| QueueError::Internal(format!("Failed to enqueue: {}", e)))?;

    tracing::info!(
        "Enqueued job {} for '{}' on queue '{}' (priority {})",
        id,
        method,
        queue,
        priority
    );
    Ok(id)
}

/// Atomically fetch the next queued job from a specific queue, ordered by priority (desc) then id (asc).
pub async fn dequeue(pool: &PgPool, queue_name: &str) -> QueueResult<Option<Job>> {
    let row: Option<(i64, String, Value, String, i32, i32, i32)> = sqlx::query_as(
        "UPDATE \"__job_queue\" \
         SET status = 'running', started = NOW(), attempts = attempts + 1 \
         WHERE id = ( \
           SELECT id FROM \"__job_queue\" \
           WHERE status = 'queued' AND queue = $1 \
           ORDER BY priority DESC, id ASC \
           LIMIT 1 \
           FOR UPDATE SKIP LOCKED \
         ) \
         RETURNING id, method, args, queue, priority, attempts, max_retries",
    )
    .bind(queue_name)
    .fetch_optional(pool)
    .await
    .map_err(|e| QueueError::Internal(format!("Failed to dequeue: {}", e)))?;

    Ok(row.map(
        |(id, method, args, queue, priority, attempts, max_retries)| Job {
            id,
            method,
            args,
            queue,
            priority,
            status: "running".to_string(),
            attempts,
            max_retries,
            error: None,
        },
    ))
}

/// Mark a job as completed.
pub async fn mark_completed(pool: &PgPool, job_id: i64) -> QueueResult<()> {
    sqlx::query("UPDATE \"__job_queue\" SET status = 'completed', finished = NOW() WHERE id = $1")
        .bind(job_id)
        .execute(pool)
        .await
        .map_err(|e| QueueError::Internal(e.to_string()))?;
    Ok(())
}

/// Mark a job as failed. Re-queues if retries remain.
pub async fn mark_failed(
    pool: &PgPool,
    job_id: i64,
    error: &str,
    can_retry: bool,
) -> QueueResult<()> {
    let new_status = if can_retry { "queued" } else { "failed" };
    sqlx::query(
        "UPDATE \"__job_queue\" SET status = $1, error = $2, finished = NOW() WHERE id = $3",
    )
    .bind(new_status)
    .bind(error)
    .bind(job_id)
    .execute(pool)
    .await
    .map_err(|e| QueueError::Internal(e.to_string()))?;
    Ok(())
}
