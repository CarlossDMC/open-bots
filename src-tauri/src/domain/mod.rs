pub mod agents;
pub mod approvals;
pub mod artifacts;
pub mod conversations;
pub mod events;
pub mod inbox;
pub mod memories;
pub mod routines;
pub mod tasks;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum DomainError {
    #[error("invalid input: {0}")]
    Validation(String),
}

pub type DomainResult<T> = Result<T, DomainError>;
