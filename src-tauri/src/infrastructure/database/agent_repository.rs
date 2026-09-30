use std::sync::Arc;

use rusqlite::{params, Row};
use uuid::Uuid;

use crate::{
    domain::agents::{Agent, AgentPermissions},
    error::{AppError, AppResult},
};

use super::Database;

pub trait AgentRepository: Send + Sync {
    fn list(&self) -> AppResult<Vec<Agent>>;
    fn find(&self, id: Uuid) -> AppResult<Option<Agent>>;
    fn save(&self, agent: &Agent) -> AppResult<()>;
}

pub struct SqliteAgentRepository {
    database: Arc<Database>,
}

impl SqliteAgentRepository {
    pub fn new(database: Arc<Database>) -> Self {
        Self { database }
    }
}

impl AgentRepository for SqliteAgentRepository {
    fn list(&self) -> AppResult<Vec<Agent>> {
        self.database.with_connection(|connection| {
            let mut statement = connection.prepare("SELECT id, name, role, description, provider_id, identity_color, avatar_variant, workspace, status, instructions, permissions_json, current_task, created_at, updated_at FROM agents ORDER BY created_at DESC")?;
            let agents = statement.query_map([], map_agent)?.collect::<Result<Vec<_>, _>>()?;
            Ok(agents)
        })
    }

    fn find(&self, id: Uuid) -> AppResult<Option<Agent>> {
        self.database.with_connection(|connection| {
            let mut statement = connection.prepare("SELECT id, name, role, description, provider_id, identity_color, avatar_variant, workspace, status, instructions, permissions_json, current_task, created_at, updated_at FROM agents WHERE id = ?1")?;
            let mut rows = statement.query([id.to_string()])?;
            rows.next()?.map(map_agent).transpose().map_err(AppError::from)
        })
    }

    fn save(&self, agent: &Agent) -> AppResult<()> {
        self.database.with_connection(|connection| {
            connection.execute("INSERT INTO agents (id, name, role, description, provider_id, identity_color, avatar_variant, workspace, status, instructions, permissions_json, current_task, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14) ON CONFLICT(id) DO UPDATE SET name=excluded.name, role=excluded.role, description=excluded.description, provider_id=excluded.provider_id, identity_color=excluded.identity_color, avatar_variant=excluded.avatar_variant, workspace=excluded.workspace, status=excluded.status, instructions=excluded.instructions, permissions_json=excluded.permissions_json, current_task=excluded.current_task, updated_at=excluded.updated_at", params![agent.id.to_string(), agent.name, agent.role, agent.description, agent.provider_id, enum_json(&agent.identity_color)?, agent.avatar_variant, agent.workspace, enum_json(&agent.status)?, agent.instructions, serde_json::to_string(&agent.permissions)?, agent.current_task, agent.created_at.to_rfc3339(), agent.updated_at.to_rfc3339()])?;
            Ok(())
        })
    }
}

fn enum_json<T: serde::Serialize>(value: &T) -> AppResult<String> {
    Ok(serde_json::to_string(value)?.trim_matches('"').to_owned())
}

fn map_agent(row: &Row<'_>) -> rusqlite::Result<Agent> {
    Ok(Agent {
        id: parse(row.get::<_, String>(0)?)?,
        name: row.get(1)?,
        role: row.get(2)?,
        description: row.get(3)?,
        provider_id: row.get(4)?,
        identity_color: deserialize_string(row.get(5)?)?,
        avatar_variant: row.get(6)?,
        workspace: row.get(7)?,
        status: deserialize_string(row.get(8)?)?,
        instructions: row.get(9)?,
        permissions: serde_json::from_str::<AgentPermissions>(&row.get::<_, String>(10)?)
            .map_err(to_sql_error)?,
        current_task: row.get(11)?,
        created_at: parse(row.get::<_, String>(12)?)?,
        updated_at: parse(row.get::<_, String>(13)?)?,
    })
}

fn parse<T: std::str::FromStr>(value: String) -> rusqlite::Result<T>
where
    T::Err: std::error::Error + Send + Sync + 'static,
{
    value.parse().map_err(to_sql_error)
}
fn deserialize_string<T: serde::de::DeserializeOwned>(value: String) -> rusqlite::Result<T> {
    serde_json::from_str(&format!("\"{value}\"")).map_err(to_sql_error)
}
fn to_sql_error(error: impl std::error::Error + Send + Sync + 'static) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(error))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::agents::{IdentityColor, NewAgent};
    #[test]
    fn persists_and_loads_agents() {
        let database = Arc::new(Database::in_memory().expect("database"));
        let repository = SqliteAgentRepository::new(database);
        let agent = Agent::create(NewAgent {
            name: "Nova".into(),
            role: "Frontend Engineer".into(),
            description: "Builds interfaces".into(),
            provider_id: "mock".into(),
            identity_color: IdentityColor::Cyan,
            workspace: "/workspace".into(),
            instructions: "Keep it clear".into(),
        })
        .expect("agent");
        repository.save(&agent).expect("save");
        assert_eq!(repository.find(agent.id).expect("find"), Some(agent));
    }
}
