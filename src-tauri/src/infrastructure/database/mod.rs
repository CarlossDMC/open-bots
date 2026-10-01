mod agent_repository;
mod approval_repository;
mod conversation_repository;
mod event_repository;
mod memory_repository;
mod routine_repository;
mod settings_repository;
mod task_repository;
mod wake_repository;

use std::{path::Path, sync::Mutex};

use rusqlite::Connection;

use crate::error::{AppError, AppResult};

pub use agent_repository::{AgentRepository, SqliteAgentRepository};
pub use approval_repository::{ApprovalRepository, SqliteApprovalRepository};
pub use conversation_repository::{ConversationRepository, SqliteConversationRepository};
pub use event_repository::{EventRepository, SqliteEventRepository};
pub use memory_repository::{MemoryRepository, SqliteMemoryRepository};
pub use routine_repository::{RoutineRepository, SqliteRoutineRepository};
pub use settings_repository::{SettingsRepository, SqliteSettingsRepository};
pub use task_repository::{SqliteTaskRepository, TaskRepository};
pub use wake_repository::{SqliteWakeRepository, WakeRepository};

/// Ordered schema migrations. Each entry runs once, when `user_version` is below its version.
const MIGRATIONS: [(i64, &str); 10] = [
    (1, include_str!("migrations/0001_initial.sql")),
    (2, include_str!("migrations/0002_agent_memories.sql")),
    (3, include_str!("migrations/0003_routines.sql")),
    (4, include_str!("migrations/0004_conversations.sql")),
    (5, include_str!("migrations/0005_agent_models.sql")),
    (6, include_str!("migrations/0006_task_ownership.sql")),
    (7, include_str!("migrations/0007_agent_wakes.sql")),
    (8, include_str!("migrations/0008_memory_source.sql")),
    (9, include_str!("migrations/0009_agent_mcp_servers.sql")),
    (
        10,
        include_str!("migrations/0010_conversation_message_source.sql"),
    ),
];

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
        let mut connection = Connection::open(path)?;
        migrate(&mut connection)?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    #[cfg(test)]
    pub fn in_memory() -> AppResult<Self> {
        let mut connection = Connection::open_in_memory()?;
        migrate(&mut connection)?;
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

fn migrate(connection: &mut Connection) -> AppResult<()> {
    connection.execute_batch("PRAGMA foreign_keys = ON;")?;
    apply_migrations(connection, &MIGRATIONS)
}

/// Runs each pending migration in its own transaction, so an interrupted or failed script
/// leaves neither partial schema changes nor an advanced `user_version` behind.
fn apply_migrations(connection: &mut Connection, migrations: &[(i64, &str)]) -> AppResult<()> {
    let current: i64 = connection.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    for &(version, script) in migrations {
        if current < version {
            let transaction = connection.transaction()?;
            transaction.execute_batch(script)?;
            transaction.commit()?;
            tracing::info!(version, "database migration applied");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn latest_version() -> i64 {
        MIGRATIONS[MIGRATIONS.len() - 1].0
    }

    fn user_version(connection: &Connection) -> i64 {
        connection
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .expect("user version")
    }

    #[test]
    fn rolls_back_a_migration_that_fails_partway() {
        let mut connection = Connection::open_in_memory().expect("connection");
        let migrations = [
            (1, "CREATE TABLE items (id TEXT); PRAGMA user_version = 1;"),
            (
                2,
                "ALTER TABLE items ADD COLUMN name TEXT; \
                 ALTER TABLE missing ADD COLUMN name TEXT; \
                 PRAGMA user_version = 2;",
            ),
        ];

        assert!(apply_migrations(&mut connection, &migrations).is_err());

        assert_eq!(user_version(&connection), 1);
        assert!(connection.prepare("SELECT name FROM items").is_err());
        let retry = [(
            2,
            "ALTER TABLE items ADD COLUMN name TEXT; PRAGMA user_version = 2;",
        )];
        apply_migrations(&mut connection, &retry).expect("retried migration");
        assert_eq!(user_version(&connection), 2);
    }

    #[test]
    fn applies_all_migrations_to_a_new_database() {
        let database = Database::in_memory().expect("database");
        database
            .with_connection(|connection| {
                assert_eq!(user_version(connection), latest_version());
                Ok(())
            })
            .expect("connection");
    }

    #[test]
    fn upgrades_a_version_one_database() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("legacy.sqlite3");
        {
            let connection = Connection::open(&path).expect("legacy connection");
            connection
                .execute_batch(MIGRATIONS[0].1)
                .expect("legacy schema");
            assert_eq!(user_version(&connection), 1);
        }
        let database = Database::open(&path).expect("upgraded database");
        database
            .with_connection(|connection| {
                assert_eq!(user_version(connection), latest_version());
                connection.prepare("SELECT id, source FROM agent_memories LIMIT 0")?;
                connection.prepare("SELECT id FROM agent_wakes LIMIT 0")?;
                connection.prepare("SELECT id FROM routines LIMIT 0")?;
                connection
                    .prepare("SELECT id, source_agent_id FROM conversation_messages LIMIT 0")?;
                connection.prepare(
                    "SELECT model, reasoning_effort, mcp_servers_json FROM agents LIMIT 0",
                )?;
                connection
                    .prepare("SELECT created_by_agent_id, result, updated_at FROM tasks LIMIT 0")?;
                Ok(())
            })
            .expect("connection");
    }
}
