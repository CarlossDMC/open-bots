use std::sync::Arc;

use chrono::DateTime;
use rusqlite::{params, Connection, Row};
use uuid::Uuid;

use crate::{
    domain::groups::{Group, GroupAuthor, GroupMessage},
    error::{AppError, AppResult},
};

use super::Database;

const SELECT_GROUPS: &str =
    "SELECT id, name, topic, created_by_json, round_json, created_at, updated_at FROM agent_groups";
const MESSAGE_COLUMNS: &str = "id, group_id, author_json, content, created_at_ms";

pub trait GroupRepository: Send + Sync {
    /// Inserts or updates the group, including its members and round.
    fn save(&self, group: &Group) -> AppResult<()>;
    fn find(&self, id: Uuid) -> AppResult<Option<Group>>;
    /// Every group, most recently active first.
    fn list(&self) -> AppResult<Vec<Group>>;
    /// Deletes the group and its messages; returns whether it existed.
    fn delete(&self, id: Uuid) -> AppResult<bool>;
    /// Appends a message and marks the group as active at its time.
    fn append_message(&self, message: &GroupMessage) -> AppResult<()>;
    /// The most recent `limit` messages, oldest first.
    fn list_messages(&self, group_id: Uuid, limit: usize) -> AppResult<Vec<GroupMessage>>;
    /// Messages after the agent's latest message in the group, oldest first and at most
    /// `limit` of the newest. Everything is new when the agent has not spoken yet.
    fn messages_since_last_from(
        &self,
        group_id: Uuid,
        agent_id: Uuid,
        limit: usize,
    ) -> AppResult<Vec<GroupMessage>>;
}

pub struct SqliteGroupRepository {
    database: Arc<Database>,
}

impl SqliteGroupRepository {
    pub fn new(database: Arc<Database>) -> Self {
        Self { database }
    }
}

impl GroupRepository for SqliteGroupRepository {
    fn save(&self, group: &Group) -> AppResult<()> {
        self.database.with_connection(|connection| {
            let transaction = connection.unchecked_transaction()?;
            transaction.execute(
                "INSERT INTO agent_groups (id, name, topic, created_by_json, round_json, created_at, updated_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7) ON CONFLICT(id) DO UPDATE SET \
                 name=excluded.name, topic=excluded.topic, round_json=excluded.round_json, \
                 updated_at=excluded.updated_at",
                params![
                    group.id.to_string(),
                    group.name,
                    group.topic,
                    serde_json::to_string(&group.created_by)?,
                    serde_json::to_string(&group.round)?,
                    group.created_at.to_rfc3339(),
                    group.updated_at.to_rfc3339(),
                ],
            )?;
            transaction.execute(
                "DELETE FROM group_members WHERE group_id = ?1",
                [group.id.to_string()],
            )?;
            for (position, agent_id) in group.member_ids.iter().enumerate() {
                transaction.execute(
                    "INSERT INTO group_members (group_id, agent_id, position) VALUES (?1, ?2, ?3)",
                    params![group.id.to_string(), agent_id.to_string(), position as i64],
                )?;
            }
            transaction.commit()?;
            Ok(())
        })
    }

    fn find(&self, id: Uuid) -> AppResult<Option<Group>> {
        self.database.with_connection(|connection| {
            let mut statement = connection.prepare(&format!("{SELECT_GROUPS} WHERE id = ?1"))?;
            let mut rows = statement.query([id.to_string()])?;
            let Some(group) = rows.next()?.map(map_group).transpose()? else {
                return Ok(None);
            };
            Ok(Some(with_members(connection, group)?))
        })
    }

    fn list(&self) -> AppResult<Vec<Group>> {
        self.database.with_connection(|connection| {
            let mut statement =
                connection.prepare(&format!("{SELECT_GROUPS} ORDER BY updated_at DESC"))?;
            let groups = statement
                .query_map([], map_group)?
                .collect::<Result<Vec<_>, _>>()?;
            groups
                .into_iter()
                .map(|group| with_members(connection, group))
                .collect()
        })
    }

    fn delete(&self, id: Uuid) -> AppResult<bool> {
        self.database.with_connection(|connection| {
            Ok(connection.execute("DELETE FROM agent_groups WHERE id = ?1", [id.to_string()])? > 0)
        })
    }

    fn append_message(&self, message: &GroupMessage) -> AppResult<()> {
        self.database.with_connection(|connection| {
            let transaction = connection.unchecked_transaction()?;
            transaction.execute(
                "INSERT INTO group_messages (id, group_id, author_json, author_agent_id, content, created_at_ms, sequence) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, \
                   (SELECT COALESCE(MAX(sequence), 0) + 1 FROM group_messages WHERE group_id = ?2))",
                params![
                    message.id.to_string(),
                    message.group_id.to_string(),
                    serde_json::to_string(&message.author)?,
                    message.author.agent_id().map(|id| id.to_string()),
                    message.content,
                    message.created_at.timestamp_millis(),
                ],
            )?;
            transaction.execute(
                "UPDATE agent_groups SET updated_at = ?2 WHERE id = ?1",
                params![message.group_id.to_string(), message.created_at.to_rfc3339()],
            )?;
            transaction.commit()?;
            Ok(())
        })
    }

    fn list_messages(&self, group_id: Uuid, limit: usize) -> AppResult<Vec<GroupMessage>> {
        let limit = sql_limit(limit)?;
        self.database.with_connection(|connection| {
            let mut statement = connection.prepare(&format!(
                "SELECT {MESSAGE_COLUMNS} FROM ( \
                   SELECT * FROM group_messages WHERE group_id = ?1 \
                   ORDER BY created_at_ms DESC, sequence DESC LIMIT ?2 \
                 ) ORDER BY created_at_ms, sequence"
            ))?;
            let messages = statement
                .query_map(params![group_id.to_string(), limit], map_message)?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(messages)
        })
    }

    fn messages_since_last_from(
        &self,
        group_id: Uuid,
        agent_id: Uuid,
        limit: usize,
    ) -> AppResult<Vec<GroupMessage>> {
        let limit = sql_limit(limit)?;
        self.database.with_connection(|connection| {
            let mut statement = connection.prepare(&format!(
                "SELECT {MESSAGE_COLUMNS} FROM ( \
                   SELECT * FROM group_messages WHERE group_id = ?1 AND sequence > COALESCE( \
                     (SELECT MAX(sequence) FROM group_messages \
                      WHERE group_id = ?1 AND author_agent_id = ?2), 0) \
                   ORDER BY sequence DESC LIMIT ?3 \
                 ) ORDER BY sequence"
            ))?;
            let messages = statement
                .query_map(
                    params![group_id.to_string(), agent_id.to_string(), limit],
                    map_message,
                )?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(messages)
        })
    }
}

fn with_members(connection: &Connection, mut group: Group) -> AppResult<Group> {
    let mut statement = connection
        .prepare("SELECT agent_id FROM group_members WHERE group_id = ?1 ORDER BY position")?;
    group.member_ids = statement
        .query_map([group.id.to_string()], |row| {
            parse(row.get::<_, String>(0)?)
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(group)
}

fn sql_limit(limit: usize) -> AppResult<i64> {
    i64::try_from(limit).map_err(|_| AppError::Validation("message limit is too large".into()))
}

fn map_group(row: &Row<'_>) -> rusqlite::Result<Group> {
    Ok(Group {
        id: parse(row.get::<_, String>(0)?)?,
        name: row.get(1)?,
        topic: row.get(2)?,
        member_ids: Vec::new(),
        created_by: serde_json::from_str(&row.get::<_, String>(3)?).map_err(to_sql_error)?,
        round: serde_json::from_str(&row.get::<_, String>(4)?).map_err(to_sql_error)?,
        created_at: parse(row.get::<_, String>(5)?)?,
        updated_at: parse(row.get::<_, String>(6)?)?,
    })
}

fn map_message(row: &Row<'_>) -> rusqlite::Result<GroupMessage> {
    let created_at_ms = row.get::<_, i64>(4)?;
    Ok(GroupMessage {
        id: parse(row.get::<_, String>(0)?)?,
        group_id: parse(row.get::<_, String>(1)?)?,
        author: serde_json::from_str::<GroupAuthor>(&row.get::<_, String>(2)?)
            .map_err(to_sql_error)?,
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
            groups::{NewGroup, RoundSpeaker},
        },
        infrastructure::database::{AgentRepository, SqliteAgentRepository},
    };

    fn agent(database: &Arc<Database>, name: &str) -> Agent {
        let agent = Agent::create(NewAgent {
            name: name.into(),
            role: "Engineer".into(),
            description: String::new(),
            provider_id: "mock".into(),
            identity_color: IdentityColor::Indigo,
            workspace: "/workspace".into(),
            instructions: String::new(),
            model_selection: Default::default(),
        })
        .expect("agent");
        SqliteAgentRepository::new(Arc::clone(database))
            .save(&agent)
            .expect("save agent");
        agent
    }

    fn setup() -> (SqliteGroupRepository, Group, Agent, Agent) {
        let database = Arc::new(Database::in_memory().expect("database"));
        let (atlas, nova) = (agent(&database, "Atlas"), agent(&database, "Nova"));
        let group = Group::create(
            NewGroup {
                name: "Release".into(),
                topic: "Ship 0.4".into(),
                member_ids: vec![nova.id, atlas.id],
            },
            GroupAuthor::Agent { agent_id: atlas.id },
        )
        .expect("group");
        let repository = SqliteGroupRepository::new(database);
        repository.save(&group).expect("save");
        (repository, group, atlas, nova)
    }

    #[test]
    fn round_trips_groups_with_members_in_order() {
        let (repository, mut group, atlas, _) = setup();
        let found = repository.find(group.id).expect("find").expect("group");
        assert_eq!(found, group);

        group.round.enqueue(&[atlas.id], 1);
        group.round.active_wake_id = Some(Uuid::new_v4());
        group.member_ids.reverse();
        repository.save(&group).expect("update");
        let found = repository.find(group.id).expect("find").expect("group");
        assert_eq!(found.member_ids, group.member_ids);
        assert_eq!(
            found.round.queue,
            vec![RoundSpeaker {
                agent_id: atlas.id,
                chain_depth: 1
            }]
        );
        assert_eq!(repository.list().expect("list"), vec![found]);

        assert!(repository.delete(group.id).expect("delete"));
        assert!(!repository.delete(group.id).expect("delete again"));
        assert!(repository.list().expect("list").is_empty());
    }

    #[test]
    fn lists_messages_and_those_an_agent_has_not_seen() {
        let (repository, group, atlas, nova) = setup();
        let post = |author: GroupAuthor, content: &str| {
            let message = GroupMessage::new(group.id, author, content).expect("message");
            repository.append_message(&message).expect("append");
            message
        };
        let kickoff = post(GroupAuthor::User, "Kickoff");
        assert_eq!(
            repository
                .messages_since_last_from(group.id, atlas.id, 10)
                .expect("unseen"),
            vec![kickoff.clone()]
        );
        let reply = post(GroupAuthor::Agent { agent_id: atlas.id }, "On it");
        let review = post(GroupAuthor::Agent { agent_id: nova.id }, "Reviewed");
        let notice = post(GroupAuthor::System, "Stopped.");

        assert_eq!(
            repository
                .messages_since_last_from(group.id, atlas.id, 10)
                .expect("unseen"),
            vec![review.clone(), notice.clone()]
        );
        assert_eq!(
            repository
                .messages_since_last_from(group.id, atlas.id, 1)
                .expect("unseen"),
            vec![notice.clone()]
        );
        assert_eq!(
            repository.list_messages(group.id, 10).expect("list"),
            vec![kickoff, reply, review.clone(), notice.clone()]
        );
        assert_eq!(
            repository.list_messages(group.id, 2).expect("list"),
            vec![review, notice]
        );
    }
}
