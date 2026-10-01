mod codex;
mod mock;
mod registry;

use std::path::PathBuf;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::{
    domain::agents::WorkspaceAccess, error::AppResult, runtime::cancellation::CancellationSignal,
};

pub use codex::CodexProvider;
pub use mock::MockProvider;
pub use registry::ProviderRegistry;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProviderSummary {
    pub id: String,
    pub name: String,
    pub kind: ProviderKind,
    pub status: DetectionStatus,
    pub detail: String,
    pub capabilities: Vec<ProviderCapability>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ProviderKind {
    Mock,
    Cli,
    Api,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum DetectionStatus {
    Available,
    NotInstalled,
    Unknown,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProviderCapability {
    Streaming,
    Sessions,
    Resume,
    Tools,
    Images,
    Shell,
    StructuredOutput,
    ContextCompaction,
}

/// One user turn sent to a provider. Providers that keep server-side or local sessions
/// receive the session from a previous turn and return the session they used.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TurnRequest {
    pub session_id: Option<String>,
    pub prompt: String,
    pub workspace: PathBuf,
    pub access: WorkspaceAccess,
}

/// Progress reported while a turn runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TurnEvent {
    SessionStarted {
        session_id: String,
    },
    Message {
        text: String,
    },
    ActionStarted {
        id: String,
        summary: String,
    },
    ActionCompleted {
        id: String,
        summary: String,
        succeeded: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TurnOutcome {
    pub session_id: Option<String>,
    pub cancelled: bool,
}

pub type TurnEvents = mpsc::UnboundedSender<TurnEvent>;

#[async_trait]
pub trait AgentProvider: Send + Sync {
    fn id(&self) -> &'static str;
    fn name(&self) -> &'static str;
    async fn detect(&self) -> AppResult<ProviderSummary>;
    fn capabilities(&self) -> Vec<ProviderCapability>;
    /// Runs one turn, reporting progress on `events` until it finishes or is cancelled.
    async fn run_turn(
        &self,
        request: TurnRequest,
        events: TurnEvents,
        cancellation: CancellationSignal,
    ) -> AppResult<TurnOutcome>;
}
