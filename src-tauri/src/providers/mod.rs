mod mock;
mod registry;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::AppResult;

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

#[derive(Debug, Clone)]
pub struct StartSessionInput {
    pub system_instructions: String,
    pub workspace: String,
}
#[derive(Debug, Clone)]
pub struct ProviderSession {
    pub id: String,
}
#[derive(Debug, Clone)]
pub struct ProviderRequest {
    pub session_id: String,
    pub messages: Vec<Value>,
}
#[derive(Debug, Clone)]
pub struct ProviderResponse {
    pub content: String,
    pub structured_action: Option<Value>,
}

#[async_trait]
pub trait AgentProvider: Send + Sync {
    fn id(&self) -> &'static str;
    fn name(&self) -> &'static str;
    async fn detect(&self) -> AppResult<ProviderSummary>;
    fn capabilities(&self) -> Vec<ProviderCapability>;
    async fn start_session(&self, input: StartSessionInput) -> AppResult<ProviderSession>;
    async fn send(&self, input: ProviderRequest) -> AppResult<ProviderResponse>;
    async fn cancel(&self, session_id: &str) -> AppResult<()>;
}
