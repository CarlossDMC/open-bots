use std::sync::Arc;

use serde_json::json;
use uuid::Uuid;

use super::provider_service::require;
use crate::{
    domain::{
        agents::{Agent, AgentStatus, McpServerSelection, ModelSelection, NewAgent},
        events::{DomainEvent, EventType},
    },
    error::{AppError, AppResult},
    infrastructure::database::{AgentRepository, EventRepository, McpCatalogRepository},
    providers::{ProviderCapability, ProviderRegistry},
    runtime::event_bus::EventBus,
};

pub struct AgentService {
    agents: Arc<dyn AgentRepository>,
    events: Arc<dyn EventRepository>,
    event_bus: EventBus,
    providers: Arc<ProviderRegistry>,
    mcp_catalog: Option<Arc<dyn McpCatalogRepository>>,
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
            mcp_catalog: None,
        }
    }

    /// Lets agents select MCP servers from the global catalog. Without it, only an empty
    /// selection is accepted.
    pub fn with_mcp_catalog(mut self, catalog: Arc<dyn McpCatalogRepository>) -> Self {
        self.mcp_catalog = Some(catalog);
        self
    }

    pub fn list(&self) -> AppResult<Vec<Agent>> {
        self.agents.list()
    }

    pub fn create(&self, input: NewAgent) -> AppResult<Agent> {
        self.create_with_mcp_servers(input, Vec::new())
    }

    /// Creates the agent with servers selected from the catalog in one step.
    pub fn create_with_mcp_servers(
        &self,
        input: NewAgent,
        mcp_servers: Vec<String>,
    ) -> AppResult<Agent> {
        let provider = self.providers.get(&input.provider_id)?;
        let mut agent = Agent::create(input)?;
        let selection = self.catalog_selection(&agent.provider_id, mcp_servers)?;
        agent.change_mcp_servers(selection)?;
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
    /// Deletes the agent and everything that belongs to it. A working agent must be stopped
    /// first.
    pub fn delete(&self, agent_id: Uuid) -> AppResult<()> {
        let agent = self
            .agents
            .find(agent_id)?
            .ok_or_else(|| AppError::NotFound(format!("agent {agent_id}")))?;
        if agent.status == AgentStatus::Working {
            return Err(AppError::Validation(format!(
                "{} is working; stop the turn before deleting it",
                agent.name
            )));
        }
        self.agents.delete(agent_id)?;
        let event = DomainEvent::new(
            EventType::AgentDeleted,
            Some(agent_id),
            json!({ "agentId": agent_id, "name": agent.name }),
        );
        self.events.append(&event)?;
        self.event_bus.publish(event);
        tracing::info!(agent_id = %agent_id, "agent deleted");
        Ok(())
    }

    /// Turns the agent's workspace writes and internet access on or off. Changes apply from
    /// its next turn.
    pub fn update_access(
        &self,
        agent_id: Uuid,
        workspace_write: bool,
        network: bool,
    ) -> AppResult<Agent> {
        let mut agent = self
            .agents
            .find(agent_id)?
            .ok_or_else(|| AppError::NotFound(format!("agent {agent_id}")))?;
        agent.change_access(workspace_write, network)?;
        self.agents.save(&agent)?;
        let event = DomainEvent::new(
            EventType::AgentUpdated,
            Some(agent.id),
            json!({
                "name": agent.name,
                "workspaceWrite": workspace_write,
                "network": network,
            }),
        );
        self.events.append(&event)?;
        self.event_bus.publish(event);
        tracing::info!(agent_id = %agent.id, workspace_write, network, "agent access updated");
        Ok(agent)
    }

    pub fn update_mcp_servers(&self, agent_id: Uuid, servers: Vec<String>) -> AppResult<Agent> {
        let mut agent = self
            .agents
            .find(agent_id)?
            .ok_or_else(|| AppError::NotFound(format!("agent {agent_id}")))?;
        let selection = self.catalog_selection(&agent.provider_id, servers)?;
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

impl AgentService {
    /// Validates a selection against the provider's catalog entries.
    fn catalog_selection(
        &self,
        provider_id: &str,
        servers: Vec<String>,
    ) -> AppResult<McpServerSelection> {
        let selection = McpServerSelection::new(servers)?;
        if selection.is_empty() {
            return Ok(selection);
        }
        let provider = self.providers.get(provider_id)?;
        require(
            provider.as_ref(),
            ProviderCapability::ConfiguredMcpServers,
            "configured MCP servers",
        )?;
        let available = match &self.mcp_catalog {
            Some(catalog) => catalog.list_for_provider(provider_id)?,
            None => Vec::new(),
        };
        if let Some(missing) = selection
            .names()
            .iter()
            .find(|name| available.iter().all(|entry| &entry.name != *name))
        {
            return Err(AppError::Validation(format!(
                "MCP server \"{missing}\" is not in the {} catalog",
                provider.name()
            )));
        }
        Ok(selection)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::agents::IdentityColor,
        infrastructure::database::{
            Database, EventRepository, McpCatalogRepository, SqliteAgentRepository,
            SqliteEventRepository, SqliteMcpCatalogRepository,
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
    fn accepts_only_servers_in_the_providers_catalog() {
        let database = Arc::new(Database::in_memory().expect("database"));
        let catalog = Arc::new(SqliteMcpCatalogRepository::new(Arc::clone(&database)));
        catalog
            .replace_for_provider("claude-code", &["claude.ai Atlassian".into()])
            .expect("catalog");
        let mut registry = ProviderRegistry::new();
        let runner = crate::providers::testing::ScriptedRunner::new(Vec::new());
        registry
            .register(Arc::new(crate::providers::ClaudeProvider::new(runner)))
            .expect("claude");
        let service = AgentService::new(
            Arc::new(SqliteAgentRepository::new(Arc::clone(&database))),
            Arc::new(SqliteEventRepository::new(database)),
            EventBus::new(8),
            Arc::new(registry),
        )
        .with_mcp_catalog(catalog);
        let mut input = new_agent(None);
        input.provider_id = "claude-code".into();

        let agent = service
            .create_with_mcp_servers(input, vec!["claude.ai Atlassian".into()])
            .expect("agent");
        assert_eq!(agent.mcp_servers.names(), ["claude.ai Atlassian"]);
        assert!(matches!(
            service.update_mcp_servers(agent.id, vec!["github".into()]),
            Err(AppError::Validation(_))
        ));
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
