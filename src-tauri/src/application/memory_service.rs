use std::sync::Arc;

use serde_json::json;
use uuid::Uuid;

use crate::{
    domain::{
        events::{DomainEvent, EventType},
        memories::AgentMemory,
    },
    error::{AppError, AppResult},
    infrastructure::database::{AgentRepository, EventRepository, MemoryRepository},
    runtime::event_bus::EventBus,
};

pub struct MemoryService {
    memories: Arc<dyn MemoryRepository>,
    agents: Arc<dyn AgentRepository>,
    events: Arc<dyn EventRepository>,
    event_bus: EventBus,
}

impl MemoryService {
    pub fn new(
        memories: Arc<dyn MemoryRepository>,
        agents: Arc<dyn AgentRepository>,
        events: Arc<dyn EventRepository>,
        event_bus: EventBus,
    ) -> Self {
        Self {
            memories,
            agents,
            events,
            event_bus,
        }
    }

    pub fn list(&self, agent_id: Uuid) -> AppResult<Vec<AgentMemory>> {
        self.memories.list_for_agent(agent_id)
    }

    pub fn add(&self, agent_id: Uuid, content: &str) -> AppResult<AgentMemory> {
        if self.agents.find(agent_id)?.is_none() {
            return Err(AppError::NotFound(format!("agent {agent_id}")));
        }
        let memory = AgentMemory::create(agent_id, content)?;
        self.memories.save(&memory)?;
        self.publish(EventType::MemoryAdded, &memory)?;
        tracing::info!(memory_id = %memory.id, agent_id = %agent_id, "agent memory added");
        Ok(memory)
    }

    pub fn remove(&self, id: Uuid) -> AppResult<()> {
        let memory = self
            .memories
            .find(id)?
            .ok_or_else(|| AppError::NotFound(format!("memory {id}")))?;
        self.memories.delete(id)?;
        self.publish(EventType::MemoryRemoved, &memory)?;
        tracing::info!(memory_id = %id, agent_id = %memory.agent_id, "agent memory removed");
        Ok(())
    }

    fn publish(&self, event_type: EventType, memory: &AgentMemory) -> AppResult<()> {
        // Memory content stays out of the event payload so the timeline never duplicates it.
        let event = DomainEvent::new(
            event_type,
            Some(memory.id),
            json!({ "agentId": memory.agent_id }),
        );
        self.events.append(&event)?;
        self.event_bus.publish(event);
        Ok(())
    }
}
