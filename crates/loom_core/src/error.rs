use serde::Serialize;

pub type LoomResult<T> = Result<T, LoomError>;

#[derive(thiserror::Error, Debug)]
pub enum LoomError {
    #[error("Validation: {0}")]
    Validation(String),

    #[error("Not Found: {doctype} {name}")]
    NotFound { doctype: String, name: String },

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Duplicate entry: {0}")]
    DuplicateEntry(String),

    #[error("Link broken: {doctype}.{fieldname} = {value}")]
    LinkValidation {
        doctype: String,
        fieldname: String,
        value: String,
    },

    #[error("Script error: {0}")]
    Script(String),

    #[error("Plugin error: {0}")]
    Plugin(String),

    #[error("DB error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("{0}")]
    Internal(String),
}

/// Serializable error response for the API layer.
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub error_type: &'static str,
}

impl LoomError {
    pub fn error_type(&self) -> &'static str {
        match self {
            LoomError::Validation(_) => "ValidationError",
            LoomError::NotFound { .. } => "NotFoundError",
            LoomError::PermissionDenied(_) => "PermissionError",
            LoomError::DuplicateEntry(_) => "DuplicateEntryError",
            LoomError::LinkValidation { .. } => "LinkValidationError",
            LoomError::Script(_) => "ScriptError",
            LoomError::Plugin(_) => "PluginError",
            LoomError::Database(_) => "DatabaseError",
            LoomError::Internal(_) => "InternalError",
        }
    }

    pub fn to_response(&self) -> ErrorResponse {
        ErrorResponse {
            error: self.to_string(),
            error_type: self.error_type(),
        }
    }
}
