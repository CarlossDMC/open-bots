use std::sync::Arc;

use chrono::DateTime;
use rusqlite::{params, Row};
use uuid::Uuid;

use crate::{
    domain::inbox::Wake,
    error::{AppError, AppResult},
};

use super::Database;

const SELECT_WAKES: &str = "SELECT id, agent_id, origin_json, content, chain_depth, \
     created_at_ms, consumed_at, outcome FROM agent_wakes";

pub trait WakeRepository: Send + Sync {
    fn enqueue(&self, wake: &Wake) -> AppResult<()>;
    /// The oldest wake the agent has not handled yet.
    fn next_pending(&self, agent_id: Uuid) -> AppResult<Option<Wake>>;
    /// Agents with at least one pending wake.
    fn agents_with_pending(&self) -> AppResult<Vec<Uuid>>;
    /// Stores the outcome of a consumed wake.
    fn save_outcome(&self, wake: &Wake) -> AppResult<()>;
}

pub struct SqliteWakeRepository {
    database: Arc<Database>,
}

impl SqliteWakeRepository {
    pub fn new(database: Arc<Database>) -> Self {
        Self { database }
    }
}

impl WakeRepository for SqliteWakeRepository {
    fn enqueue(&self, wake: &Wake) -> AppResult<()> {
        self.database.with_connection(|connection| {
            connection.execute(
                "INSERT INTO agent_wakes (id, agent_id, origin_json, content, chain_depth, \
                 created_at_ms, sequence, consumed_at, outcome) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, \
                   (SELECT COALESCE(MAX(sequence), 0) + 1 FROM agent_wakes WHERE agent_id = ?2), \
                   NULL, NULL)",
                params![
                    wake.id.to_string(),
                    wake.agent_id.to_string(),
                    serde_json::to_string(&wake.origin)?,
                    wake.content,
                    wake.chain_depth,
                    wake.created_at.timestamp_millis(),
                ],
            )?;
            Ok(())
        })
    }

    fn next_pending(&self, agent_id: Uuid) -> AppResult<Option<Wake>> {
        self.database.with_connection(|connection| {
            let mut statement = connection.prepare(&format!(
                "{SELECT_WAKES} WHERE agent_id = ?1 AND consumed_at IS NULL \
                 ORDER BY created_at_ms, sequence LIMIT 1"
            ))?;
            let mut rows = statement.query([agent_id.to_string()])?;
            rows.next()?
                .map(map_wake)
                .transpose()
                .map_err(AppError::from)
        })
    }

    fn agents_with_pending(&self) -> AppResult<Vec<Uuid>> {
        self.database.with_connection(|connection| {
            let mut statement = connection
                .prepare("SELECT DISTINCT agent_id FROM agent_wakes WHERE consumed_at IS NULL")?;
            let agents = statement
                .query_map([], |row| parse(row.get::<_, String>(0)?))?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(agents)
        })
    }

    fn save_outcome(&self, wake: &Wake) -> AppResult<()> {
        let outcome = wake
            .outcome
            .map(serde_json::to_value)
            .transpose()?
            .and_then(|value| value.as_str().map(str::to_owned));
        self.database.with_connection(|connection| {
            connection.execute(
                "UPDATE agent_wakes SET consumed_at = ?2, outcome = ?3 WHERE id = ?1",
                params![
                    wake.id.to_string(),
                    wake.consumed_at.map(|at| at.to_rfc3339()),
                    outcome,
                ],
            )?;
            Ok(())
        })
    }
}

fn map_wake(row: &Row<'_>) -> rusqlite::Result<Wake> {
    let created_at_ms = row.get::<_, i64>(5)?;
    Ok(Wake {
        id: parse(row.get::<_, String>(0)?)?,
        agent_id: parse(row.get::<_, String>(1)?)?,
        origin: serde_json::from_str(&row.get::<_, String>(2)?).map_err(to_sql_error)?,
        content: row.get(3)?,
        chain_depth: row.get(4)?,
        created_at: DateTime::from_timestamp_millis(created_at_ms)
            .ok_or_else(|| rusqlite::Error::IntegralValueOutOfRange(5, created_at_ms))?,
        consumed_at: row.get::<_, Option<String>>(6)?.map(parse).transpose()?,
        outcome: row
            .get::<_, Option<String>>(7)?
            .map(|name| serde_json::from_value(serde_json::Value::String(name)))
            .transpose()
            .map_err(to_sql_error)?,
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
    use chrono::{SubsecRound, Utc};

    use super::*;
    use crate::{
        domain::{
            agents::{Agent, IdentityColor, NewAgent},
            inbox::{WakeOrigin, WakeOutcome},
        },
        infrastructure::database::{AgentRepository, SqliteAgentRepository},
    };

    #[test]
    fn queues_wakes_in_arrival_order_until_consumed() {
        let database = Arc::new(Database::in_memory().expect("database"));
        let agent = Agent::create(NewAgent {
            name: "Atlas".into(),
            role: "Engineer".into(),
            description: String::new(),
            provider_id: "mock".into(),
            identity_color: IdentityColor::Indigo,
            workspace: "/workspace".into(),
            instructions: String::new(),
            model_selection: Default::default(),
        })
        .expect("agent");
        SqliteAgentRepository::new(Arc::clone(&database))
            .save(&agent)
            .expect("save agent");
        let repository = SqliteWakeRepository::new(database);
        let now = Utc::now().trunc_subsecs(3);
        let wake = |name: &str| {
            Wake::new(
                agent.id,
                WakeOrigin::Routine {
                    routine_id: Uuid::new_v4(),
                    name: name.into(),
                },
                "Do the thing",
                1,
                now,
            )
        };
        let mut first = wake("first");
        let second = wake("second");
        repository.enqueue(&first).expect("enqueue");
        repository.enqueue(&second).expect("enqueue");

        assert_eq!(
            repository.next_pending(agent.id).expect("next"),
            Some(first.clone())
        );
        assert_eq!(
            repository.agents_with_pending().expect("pending"),
            vec![agent.id]
        );
        first.consume(WakeOutcome::Started, now).expect("consume");
        repository.save_outcome(&first).expect("save");
        assert_eq!(
            repository.next_pending(agent.id).expect("next"),
            Some(second)
        );
    }
}
