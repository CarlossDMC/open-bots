use async_trait::async_trait;

use super::{
    AgentProvider, DetectionStatus, ProviderCapability, ProviderKind, ProviderRequest,
    ProviderResponse, ProviderSession, ProviderSummary, StartSessionInput,
};
use crate::error::AppResult;

#[derive(Debug, Default)]
pub struct MockProvider;

#[async_trait]
impl AgentProvider for MockProvider {
    fn id(&self) -> &'static str {
        "mock"
    }
    fn name(&self) -> &'static str {
        "Mock Provider"
    }
    async fn detect(&self) -> AppResult<ProviderSummary> {
        Ok(ProviderSummary {
            id: self.id().into(),
            name: self.name().into(),
            kind: ProviderKind::Mock,
            status: DetectionStatus::Available,
            detail: "Deterministic development adapter; no model calls are made.".into(),
            capabilities: self.capabilities(),
        })
    }
    fn capabilities(&self) -> Vec<ProviderCapability> {
        vec![
            ProviderCapability::Sessions,
            ProviderCapability::StructuredOutput,
        ]
    }
    async fn start_session(&self, _input: StartSessionInput) -> AppResult<ProviderSession> {
        Ok(ProviderSession {
            id: format!("mock-{}", uuid::Uuid::new_v4()),
        })
    }
    async fn send(&self, input: ProviderRequest) -> AppResult<ProviderResponse> {
        Ok(ProviderResponse {
            content: format!("Mock response for session {}.", input.session_id),
            structured_action: None,
        })
    }
    async fn cancel(&self, _session_id: &str) -> AppResult<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn reports_only_implemented_capabilities() {
        let provider = MockProvider;
        let result = provider.detect().await.expect("detection");
        assert_eq!(result.status, DetectionStatus::Available);
        assert!(!result.capabilities.contains(&ProviderCapability::Tools));
    }
}
