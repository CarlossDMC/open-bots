mod json_rpc;
mod line_process;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf};
use uuid::Uuid;

use crate::error::AppResult;

pub use json_rpc::{
    JsonRpcError, JsonRpcExit, JsonRpcMessage, JsonRpcProcessClient, JsonRpcSession,
    TokioJsonRpcProcessClient,
};
pub use line_process::{LineCommand, LineProcessExit, LineProcessRunner, TokioLineProcessRunner};

#[derive(Debug, Clone)]
pub struct ProcessSpec {
    pub program: String,
    pub arguments: Vec<String>,
    pub working_directory: PathBuf,
    pub environment: HashMap<String, String>,
    pub interactive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedProcess {
    pub id: Uuid,
    pub status: ProcessStatus,
    pub exit_code: Option<i32>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ProcessStatus {
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[async_trait]
pub trait ProcessManager: Send + Sync {
    async fn spawn(&self, specification: ProcessSpec) -> AppResult<ManagedProcess>;
    async fn cancel(&self, process_id: Uuid) -> AppResult<()>;
    async fn status(&self, process_id: Uuid) -> AppResult<ManagedProcess>;
}

pub trait ProcessOutputSink: Send + Sync {
    fn stdout(&self, process_id: Uuid, chunk: &[u8]);
    fn stderr(&self, process_id: Uuid, chunk: &[u8]);
}

#[derive(Debug, Clone)]
pub struct PtySize {
    pub columns: u16,
    pub rows: u16,
}

#[async_trait]
pub trait PtySession: Send + Sync {
    async fn write(&self, input: &[u8]) -> AppResult<()>;
    async fn resize(&self, size: PtySize) -> AppResult<()>;
    async fn close(&self) -> AppResult<()>;
}

#[async_trait]
pub trait PtyManager: Send + Sync {
    async fn spawn_interactive(&self, specification: ProcessSpec)
        -> AppResult<Box<dyn PtySession>>;
}
