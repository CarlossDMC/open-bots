use std::sync::Arc;

use chrono::{DateTime, Local, SubsecRound, Utc};
use serde_json::json;
use tokio::sync::Notify;
use uuid::Uuid;

use crate::{
    domain::{
        events::{DomainEvent, EventType},
        routines::{NewRoutine, Routine},
    },
    error::{AppError, AppResult},
    infrastructure::database::{AgentRepository, EventRepository, RoutineRepository},
    runtime::event_bus::EventBus,
};

pub struct RoutineService {
    routines: Arc<dyn RoutineRepository>,
    agents: Arc<dyn AgentRepository>,
    events: Arc<dyn EventRepository>,
    event_bus: EventBus,
    changes: Arc<Notify>,
}

impl RoutineService {
    pub fn new(
        routines: Arc<dyn RoutineRepository>,
        agents: Arc<dyn AgentRepository>,
        events: Arc<dyn EventRepository>,
        event_bus: EventBus,
    ) -> Self {
        Self {
            routines,
            agents,
            events,
            event_bus,
            changes: Arc::new(Notify::new()),
        }
    }

    /// Signalled whenever a change may move the next due time.
    pub fn changes(&self) -> Arc<Notify> {
        Arc::clone(&self.changes)
    }

    pub fn list(&self, agent_id: Uuid) -> AppResult<Vec<Routine>> {
        self.routines.list_for_agent(agent_id)
    }

    pub fn create(&self, input: NewRoutine) -> AppResult<Routine> {
        if self.agents.find(input.agent_id)?.is_none() {
            return Err(AppError::NotFound(format!("agent {}", input.agent_id)));
        }
        let existing = self.routines.count_for_agent(input.agent_id)?;
        let routine = Routine::create(input, existing, now(), &Local)?;
        self.routines.save(&routine)?;
        self.publish(EventType::RoutineCreated, &routine, json!({}))?;
        self.changes.notify_one();
        tracing::info!(routine_id = %routine.id, agent_id = %routine.agent_id, "routine created");
        Ok(routine)
    }

    pub fn set_enabled(&self, id: Uuid, enabled: bool) -> AppResult<Routine> {
        let mut routine = self.find(id)?;
        routine.set_enabled(enabled, now(), &Local);
        self.routines.save(&routine)?;
        self.publish(
            EventType::RoutineUpdated,
            &routine,
            json!({ "enabled": enabled }),
        )?;
        self.changes.notify_one();
        Ok(routine)
    }

    pub fn delete(&self, id: Uuid) -> AppResult<()> {
        let routine = self.find(id)?;
        self.routines.delete(id)?;
        self.publish(EventType::RoutineDeleted, &routine, json!({}))?;
        self.changes.notify_one();
        tracing::info!(routine_id = %id, "routine deleted");
        Ok(())
    }

    pub fn next_due_at(&self) -> AppResult<Option<DateTime<Utc>>> {
        self.routines.next_due_at()
    }

    /// Records a run for every routine due at `now` and publishes `routine.triggered`.
    /// The agent runtime turns the event into a wake that runs the instructions.
    pub fn fire_due(&self, now: DateTime<Utc>) -> AppResult<Vec<Routine>> {
        let mut fired = Vec::new();
        for mut routine in self.routines.list_due(now)? {
            let scheduled_for = routine.next_run_at;
            routine.record_run(now, &Local);
            self.routines.save(&routine)?;
            self.publish(
                EventType::RoutineTriggered,
                &routine,
                json!({ "scheduledFor": scheduled_for }),
            )?;
            tracing::info!(routine_id = %routine.id, agent_id = %routine.agent_id, "routine triggered");
            fired.push(routine);
        }
        Ok(fired)
    }

    fn find(&self, id: Uuid) -> AppResult<Routine> {
        self.routines
            .find(id)?
            .ok_or_else(|| AppError::NotFound(format!("routine {id}")))
    }

    fn publish(
        &self,
        event_type: EventType,
        routine: &Routine,
        mut payload: serde_json::Value,
    ) -> AppResult<()> {
        // Instructions stay out of the payload; the timeline only needs identity.
        payload["agentId"] = json!(routine.agent_id);
        payload["name"] = json!(routine.name);
        let event = DomainEvent::new(event_type, Some(routine.id), payload);
        self.events.append(&event)?;
        self.event_bus.publish(event);
        Ok(())
    }
}

/// Whole seconds keep in-memory values equal to their persisted form.
pub(crate) fn now() -> DateTime<Utc> {
    Utc::now().trunc_subsecs(0)
}
