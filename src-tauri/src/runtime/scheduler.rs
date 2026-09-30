use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::error::AppResult;

#[derive(Debug, Clone)]
pub struct ScheduledWake {
    pub agent_id: Uuid,
    pub wake_at: DateTime<Utc>,
    pub reason: String,
}

#[async_trait]
pub trait Scheduler: Send + Sync {
    async fn schedule(&self, wake: ScheduledWake) -> AppResult<()>;
    async fn cancel(&self, agent_id: Uuid) -> AppResult<()>;
}
