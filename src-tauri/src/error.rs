use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error(transparent)]
    Domain(#[from] crate::domain::DomainError),
    #[error("database operation failed: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("serialization failed: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("invalid input: {0}")]
    Validation(String),
    #[error("resource not found: {0}")]
    NotFound(String),
    #[error("provider operation failed: {0}")]
    Provider(String),
    #[error("process operation failed: {0}")]
    Process(String),
    #[error("git operation failed: {0}")]
    Git(String),
    #[error("runtime operation is not implemented: {0}")]
    Unsupported(String),
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;
