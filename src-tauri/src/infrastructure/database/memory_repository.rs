use std::sync::Arc;

use rusqlite::{params, Row};
use uuid::Uuid;

use crate::{domain::memories::AgentMemory, error::AppResult};

use super::Database;

pub trait MemoryRepository: Send + Sync {
    fn list_for_agent(&self, agent_id: Uuid) -> AppResult<Vec<AgentMemory>>;
    fn find(&self, id: Uuid) -> AppResult<Option<AgentMemory>>;
    fn save(&self, memory: &AgentMemory) -> AppResult<()>;
    fn delete(&self, id: Uuid) -> AppResult<()>;
}

pub struct SqliteMemoryRepository {
    database: Arc<Database>,
}

impl SqliteMemoryRepository {
    pub fn new(database: Arc<Database>) -> Self {
        Self { database }
    }
}

impl MemoryRepository for SqliteMemoryRepository {
    fn list_for_agent(&self, agent_id: Uuid) -> AppResult<Vec<AgentMemory>> {
        self.database.with_connection(|connection| {
            let mut statement = connection.prepare(
                "SELECT id, agent_id, content, created_at FROM agent_memories \
                 WHERE agent_id = ?1 ORDER BY created_at DESC",
            )?;
            let memories = statement
                .query_map([agent_id.to_string()], map_memory)?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(memories)
        })
    }

    fn find(&self, id: Uuid) -> AppResult<Option<AgentMemory>> {
        self.database.with_connection(|connection| {
            let mut statement = connection.prepare(
                "SELECT id, agent_id, content, created_at FROM agent_memories WHERE id = ?1",
            )?;
            let mut rows = statement.query([id.to_string()])?;
            Ok(rows.next()?.map(map_memory).transpose()?)
        })
    }

    fn save(&self, memory: &AgentMemory) -> AppResult<()> {
        self.database.with_connection(|connection| {
            connection.execute(
                "INSERT INTO agent_memories (id, agent_id, content, created_at) VALUES (?1, ?2, ?3, ?4) \
                 ON CONFLICT(id) DO UPDATE SET content=excluded.content",
                params![
                    memory.id.to_string(),
                    memory.agent_id.to_string(),
                    memory.content,
                    memory.created_at.to_rfc3339(),
                ],
            )?;
            Ok(())
        })
    }

    fn delete(&self, id: Uuid) -> AppResult<()> {
        self.database.with_connection(|connection| {
            connection.execute("DELETE FROM agent_memories WHERE id = ?1", [id.to_string()])?;
            Ok(())
        })
    }
}

fn map_memory(row: &Row<'_>) -> rusqlite::Result<AgentMemory> {
    Ok(AgentMemory {
        id: parse(row.get::<_, String>(0)?)?,
        agent_id: parse(row.get::<_, String>(1)?)?,
        content: row.get(2)?,
        created_at: parse(row.get::<_, String>(3)?)?,
    })
}

fn parse<T: std::str::FromStr>(value: String) -> rusqlite::Result<T>
where
    T::Err: std::error::Error + Send + Sync + 'static,
{
    value.parse().map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(error))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::agents::{Agent, IdentityColor, NewAgent},
        infrastructure::database::{AgentRepository, SqliteAgentRepository},
    };

    #[test]
    fn persists_lists_and_deletes_memories() {
        let database = Arc::new(Database::in_memory().expect("database"));
        let agent = Agent::create(NewAgent {
            name: "Nova".into(),
            role: "Frontend Engineer".into(),
            description: String::new(),
            provider_id: "mock".into(),
            identity_color: IdentityColor::Cyan,
            workspace: "/workspace".into(),
            instructions: String::new(),
        })
        .expect("agent");
        SqliteAgentRepository::new(Arc::clone(&database))
            .save(&agent)
            .expect("save agent");
        let repository = SqliteMemoryRepository::new(database);
        let memory = AgentMemory::create(agent.id, "Prefers small commits").expect("memory");

        repository.save(&memory).expect("save");
        assert_eq!(
            repository.list_for_agent(agent.id).expect("list"),
            vec![memory.clone()]
        );
        assert_eq!(
            repository.find(memory.id).expect("find"),
            Some(memory.clone())
        );

        repository.delete(memory.id).expect("delete");
        assert!(repository
            .list_for_agent(agent.id)
            .expect("list")
            .is_empty());
    }
}
