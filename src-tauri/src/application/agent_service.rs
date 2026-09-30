use std::sync::Arc;

use serde_json::json;

use crate::{
    domain::{
        agents::{Agent, NewAgent},
        events::{DomainEvent, EventType},
    },
    error::AppResult,
    infrastructure::database::{AgentRepository, EventRepository},
    providers::ProviderRegistry,
    runtime::event_bus::EventBus,
};

pub struct AgentService {
    agents: Arc<dyn AgentRepository>,
    events: Arc<dyn EventRepository>,
    event_bus: EventBus,
    providers: Arc<ProviderRegistry>,
}

impl AgentService {
    pub fn new(
        agents: Arc<dyn AgentRepository>,
        events: Arc<dyn EventRepository>,
        event_bus: EventBus,
        providers: Arc<ProviderRegistry>,
    ) -> Self {
        Self {
            agents,
            events,
            event_bus,
            providers,
        }
    }

    pub fn list(&self) -> AppResult<Vec<Agent>> {
        self.agents.list()
    }

    pub fn create(&self, input: NewAgent) -> AppResult<Agent> {
        self.providers.get(&input.provider_id)?;
        let agent = Agent::create(input)?;
        self.agents.save(&agent)?;
        let event = DomainEvent::new(
            EventType::AgentCreated,
            Some(agent.id),
            json!({ "name": agent.name, "providerId": agent.provider_id }),
        );
        self.events.append(&event)?;
        self.event_bus.publish(event);
        tracing::info!(agent_id = %agent.id, provider_id = %agent.provider_id, "agent created");
        Ok(agent)
    }
}
