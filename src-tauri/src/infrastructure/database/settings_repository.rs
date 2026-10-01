use std::sync::Arc;

use rusqlite::{params, OptionalExtension};
use serde_json::Value;

use crate::error::AppResult;

use super::Database;

/// Application settings stored as JSON values under stable keys.
pub trait SettingsRepository: Send + Sync {
    fn get(&self, key: &str) -> AppResult<Option<Value>>;
    fn set(&self, key: &str, value: &Value) -> AppResult<()>;
}

pub struct SqliteSettingsRepository {
    database: Arc<Database>,
}

impl SqliteSettingsRepository {
    pub fn new(database: Arc<Database>) -> Self {
        Self { database }
    }
}

impl SettingsRepository for SqliteSettingsRepository {
    fn get(&self, key: &str) -> AppResult<Option<Value>> {
        self.database.with_connection(|connection| {
            let raw: Option<String> = connection
                .query_row(
                    "SELECT value_json FROM settings WHERE key = ?1",
                    [key],
                    |row| row.get(0),
                )
                .optional()?;
            Ok(raw.map(|text| serde_json::from_str(&text)).transpose()?)
        })
    }

    fn set(&self, key: &str, value: &Value) -> AppResult<()> {
        self.database.with_connection(|connection| {
            connection.execute(
                "INSERT INTO settings (key, value_json) VALUES (?1, ?2) \
                 ON CONFLICT(key) DO UPDATE SET value_json = excluded.value_json",
                params![key, serde_json::to_string(value)?],
            )?;
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn stores_and_replaces_values() {
        let repository =
            SqliteSettingsRepository::new(Arc::new(Database::in_memory().expect("database")));
        assert_eq!(repository.get("runtime.maxChainTurns").expect("get"), None);
        repository
            .set("runtime.maxChainTurns", &json!(5))
            .expect("set");
        repository
            .set("runtime.maxChainTurns", &json!(8))
            .expect("replace");
        assert_eq!(
            repository.get("runtime.maxChainTurns").expect("get"),
            Some(json!(8))
        );
    }
}
