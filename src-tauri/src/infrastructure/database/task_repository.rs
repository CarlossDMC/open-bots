use std::sync::Arc;

use rusqlite::{params, Row};
use uuid::Uuid;

use crate::{
    domain::tasks::{Task, TaskStatus},
    error::{AppError, AppResult},
};

use super::Database;

const SELECT_TASKS: &str = "SELECT id, title, description, status, assigned_agent_id, \
     created_by_agent_id, parent_task_id, workspace_id, result, created_at, updated_at, \
     started_at, completed_at FROM tasks";

pub trait TaskRepository: Send + Sync {
    /// Every task, newest first.
    fn list(&self) -> AppResult<Vec<Task>>;
    /// Tasks assigned to or created by the agent, newest first.
    fn list_for_agent(&self, agent_id: Uuid) -> AppResult<Vec<Task>>;
    fn find(&self, id: Uuid) -> AppResult<Option<Task>>;
    fn save(&self, task: &Task) -> AppResult<()>;
}

pub struct SqliteTaskRepository {
    database: Arc<Database>,
}

impl SqliteTaskRepository {
    pub fn new(database: Arc<Database>) -> Self {
        Self { database }
    }
}

impl TaskRepository for SqliteTaskRepository {
    fn list(&self) -> AppResult<Vec<Task>> {
        self.database.with_connection(|connection| {
            let mut statement =
                connection.prepare(&format!("{SELECT_TASKS} ORDER BY created_at DESC"))?;
            let tasks = statement
                .query_map([], map_task)?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(tasks)
        })
    }

    fn list_for_agent(&self, agent_id: Uuid) -> AppResult<Vec<Task>> {
        self.database.with_connection(|connection| {
            let mut statement = connection.prepare(&format!(
                "{SELECT_TASKS} WHERE assigned_agent_id = ?1 OR created_by_agent_id = ?1 \
                 ORDER BY created_at DESC"
            ))?;
            let tasks = statement
                .query_map([agent_id.to_string()], map_task)?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(tasks)
        })
    }

    fn find(&self, id: Uuid) -> AppResult<Option<Task>> {
        self.database.with_connection(|connection| {
            let mut statement = connection.prepare(&format!("{SELECT_TASKS} WHERE id = ?1"))?;
            let mut rows = statement.query([id.to_string()])?;
            rows.next()?
                .map(map_task)
                .transpose()
                .map_err(AppError::from)
        })
    }

    fn save(&self, task: &Task) -> AppResult<()> {
        self.database.with_connection(|connection| {
            connection.execute(
                "INSERT INTO tasks (id, title, description, status, assigned_agent_id, \
                 created_by_agent_id, parent_task_id, workspace_id, result, created_at, \
                 updated_at, started_at, completed_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13) \
                 ON CONFLICT(id) DO UPDATE SET title=excluded.title, \
                 description=excluded.description, status=excluded.status, \
                 assigned_agent_id=excluded.assigned_agent_id, result=excluded.result, \
                 updated_at=excluded.updated_at, started_at=excluded.started_at, \
                 completed_at=excluded.completed_at",
                params![
                    task.id.to_string(),
                    task.title,
                    task.description,
                    status_name(task.status)?,
                    task.assigned_agent_id.map(|id| id.to_string()),
                    task.created_by_agent_id.map(|id| id.to_string()),
                    task.parent_task_id.map(|id| id.to_string()),
                    task.workspace_id,
                    task.result,
                    task.created_at.to_rfc3339(),
                    task.updated_at.to_rfc3339(),
                    task.started_at.map(|at| at.to_rfc3339()),
                    task.completed_at.map(|at| at.to_rfc3339()),
                ],
            )?;
            Ok(())
        })
    }
}

fn status_name(status: TaskStatus) -> AppResult<String> {
    match serde_json::to_value(status)? {
        serde_json::Value::String(name) => Ok(name),
        _ => Err(AppError::Validation("task status is not a string".into())),
    }
}

fn map_task(row: &Row<'_>) -> rusqlite::Result<Task> {
    let optional_id = |index| -> rusqlite::Result<Option<Uuid>> {
        row.get::<_, Option<String>>(index)?.map(parse).transpose()
    };
    Ok(Task {
        id: parse(row.get::<_, String>(0)?)?,
        title: row.get(1)?,
        description: row.get(2)?,
        status: serde_json::from_value(serde_json::Value::String(row.get(3)?))
            .map_err(to_sql_error)?,
        assigned_agent_id: optional_id(4)?,
        created_by_agent_id: optional_id(5)?,
        parent_task_id: optional_id(6)?,
        workspace_id: row.get(7)?,
        result: row.get(8)?,
        created_at: parse(row.get::<_, String>(9)?)?,
        updated_at: parse(row.get::<_, String>(10)?)?,
        started_at: row.get::<_, Option<String>>(11)?.map(parse).transpose()?,
        completed_at: row.get::<_, Option<String>>(12)?.map(parse).transpose()?,
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
            tasks::NewTask,
        },
        infrastructure::database::{AgentRepository, SqliteAgentRepository},
    };

    fn agent(database: &Arc<Database>, name: &str) -> Agent {
        let agent = Agent::create(NewAgent {
            name: name.into(),
            role: "Engineer".into(),
            description: String::new(),
            provider_id: "mock".into(),
            identity_color: IdentityColor::Violet,
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

    #[test]
    fn round_trips_tasks_and_filters_by_agent() {
        let database = Arc::new(Database::in_memory().expect("database"));
        let atlas = agent(&database, "Atlas");
        let nova = agent(&database, "Nova");
        let repository = SqliteTaskRepository::new(Arc::clone(&database));
        let now = Utc::now().trunc_subsecs(0);

        let mut delegated = Task::create(
            NewTask {
                title: "Review the API".into(),
                description: "Check the contract".into(),
                assigned_agent_id: Some(nova.id),
                created_by_agent_id: Some(atlas.id),
                parent_task_id: None,
                workspace_id: "/workspace".into(),
            },
            now,
        )
        .expect("task");
        repository.save(&delegated).expect("save");
        let unrelated = Task::create(
            NewTask {
                title: "Unassigned".into(),
                description: String::new(),
                assigned_agent_id: None,
                created_by_agent_id: None,
                parent_task_id: None,
                workspace_id: String::new(),
            },
            now,
        )
        .expect("task");
        repository.save(&unrelated).expect("save");

        delegated
            .update_status(TaskStatus::Completed, Some("Looks good".into()), now)
            .expect("complete");
        repository.save(&delegated).expect("update");

        assert_eq!(
            repository.find(delegated.id).expect("find"),
            Some(delegated.clone())
        );
        assert_eq!(repository.list().expect("list").len(), 2);
        assert_eq!(
            repository.list_for_agent(atlas.id).expect("for atlas"),
            vec![delegated.clone()]
        );
        assert_eq!(
            repository.list_for_agent(nova.id).expect("for nova"),
            vec![delegated]
        );
    }
}
