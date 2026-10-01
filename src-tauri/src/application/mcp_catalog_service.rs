use std::sync::Arc;

use serde_json::json;

use super::provider_service::require;
use crate::{
    domain::{
        events::{DomainEvent, EventType},
        mcp_servers::{normalize_mcp_server_names, McpCatalogEntry, MAX_CATALOG_SERVERS},
    },
    error::AppResult,
    infrastructure::database::{EventRepository, McpCatalogRepository},
    providers::{ConfiguredMcpServer, ProviderCapability, ProviderRegistry},
    runtime::event_bus::EventBus,
};

/// The global catalog of MCP servers, imported by name from each provider's own
/// configuration. Agents select their servers from it, and a server removed from the
/// catalog stops reaching every agent from its next turn.
pub struct McpCatalogService {
    catalog: Arc<dyn McpCatalogRepository>,
    events: Arc<dyn EventRepository>,
    event_bus: EventBus,
    providers: Arc<ProviderRegistry>,
}

impl McpCatalogService {
    pub fn new(
        catalog: Arc<dyn McpCatalogRepository>,
        events: Arc<dyn EventRepository>,
        event_bus: EventBus,
        providers: Arc<ProviderRegistry>,
    ) -> Self {
        Self {
            catalog,
            events,
            event_bus,
            providers,
        }
    }

    pub fn list(&self) -> AppResult<Vec<McpCatalogEntry>> {
        self.catalog.list()
    }

    /// Reads the servers in the provider's own configuration without changing the catalog.
    pub async fn discover(&self, provider_id: &str) -> AppResult<Vec<ConfiguredMcpServer>> {
        let provider = self.providers.get(provider_id)?;
        require(
            provider.as_ref(),
            ProviderCapability::ConfiguredMcpServers,
            "configured MCP servers",
        )?;
        provider.list_configured_mcp_servers().await
    }

    /// Replaces the provider's catalog entries with `names`.
    pub fn save(&self, provider_id: &str, names: Vec<String>) -> AppResult<Vec<McpCatalogEntry>> {
        let provider = self.providers.get(provider_id)?;
        let names = normalize_mcp_server_names(names, MAX_CATALOG_SERVERS)?;
        if !names.is_empty() {
            require(
                provider.as_ref(),
                ProviderCapability::ConfiguredMcpServers,
                "configured MCP servers",
            )?;
        }
        self.catalog.replace_for_provider(provider_id, &names)?;
        let event = DomainEvent::new(
            EventType::McpCatalogUpdated,
            None,
            json!({ "providerId": provider_id, "servers": names }),
        );
        self.events.append(&event)?;
        self.event_bus.publish(event);
        tracing::info!(provider_id, servers = names.len(), "MCP catalog updated");
        self.catalog.list()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        error::AppError,
        infrastructure::database::{Database, SqliteEventRepository, SqliteMcpCatalogRepository},
        providers::MockProvider,
    };

    fn service() -> (McpCatalogService, Arc<SqliteEventRepository>) {
        let database = Arc::new(Database::in_memory().expect("database"));
        let events = Arc::new(SqliteEventRepository::new(Arc::clone(&database)));
        let mut registry = ProviderRegistry::new();
        registry.register(Arc::new(MockProvider)).expect("mock");
        let service = McpCatalogService::new(
            Arc::new(SqliteMcpCatalogRepository::new(database)),
            events.clone(),
            EventBus::new(8),
            Arc::new(registry),
        );
        (service, events)
    }

    #[tokio::test]
    async fn rejects_providers_without_configured_servers() {
        let (service, _) = service();
        assert!(matches!(
            service.discover("mock").await,
            Err(AppError::Unsupported(_))
        ));
        assert!(matches!(
            service.save("mock", vec!["github".into()]),
            Err(AppError::Unsupported(_))
        ));
    }

    #[test]
    fn records_catalog_changes_as_events() {
        let (service, events) = service();
        assert!(service.save("mock", Vec::new()).expect("clear").is_empty());
        let recent = events.list_recent(10).expect("events");
        assert!(recent
            .iter()
            .any(|event| event.event_type == EventType::McpCatalogUpdated));
    }
}
