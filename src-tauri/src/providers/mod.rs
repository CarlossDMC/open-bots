mod claude;
mod cli;
mod codex;
mod mock;
mod registry;
#[cfg(test)]
mod testing;

use std::path::PathBuf;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::{
    domain::agents::WorkspaceAccess,
    error::{AppError, AppResult},
    runtime::cancellation::CancellationSignal,
};

pub use claude::ClaudeProvider;
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
    /// The provider lists its models and runs turns with a chosen model.
    ModelSelection,
    /// The provider reports account usage against its rate-limit windows.
    UsageLimits,
    /// The provider connects to the Open Bots MCP server during a turn, so the agent can
    /// manage tasks, memories, messages, and approvals.
    RuntimeTools,
}

/// A model a provider can run, as reported by the provider's own catalog.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProviderModel {
    pub id: String,
    pub display_name: String,
    pub description: String,
    pub is_default: bool,
    pub reasoning_efforts: Vec<String>,
    pub default_reasoning_effort: Option<String>,
}

/// Account usage reported by a provider at `checked_at`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProviderUsage {
    pub provider_id: String,
    pub plan: Option<String>,
    pub windows: Vec<UsageWindow>,
    pub limit_reached: bool,
    pub checked_at: DateTime<Utc>,
}

/// One rate-limit window, such as a five-hour or weekly allowance.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UsageWindow {
    pub duration_minutes: Option<i64>,
    /// Narrower allowance the window applies to, such as one model family; `None` means all
    /// usage counts against it.
    pub scope: Option<String>,
    pub used_percent: u8,
    pub resets_at: Option<DateTime<Utc>>,
}

/// One user turn sent to a provider. Providers that keep server-side or local sessions
/// receive the session from a previous turn and return the session they used.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TurnRequest {
    pub session_id: Option<String>,
    pub prompt: String,
    pub workspace: PathBuf,
    pub access: WorkspaceAccess,
    /// `None` keeps the provider's default model.
    pub model: Option<String>,
    pub reasoning_effort: Option<String>,
    /// Set only for providers that declare `RuntimeTools`.
    pub runtime_tools: Option<RuntimeToolsEndpoint>,
}

/// MCP server name the adapters register, so tools appear as `mcp__open_bots__<tool>`.
pub const RUNTIME_TOOLS_SERVER_NAME: &str = "open_bots";
/// Environment variable that carries the per-turn bearer token to the provider process.
pub const RUNTIME_TOOLS_TOKEN_ENV: &str = "OPEN_BOTS_MCP_TOKEN";

/// Where a provider reaches the Open Bots MCP server for one turn. The token is a secret:
/// adapters pass it through the environment, never as an argument, and `Debug` hides it.
#[derive(Clone, PartialEq, Eq)]
pub struct RuntimeToolsEndpoint {
    pub url: String,
    pub token: String,
}

impl std::fmt::Debug for RuntimeToolsEndpoint {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RuntimeToolsEndpoint")
            .field("url", &self.url)
            .field("token", &"<redacted>")
            .finish()
    }
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
    /// Implemented by providers that declare `ModelSelection`.
    async fn list_models(&self) -> AppResult<Vec<ProviderModel>> {
        Err(AppError::Unsupported(format!(
            "{} does not list models",
            self.name()
        )))
    }
    /// Implemented by providers that declare `UsageLimits`.
    async fn read_usage(&self) -> AppResult<ProviderUsage> {
        Err(AppError::Unsupported(format!(
            "{} does not report usage limits",
            self.name()
        )))
    }
}
