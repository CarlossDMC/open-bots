mod agent_repository;
mod event_repository;

use std::{path::Path, sync::Mutex};

use rusqlite::Connection;

use crate::error::{AppError, AppResult};

pub use agent_repository::{AgentRepository, SqliteAgentRepository};
pub use event_repository::{EventRepository, SqliteEventRepository};

pub struct Database {
    connection: Mutex<Connection>,
}

impl Database {
    pub fn open(path: &Path) -> AppResult<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| {
                AppError::Database(rusqlite::Error::ToSqlConversionFailure(Box::new(error)))
            })?;
        }
        let connection = Connection::open(path)?;
        connection.execute_batch(include_str!("migrations/0001_initial.sql"))?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    #[cfg(test)]
    pub fn in_memory() -> AppResult<Self> {
        let connection = Connection::open_in_memory()?;
        connection.execute_batch(include_str!("migrations/0001_initial.sql"))?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    pub fn with_connection<T>(
        &self,
        operation: impl FnOnce(&Connection) -> AppResult<T>,
    ) -> AppResult<T> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| AppError::Database(rusqlite::Error::InvalidQuery))?;
        operation(&connection)
    }
}
