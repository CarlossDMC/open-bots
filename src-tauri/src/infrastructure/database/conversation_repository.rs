use std::sync::Arc;

use chrono::{DateTime, Utc};
use rusqlite::{params, OptionalExtension, Row};
use uuid::Uuid;

use crate::{
    domain::conversations::ConversationMessage,
    error::{AppError, AppResult},
};

use super::Database;

pub trait ConversationRepository: Send + Sync {
    /// The most recent `limit` messages, oldest first.
    fn list_messages(&self, agent_id: Uuid, limit: usize) -> AppResult<Vec<ConversationMessage>>;
    fn append_message(&self, message: &ConversationMessage) -> AppResult<()>;
    fn provider_session(&self, agent_id: Uuid, provider_id: &str) -> AppResult<Option<String>>;
    fn save_provider_session(
        &self,
        agent_id: Uuid,
        provider_id: &str,
        session_id: &str,
    ) -> AppResult<()>;
    /// Forgets every provider session of the agent; returns how many were removed.
    fn clear_provider_sessions(&self, agent_id: Uuid) -> AppResult<usize>;
}

pub struct SqliteConversationRepository {
    database: Arc<Database>,
}

impl SqliteConversationRepository {
    pub fn new(database: Arc<Database>) -> Self {
        Self { database }
    }
}

impl ConversationRepository for SqliteConversationRepository {
    fn list_messages(&self, agent_id: Uuid, limit: usize) -> AppResult<Vec<ConversationMessage>> {
        let limit = i64::try_from(limit)
            .map_err(|_| AppError::Validation("message limit is too large".into()))?;
        self.database.with_connection(|connection| {
            let mut statement = connection.prepare(
                "SELECT id, agent_id, role, content, created_at_ms, source_agent_id FROM ( \
                   SELECT * FROM conversation_messages WHERE agent_id = ?1 \
                   ORDER BY created_at_ms DESC, sequence DESC LIMIT ?2 \
                 ) ORDER BY created_at_ms, sequence",
            )?;
            let messages = statement
                .query_map(params![agent_id.to_string(), limit], map_message)?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(messages)
        })
    }

    fn append_message(&self, message: &ConversationMessage) -> AppResult<()> {
        self.database.with_connection(|connection| {
            connection.execute(
                "INSERT INTO conversation_messages (id, agent_id, role, content, created_at_ms, sequence, source_agent_id) \
                 VALUES (?1, ?2, ?3, ?4, ?5, \
                   (SELECT COALESCE(MAX(sequence), 0) + 1 FROM conversation_messages WHERE agent_id = ?2), ?6)",
                params![
                    message.id.to_string(),
                    message.agent_id.to_string(),
                    serde_json::to_string(&message.role)?.trim_matches('"'),
                    message.content,
                    message.created_at.timestamp_millis(),
                    message.source_agent_id.map(|id| id.to_string()),
                ],
            )?;
            Ok(())
        })
    }

    fn provider_session(&self, agent_id: Uuid, provider_id: &str) -> AppResult<Option<String>> {
        self.database.with_connection(|connection| {
            Ok(connection
                .query_row(
                    "SELECT session_id FROM provider_sessions WHERE agent_id = ?1 AND provider_id = ?2",
                    params![agent_id.to_string(), provider_id],
                    |row| row.get(0),
                )
                .optional()?)
        })
    }

    fn save_provider_session(
        &self,
        agent_id: Uuid,
        provider_id: &str,
        session_id: &str,
    ) -> AppResult<()> {
        self.database.with_connection(|connection| {
            connection.execute(
                "INSERT INTO provider_sessions (agent_id, provider_id, session_id, updated_at) \
                 VALUES (?1, ?2, ?3, ?4) \
                 ON CONFLICT(agent_id, provider_id) DO UPDATE SET \
                 session_id=excluded.session_id, updated_at=excluded.updated_at",
                params![
                    agent_id.to_string(),
                    provider_id,
                    session_id,
                    Utc::now().to_rfc3339()
                ],
            )?;
            Ok(())
        })
    }

    fn clear_provider_sessions(&self, agent_id: Uuid) -> AppResult<usize> {
        self.database.with_connection(|connection| {
            Ok(connection.execute(
                "DELETE FROM provider_sessions WHERE agent_id = ?1",
                [agent_id.to_string()],
            )?)
        })
    }
}

fn map_message(row: &Row<'_>) -> rusqlite::Result<ConversationMessage> {
    let role = row.get::<_, String>(2)?;
    let created_at_ms = row.get::<_, i64>(4)?;
    Ok(ConversationMessage {
        id: parse(row.get::<_, String>(0)?)?,
        agent_id: parse(row.get::<_, String>(1)?)?,
        source_agent_id: row.get::<_, Option<String>>(5)?.map(parse).transpose()?,
        role: serde_json::from_str(&format!("\"{role}\"")).map_err(to_sql_error)?,
        content: row.get(3)?,
        created_at: DateTime::from_timestamp_millis(created_at_ms)
            .ok_or(rusqlite::Error::IntegralValueOutOfRange(4, created_at_ms))?,
    })
}

fn parse<T: std::str::FromStr>(value: String) -> rusqlite::Result<T>
where
    T::Err: std::error::Error + Send + Sync + 'static,
{
    value.parse().map_err(to_sql_error)
}

fn to_sql_error(error: impl std::error::Error + Send + Sync + 'static) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(error))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::{
            agents::{Agent, IdentityColor, NewAgent},
            conversations::MessageRole,
        },
        infrastructure::database::{AgentRepository, SqliteAgentRepository},
    };
    use chrono::SubsecRound;

    fn setup() -> (SqliteConversationRepository, Agent) {
        let database = Arc::new(Database::in_memory().expect("database"));
        let agent = Agent::create(NewAgent {
            name: "Atlas".into(),
            role: "Engineer".into(),
            description: String::new(),
            provider_id: "codex".into(),
            identity_color: IdentityColor::Indigo,
            workspace: "/workspace".into(),
            instructions: String::new(),
            model_selection: Default::default(),
        })
        .expect("agent");
        SqliteAgentRepository::new(Arc::clone(&database))
            .save(&agent)
            .expect("save agent");
        (SqliteConversationRepository::new(database), agent)
    }

    fn message(agent: &Agent, role: MessageRole, content: &str) -> ConversationMessage {
        let mut message = ConversationMessage::new(agent.id, role, content).expect("message");
        message.created_at = message.created_at.trunc_subsecs(3);
        message
    }

    #[test]
    fn lists_the_latest_messages_in_order() {
        let (repository, agent) = setup();
        let first = message(&agent, MessageRole::User, "one");
        let mut second = message(&agent, MessageRole::Agent, "two");
        second.created_at = first.created_at;
        let mut third = message(&agent, MessageRole::System, "three");
        third.source_agent_id = Some(agent.id);
        for message in [&first, &second, &third] {
            repository.append_message(message).expect("append");
        }

        assert_eq!(
            repository.list_messages(agent.id, 10).expect("list"),
            vec![first, second.clone(), third.clone()]
        );
        assert_eq!(
            repository.list_messages(agent.id, 2).expect("list"),
            vec![second, third]
        );
    }

    #[test]
    fn stores_one_provider_session_per_agent_and_provider() {
        let (repository, agent) = setup();
        assert_eq!(
            repository
                .provider_session(agent.id, "codex")
                .expect("find"),
            None
        );
        repository
            .save_provider_session(agent.id, "codex", "thread-1")
            .expect("save");
        repository
            .save_provider_session(agent.id, "codex", "thread-2")
            .expect("update");
        assert_eq!(
            repository
                .provider_session(agent.id, "codex")
                .expect("find"),
            Some("thread-2".into())
        );
        assert_eq!(
            repository.clear_provider_sessions(agent.id).expect("clear"),
            1
        );
        assert_eq!(
            repository
                .provider_session(agent.id, "codex")
                .expect("find after clear"),
            None
        );
        assert_eq!(
            repository.provider_session(agent.id, "mock").expect("find"),
            None
        );
    }
}
