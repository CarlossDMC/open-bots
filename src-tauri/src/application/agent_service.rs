use std::sync::Arc;

use serde_json::json;
use uuid::Uuid;

use super::provider_service::require;
use crate::{
    domain::{
        agents::{Agent, McpServerSelection, ModelSelection, NewAgent},
        events::{DomainEvent, EventType},
    },
    error::{AppError, AppResult},
    infrastructure::database::{AgentRepository, EventRepository},
    providers::{ProviderCapability, ProviderRegistry},
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
        let provider = self.providers.get(&input.provider_id)?;
        let agent = Agent::create(input)?;
        if !agent.model_selection.is_provider_default() {
            require(
                provider.as_ref(),
                ProviderCapability::ModelSelection,
                "model selection",
            )?;
        }
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

    /// Applies from the agent's next turn; the provider session is kept.
    pub fn update_model(
        &self,
        agent_id: Uuid,
        model: Option<String>,
        reasoning_effort: Option<String>,
    ) -> AppResult<Agent> {
        let selection = ModelSelection::new(model, reasoning_effort)?;
        let mut agent = self
            .agents
            .find(agent_id)?
            .ok_or_else(|| AppError::NotFound(format!("agent {agent_id}")))?;
        if !selection.is_provider_default() {
            let provider = self.providers.get(&agent.provider_id)?;
            require(
                provider.as_ref(),
                ProviderCapability::ModelSelection,
                "model selection",
            )?;
        }
        agent.change_model(selection)?;
        self.agents.save(&agent)?;
        let event = DomainEvent::new(
            EventType::AgentUpdated,
            Some(agent.id),
            json!({
                "name": agent.name,
                "model": agent.model_selection.model(),
                "reasoningEffort": agent.model_selection.reasoning_effort(),
            }),
        );
        self.events.append(&event)?;
        self.event_bus.publish(event);
        tracing::info!(agent_id = %agent.id, "agent model updated");
        Ok(agent)
    }

    /// Applies from the agent's next turn; the provider session is kept.
    pub fn update_mcp_servers(&self, agent_id: Uuid, servers: Vec<String>) -> AppResult<Agent> {
        let selection = McpServerSelection::new(servers)?;
        let mut agent = self
            .agents
            .find(agent_id)?
            .ok_or_else(|| AppError::NotFound(format!("agent {agent_id}")))?;
        if !selection.is_empty() {
            let provider = self.providers.get(&agent.provider_id)?;
            require(
                provider.as_ref(),
                ProviderCapability::ConfiguredMcpServers,
                "configured MCP servers",
            )?;
        }
        agent.change_mcp_servers(selection)?;
        self.agents.save(&agent)?;
        let event = DomainEvent::new(
            EventType::AgentUpdated,
            Some(agent.id),
            json!({ "name": agent.name, "mcpServers": agent.mcp_servers }),
        );
        self.events.append(&event)?;
        self.event_bus.publish(event);
        tracing::info!(
            agent_id = %agent.id,
            servers = agent.mcp_servers.names().len(),
            "agent MCP servers updated"
        );
        Ok(agent)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::agents::IdentityColor,
        infrastructure::database::{
            Database, EventRepository, SqliteAgentRepository, SqliteEventRepository,
        },
        providers::MockProvider,
    };

    fn service() -> (AgentService, Arc<SqliteEventRepository>) {
        let database = Arc::new(Database::in_memory().expect("database"));
        let events = Arc::new(SqliteEventRepository::new(Arc::clone(&database)));
        let mut registry = ProviderRegistry::new();
        registry.register(Arc::new(MockProvider)).expect("mock");
        let service = AgentService::new(
            Arc::new(SqliteAgentRepository::new(database)),
            events.clone(),
            EventBus::new(8),
            Arc::new(registry),
        );
        (service, events)
    }

    fn new_agent(model: Option<&str>) -> NewAgent {
        NewAgent {
            name: "Atlas".into(),
            role: "Engineer".into(),
            description: String::new(),
            provider_id: "mock".into(),
            identity_color: IdentityColor::Indigo,
            workspace: "/tmp".into(),
            instructions: String::new(),
            model_selection: ModelSelection::new(model.map(str::to_owned), None)
                .expect("selection"),
        }
    }

    #[test]
    fn rejects_models_for_providers_without_model_selection() {
        let (service, _) = service();
        assert!(matches!(
            service.create(new_agent(Some("gpt-5.5"))),
            Err(AppError::Unsupported(_))
        ));
        let agent = service.create(new_agent(None)).expect("agent");
        assert!(matches!(
            service.update_model(agent.id, Some("gpt-5.5".into()), None),
            Err(AppError::Unsupported(_))
        ));
    }

    #[test]
    fn rejects_mcp_servers_for_providers_without_configured_servers() {
        let (service, _) = service();
        let agent = service.create(new_agent(None)).expect("agent");
        assert!(matches!(
            service.update_mcp_servers(agent.id, vec!["github".into()]),
            Err(AppError::Unsupported(_))
        ));
        let cleared = service
            .update_mcp_servers(agent.id, Vec::new())
            .expect("clearing is always allowed");
        assert!(cleared.mcp_servers.is_empty());
    }

    #[test]
    fn persists_model_changes_with_an_event() {
        let (service, events) = service();
        let agent = service.create(new_agent(None)).expect("agent");
        let updated = service
            .update_model(agent.id, None, None)
            .expect("reset to default");
        assert!(updated.model_selection.is_provider_default());
        assert_eq!(service.list().expect("list"), vec![updated]);
        let recent = events.list_recent(10).expect("events");
        assert!(recent
            .iter()
            .any(|event| event.event_type == EventType::AgentUpdated));
        assert!(matches!(
            service.update_model(Uuid::new_v4(), None, None),
            Err(AppError::NotFound(_))
        ));
    }
}
