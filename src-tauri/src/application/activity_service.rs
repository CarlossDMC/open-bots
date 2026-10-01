use std::sync::Arc;

use crate::{
    domain::events::DomainEvent, error::AppResult, infrastructure::database::EventRepository,
};

pub const MAX_ACTIVITY_EVENTS: usize = 500;

pub struct ActivityService {
    events: Arc<dyn EventRepository>,
}

impl ActivityService {
    pub fn new(events: Arc<dyn EventRepository>) -> Self {
        Self { events }
    }

    /// Returns the newest persisted events, capped so the timeline stays bounded.
    pub fn recent(&self, limit: usize) -> AppResult<Vec<DomainEvent>> {
        self.events.list_recent(limit.clamp(1, MAX_ACTIVITY_EVENTS))
    }
}
