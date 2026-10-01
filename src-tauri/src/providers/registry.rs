use std::{collections::HashMap, sync::Arc};

use crate::error::{AppError, AppResult};

use super::{AgentProvider, ProviderSummary};

#[derive(Default)]
pub struct ProviderRegistry {
    providers: HashMap<String, Arc<dyn AgentProvider>>,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn register(&mut self, provider: Arc<dyn AgentProvider>) -> AppResult<()> {
        let id = provider.id().to_owned();
        if self.providers.contains_key(&id) {
            return Err(AppError::Provider(format!(
                "provider '{id}' is already registered"
            )));
        }
        self.providers.insert(id, provider);
        Ok(())
    }
    pub fn get(&self, id: &str) -> AppResult<Arc<dyn AgentProvider>> {
        self.providers
            .get(id)
            .cloned()
            .ok_or_else(|| AppError::NotFound(format!("provider '{id}'")))
    }
    /// Every registered provider, ordered by name.
    pub fn all(&self) -> Vec<Arc<dyn AgentProvider>> {
        let mut providers: Vec<_> = self.providers.values().cloned().collect();
        providers.sort_by(|left, right| left.name().cmp(right.name()));
        providers
    }
    pub async fn detect_all(&self) -> Vec<ProviderSummary> {
        let mut results = Vec::with_capacity(self.providers.len());
        for provider in self.providers.values() {
            match provider.detect().await {
                Ok(result) => results.push(result),
                Err(error) => {
                    tracing::warn!(provider_id = provider.id(), error = %error, "provider detection failed");
                    results.push(ProviderSummary {
                        id: provider.id().into(),
                        name: provider.name().into(),
                        kind: super::ProviderKind::Mock,
                        status: super::DetectionStatus::Unknown,
                        detail: "Provider detection failed. See local logs for details.".into(),
                        capabilities: Vec::new(),
                    });
                }
            }
        }
        results.sort_by(|left, right| left.name.cmp(&right.name));
        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::MockProvider;
    #[test]
    fn rejects_duplicate_provider_ids() {
        let mut registry = ProviderRegistry::new();
        registry.register(Arc::new(MockProvider)).expect("register");
        assert!(registry.register(Arc::new(MockProvider)).is_err());
    }
}
