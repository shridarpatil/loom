pub type QueueResult<T> = Result<T, QueueError>;

#[derive(thiserror::Error, Debug)]
pub enum QueueError {
    #[error("{0}")]
    Internal(String),
    #[error("Script error: {0}")]
    Script(String),
}
