use std::sync::Arc;

use rusqlite::Row;

use crate::{
    domain::events::DomainEvent,
    error::{AppError, AppResult},
};

use super::Database;

pub trait EventRepository: Send + Sync {
    fn append(&self, event: &DomainEvent) -> AppResult<()>;
    fn list_recent(&self, limit: usize) -> AppResult<Vec<DomainEvent>>;
}

pub struct SqliteEventRepository {
    database: Arc<Database>,
}

impl SqliteEventRepository {
    pub fn new(database: Arc<Database>) -> Self {
        Self { database }
    }
}

impl EventRepository for SqliteEventRepository {
    fn append(&self, event: &DomainEvent) -> AppResult<()> {
        self.database.with_connection(|connection| {
            connection.execute("INSERT INTO events (id, event_type, aggregate_id, payload_json, occurred_at) VALUES (?1, ?2, ?3, ?4, ?5)", rusqlite::params![event.id.to_string(), serde_json::to_string(&event.event_type)?.trim_matches('"'), event.aggregate_id.map(|id| id.to_string()), serde_json::to_string(&event.payload)?, event.occurred_at.to_rfc3339()])?;
            Ok(())
        })
    }

    fn list_recent(&self, limit: usize) -> AppResult<Vec<DomainEvent>> {
        let limit = i64::try_from(limit)
            .map_err(|_| AppError::Validation("event limit is too large".into()))?;
        self.database.with_connection(|connection| {
            let mut statement = connection.prepare(
                "SELECT id, event_type, aggregate_id, payload_json, occurred_at \
                 FROM events ORDER BY occurred_at DESC LIMIT ?1",
            )?;
            let events = statement
                .query_map([limit], map_event)?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(events)
        })
    }
}

fn map_event(row: &Row<'_>) -> rusqlite::Result<DomainEvent> {
    let event_type = row.get::<_, String>(1)?;
    let aggregate_id = row.get::<_, Option<String>>(2)?;
    Ok(DomainEvent {
        id: parse(row.get::<_, String>(0)?)?,
        event_type: serde_json::from_str(&format!("\"{event_type}\"")).map_err(to_sql_error)?,
        aggregate_id: aggregate_id.map(parse).transpose()?,
        payload: serde_json::from_str(&row.get::<_, String>(3)?).map_err(to_sql_error)?,
        occurred_at: parse(row.get::<_, String>(4)?)?,
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
