use std::sync::Arc;

use chrono::Utc;
use rusqlite::{params, Row};

use crate::{domain::mcp_servers::McpCatalogEntry, error::AppResult};

use super::Database;

/// The global catalog of MCP servers agents can select, grouped by provider.
pub trait McpCatalogRepository: Send + Sync {
    fn list(&self) -> AppResult<Vec<McpCatalogEntry>>;
    fn list_for_provider(&self, provider_id: &str) -> AppResult<Vec<McpCatalogEntry>>;
    /// Replaces the provider's entries with `names`, keeping the time each existing entry was
    /// first added.
    fn replace_for_provider(&self, provider_id: &str, names: &[String]) -> AppResult<()>;
}

pub struct SqliteMcpCatalogRepository {
    database: Arc<Database>,
}

impl SqliteMcpCatalogRepository {
    pub fn new(database: Arc<Database>) -> Self {
        Self { database }
    }
}

impl McpCatalogRepository for SqliteMcpCatalogRepository {
    fn list(&self) -> AppResult<Vec<McpCatalogEntry>> {
        self.database.with_connection(|connection| {
            let mut statement = connection.prepare(
                "SELECT provider_id, name, added_at FROM mcp_catalog ORDER BY provider_id, name",
            )?;
            let rows = statement.query_map([], map_entry)?;
            Ok(rows.collect::<Result<_, _>>()?)
        })
    }

    fn list_for_provider(&self, provider_id: &str) -> AppResult<Vec<McpCatalogEntry>> {
        self.database.with_connection(|connection| {
            let mut statement = connection.prepare(
                "SELECT provider_id, name, added_at FROM mcp_catalog WHERE provider_id = ?1 \
                 ORDER BY name",
            )?;
            let rows = statement.query_map([provider_id], map_entry)?;
            Ok(rows.collect::<Result<_, _>>()?)
        })
    }

    fn replace_for_provider(&self, provider_id: &str, names: &[String]) -> AppResult<()> {
        self.database.with_connection(|connection| {
            let transaction = connection.unchecked_transaction()?;
            let kept = serde_json::to_string(names)?;
            transaction.execute(
                "DELETE FROM mcp_catalog WHERE provider_id = ?1 \
                 AND name NOT IN (SELECT value FROM json_each(?2))",
                params![provider_id, kept],
            )?;
            let now = Utc::now().to_rfc3339();
            for name in names {
                transaction.execute(
                    "INSERT INTO mcp_catalog (provider_id, name, added_at) VALUES (?1, ?2, ?3) \
                     ON CONFLICT(provider_id, name) DO NOTHING",
                    params![provider_id, name, now],
                )?;
            }
            transaction.commit()?;
            Ok(())
        })
    }
}

fn map_entry(row: &Row<'_>) -> rusqlite::Result<McpCatalogEntry> {
    let added_at: String = row.get(2)?;
    Ok(McpCatalogEntry {
        provider_id: row.get(0)?,
        name: row.get(1)?,
        added_at: added_at.parse().map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
                2,
                rusqlite::types::Type::Text,
                Box::new(error),
            )
        })?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replaces_a_providers_entries_and_keeps_others() {
        let repository =
            SqliteMcpCatalogRepository::new(Arc::new(Database::in_memory().expect("database")));
        repository
            .replace_for_provider("claude-code", &["github".into(), "sentry".into()])
            .expect("save");
        repository
            .replace_for_provider("codex", &["docs".into()])
            .expect("save other provider");
        let first_added = repository.list_for_provider("claude-code").expect("list")[0].added_at;

        repository
            .replace_for_provider(
                "claude-code",
                &["github".into(), "claude.ai Atlassian".into()],
            )
            .expect("replace");

        let claude = repository.list_for_provider("claude-code").expect("list");
        let names: Vec<_> = claude.iter().map(|entry| entry.name.as_str()).collect();
        assert_eq!(names, ["claude.ai Atlassian", "github"]);
        assert_eq!(claude[1].added_at, first_added);
        assert_eq!(repository.list().expect("all").len(), 3);
        repository
            .replace_for_provider("claude-code", &[])
            .expect("clear");
        assert!(repository
            .list_for_provider("claude-code")
            .expect("list")
            .is_empty());
    }
}
