use std::sync::Arc;

use chrono::{DateTime, Utc};
use rusqlite::{params, Row};
use uuid::Uuid;

use crate::{
    domain::routines::Routine,
    error::{AppError, AppResult},
};

use super::Database;

const SELECT_ROUTINES: &str = "SELECT id, agent_id, name, instructions, schedule_json, enabled, \
     next_run_at_ms, last_run_at, created_at, updated_at FROM routines";

pub trait RoutineRepository: Send + Sync {
    fn list_for_agent(&self, agent_id: Uuid) -> AppResult<Vec<Routine>>;
    fn find(&self, id: Uuid) -> AppResult<Option<Routine>>;
    fn count_for_agent(&self, agent_id: Uuid) -> AppResult<usize>;
    /// Enabled routines whose next run is at or before `now`.
    fn list_due(&self, now: DateTime<Utc>) -> AppResult<Vec<Routine>>;
    /// The earliest next run among enabled routines.
    fn next_due_at(&self) -> AppResult<Option<DateTime<Utc>>>;
    fn save(&self, routine: &Routine) -> AppResult<()>;
    fn delete(&self, id: Uuid) -> AppResult<()>;
}

pub struct SqliteRoutineRepository {
    database: Arc<Database>,
}

impl SqliteRoutineRepository {
    pub fn new(database: Arc<Database>) -> Self {
        Self { database }
    }
}

impl RoutineRepository for SqliteRoutineRepository {
    fn list_for_agent(&self, agent_id: Uuid) -> AppResult<Vec<Routine>> {
        self.database.with_connection(|connection| {
            let mut statement = connection.prepare(&format!(
                "{SELECT_ROUTINES} WHERE agent_id = ?1 ORDER BY created_at"
            ))?;
            let routines = statement
                .query_map([agent_id.to_string()], map_routine)?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(routines)
        })
    }

    fn find(&self, id: Uuid) -> AppResult<Option<Routine>> {
        self.database.with_connection(|connection| {
            let mut statement = connection.prepare(&format!("{SELECT_ROUTINES} WHERE id = ?1"))?;
            let mut rows = statement.query([id.to_string()])?;
            rows.next()?
                .map(map_routine)
                .transpose()
                .map_err(AppError::from)
        })
    }

    fn count_for_agent(&self, agent_id: Uuid) -> AppResult<usize> {
        self.database.with_connection(|connection| {
            let count: i64 = connection.query_row(
                "SELECT COUNT(*) FROM routines WHERE agent_id = ?1",
                [agent_id.to_string()],
                |row| row.get(0),
            )?;
            usize::try_from(count)
                .map_err(|_| AppError::Validation("routine count is out of range".into()))
        })
    }

    fn list_due(&self, now: DateTime<Utc>) -> AppResult<Vec<Routine>> {
        self.database.with_connection(|connection| {
            let mut statement = connection.prepare(&format!(
                "{SELECT_ROUTINES} WHERE enabled = 1 AND next_run_at_ms <= ?1 ORDER BY next_run_at_ms"
            ))?;
            let routines = statement
                .query_map([now.timestamp_millis()], map_routine)?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(routines)
        })
    }

    fn next_due_at(&self) -> AppResult<Option<DateTime<Utc>>> {
        self.database.with_connection(|connection| {
            let next: Option<i64> = connection.query_row(
                "SELECT MIN(next_run_at_ms) FROM routines WHERE enabled = 1",
                [],
                |row| row.get(0),
            )?;
            Ok(next.and_then(DateTime::from_timestamp_millis))
        })
    }

    fn save(&self, routine: &Routine) -> AppResult<()> {
        self.database.with_connection(|connection| {
            connection.execute(
                "INSERT INTO routines (id, agent_id, name, instructions, schedule_json, enabled, \
                 next_run_at_ms, last_run_at, created_at, updated_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10) \
                 ON CONFLICT(id) DO UPDATE SET name=excluded.name, instructions=excluded.instructions, \
                 schedule_json=excluded.schedule_json, enabled=excluded.enabled, \
                 next_run_at_ms=excluded.next_run_at_ms, last_run_at=excluded.last_run_at, \
                 updated_at=excluded.updated_at",
                params![
                    routine.id.to_string(),
                    routine.agent_id.to_string(),
                    routine.name,
                    routine.instructions,
                    serde_json::to_string(&routine.schedule)?,
                    routine.enabled,
                    routine.next_run_at.timestamp_millis(),
                    routine.last_run_at.map(|at| at.to_rfc3339()),
                    routine.created_at.to_rfc3339(),
                    routine.updated_at.to_rfc3339(),
                ],
            )?;
            Ok(())
        })
    }

    fn delete(&self, id: Uuid) -> AppResult<()> {
        self.database.with_connection(|connection| {
            connection.execute("DELETE FROM routines WHERE id = ?1", [id.to_string()])?;
            Ok(())
        })
    }
}

fn map_routine(row: &Row<'_>) -> rusqlite::Result<Routine> {
    let next_run_at_ms = row.get::<_, i64>(6)?;
    Ok(Routine {
        id: parse(row.get::<_, String>(0)?)?,
        agent_id: parse(row.get::<_, String>(1)?)?,
        name: row.get(2)?,
        instructions: row.get(3)?,
        schedule: serde_json::from_str(&row.get::<_, String>(4)?).map_err(to_sql_error)?,
        enabled: row.get(5)?,
        next_run_at: DateTime::from_timestamp_millis(next_run_at_ms)
            .ok_or_else(|| rusqlite::Error::IntegralValueOutOfRange(6, next_run_at_ms))?,
        last_run_at: row.get::<_, Option<String>>(7)?.map(parse).transpose()?,
        created_at: parse(row.get::<_, String>(8)?)?,
        updated_at: parse(row.get::<_, String>(9)?)?,
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
            routines::{NewRoutine, RoutineSchedule},
        },
        infrastructure::database::{AgentRepository, SqliteAgentRepository},
    };

    fn at(value: &str) -> DateTime<Utc> {
        value.parse().expect("timestamp")
    }

    #[test]
    fn persists_routines_and_finds_due_ones() {
        let database = Arc::new(Database::in_memory().expect("database"));
        let agent = Agent::create(NewAgent {
            name: "Orbit".into(),
            role: "Release Manager".into(),
            description: String::new(),
            provider_id: "mock".into(),
            identity_color: IdentityColor::Violet,
            workspace: "/workspace".into(),
            instructions: String::new(),
            model_selection: Default::default(),
        })
        .expect("agent");
        SqliteAgentRepository::new(Arc::clone(&database))
            .save(&agent)
            .expect("save agent");
        let repository = SqliteRoutineRepository::new(database);
        let created = at("2026-01-10T10:00:00Z");
        let new_routine = |name: &str, minutes| NewRoutine {
            agent_id: agent.id,
            name: name.into(),
            instructions: "Check the release queue".into(),
            schedule: RoutineSchedule::Interval { minutes },
        };
        let soon = Routine::create(new_routine("Soon", 10), 0, created, &Utc).expect("routine");
        let mut paused =
            Routine::create(new_routine("Paused", 5), 1, created, &Utc).expect("routine");
        paused.set_enabled(false, created, &Utc);
        repository.save(&soon).expect("save");
        repository.save(&paused).expect("save");

        assert_eq!(repository.find(soon.id).expect("find"), Some(soon.clone()));
        assert_eq!(repository.count_for_agent(agent.id).expect("count"), 2);
        assert_eq!(repository.list_for_agent(agent.id).expect("list").len(), 2);
        assert_eq!(
            repository.next_due_at().expect("next"),
            Some(soon.next_run_at)
        );
        assert!(repository
            .list_due(at("2026-01-10T10:09:00Z"))
            .expect("due")
            .is_empty());
        assert_eq!(
            repository
                .list_due(at("2026-01-10T10:10:00Z"))
                .expect("due"),
            vec![soon.clone()]
        );

        repository.delete(soon.id).expect("delete");
        assert_eq!(repository.next_due_at().expect("next"), None);
    }
}
