use std::sync::Arc;

use rusqlite::{params, Row};
use uuid::Uuid;

use crate::{
    domain::approvals::ApprovalRequest,
    error::{AppError, AppResult},
};

use super::Database;

const SELECT_APPROVALS: &str =
    "SELECT id, agent_id, action, reason, status, created_at, resolved_at FROM approvals";

pub trait ApprovalRepository: Send + Sync {
    fn list(&self) -> AppResult<Vec<ApprovalRequest>>;
    fn find(&self, id: Uuid) -> AppResult<Option<ApprovalRequest>>;
    fn save(&self, approval: &ApprovalRequest) -> AppResult<()>;
}

pub struct SqliteApprovalRepository {
    database: Arc<Database>,
}

impl SqliteApprovalRepository {
    pub fn new(database: Arc<Database>) -> Self {
        Self { database }
    }
}

impl ApprovalRepository for SqliteApprovalRepository {
    fn list(&self) -> AppResult<Vec<ApprovalRequest>> {
        self.database.with_connection(|connection| {
            let mut statement =
                connection.prepare(&format!("{SELECT_APPROVALS} ORDER BY created_at DESC"))?;
            let approvals = statement
                .query_map([], map_approval)?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(approvals)
        })
    }

    fn find(&self, id: Uuid) -> AppResult<Option<ApprovalRequest>> {
        self.database.with_connection(|connection| {
            let mut statement = connection.prepare(&format!("{SELECT_APPROVALS} WHERE id = ?1"))?;
            let mut rows = statement.query([id.to_string()])?;
            rows.next()?
                .map(map_approval)
                .transpose()
                .map_err(AppError::from)
        })
    }

    fn save(&self, approval: &ApprovalRequest) -> AppResult<()> {
        self.database.with_connection(|connection| {
            connection.execute(
                "INSERT INTO approvals (id, agent_id, action, reason, status, created_at, resolved_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7) \
                 ON CONFLICT(id) DO UPDATE SET status=excluded.status, resolved_at=excluded.resolved_at",
                params![
                    approval.id.to_string(),
                    approval.agent_id.to_string(),
                    approval.action,
                    approval.reason,
                    serde_json::to_string(&approval.status)?.trim_matches('"'),
                    approval.created_at.to_rfc3339(),
                    approval.resolved_at.map(|at| at.to_rfc3339()),
                ],
            )?;
            Ok(())
        })
    }
}

fn map_approval(row: &Row<'_>) -> rusqlite::Result<ApprovalRequest> {
    let status = row.get::<_, String>(4)?;
    Ok(ApprovalRequest {
        id: parse(row.get::<_, String>(0)?)?,
        agent_id: parse(row.get::<_, String>(1)?)?,
        action: row.get(2)?,
        reason: row.get(3)?,
        status: serde_json::from_str(&format!("\"{status}\"")).map_err(to_sql_error)?,
        created_at: parse(row.get::<_, String>(5)?)?,
        resolved_at: row.get::<_, Option<String>>(6)?.map(parse).transpose()?,
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
            approvals::ApprovalStatus,
        },
        infrastructure::database::{AgentRepository, SqliteAgentRepository},
    };

    fn saved_agent(database: &Arc<Database>) -> Agent {
        let agent = Agent::create(NewAgent {
            name: "Atlas".into(),
            role: "Backend Engineer".into(),
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

    #[test]
    fn persists_and_resolves_approvals() {
        let database = Arc::new(Database::in_memory().expect("database"));
        let agent = saved_agent(&database);
        let repository = SqliteApprovalRepository::new(database);
        let mut approval =
            ApprovalRequest::create(agent.id, "git push", "Ship the fix").expect("approval");
        repository.save(&approval).expect("save");
        assert_eq!(
            repository.find(approval.id).expect("find"),
            Some(approval.clone())
        );

        approval.resolve(ApprovalStatus::Denied).expect("resolve");
        repository.save(&approval).expect("update");
        assert_eq!(repository.list().expect("list"), vec![approval]);
    }
}
