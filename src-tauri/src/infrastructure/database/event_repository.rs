use std::sync::Arc;

use crate::{domain::events::DomainEvent, error::AppResult};

use super::Database;

pub trait EventRepository: Send + Sync {
    fn append(&self, event: &DomainEvent) -> AppResult<()>;
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
}
