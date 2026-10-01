use async_trait::async_trait;

use super::{
    AgentProvider, DetectionStatus, ProviderCapability, ProviderKind, ProviderSummary, TurnEvent,
    TurnEvents, TurnOutcome, TurnRequest,
};
use crate::{error::AppResult, runtime::cancellation::CancellationSignal};

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
        vec![ProviderCapability::Sessions]
    }
    async fn run_turn(
        &self,
        request: TurnRequest,
        events: TurnEvents,
        _cancellation: CancellationSignal,
    ) -> AppResult<TurnOutcome> {
        let session_id = request
            .session_id
            .unwrap_or_else(|| format!("mock-{}", uuid::Uuid::new_v4()));
        let _ = events.send(TurnEvent::SessionStarted {
            session_id: session_id.clone(),
        });
        let received = request.prompt.chars().count();
        let _ = events.send(TurnEvent::Message {
            text: format!("Mock response: received {received} characters. No model was called."),
        });
        Ok(TurnOutcome {
            session_id: Some(session_id),
            cancelled: false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::agents::WorkspaceAccess;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn reports_only_implemented_capabilities() {
        let provider = MockProvider;
        let result = provider.detect().await.expect("detection");
        assert_eq!(result.status, DetectionStatus::Available);
        assert!(!result.capabilities.contains(&ProviderCapability::Tools));
    }

    #[tokio::test]
    async fn replies_deterministically_and_keeps_the_session() {
        let (sender, mut receiver) = mpsc::unbounded_channel();
        let outcome = MockProvider
            .run_turn(
                TurnRequest {
                    session_id: Some("mock-session".into()),
                    prompt: "hello".into(),
                    workspace: "/tmp".into(),
                    access: WorkspaceAccess::ReadOnly,
                    model: None,
                    reasoning_effort: None,
                    runtime_tools: None,
                    mcp_servers: Vec::new(),
                },
                sender,
                CancellationSignal::never(),
            )
            .await
            .expect("turn");
        assert_eq!(outcome.session_id.as_deref(), Some("mock-session"));
        receiver.recv().await.expect("session event");
        assert_eq!(
            receiver.recv().await,
            Some(TurnEvent::Message {
                text: "Mock response: received 5 characters. No model was called.".into()
            })
        );
    }
}
