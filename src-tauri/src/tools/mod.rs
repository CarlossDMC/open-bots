pub mod runtime;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::BTreeMap, sync::Arc};

use crate::{
    domain::agents::PermissionLevel,
    error::{AppError, AppResult},
    runtime::turn_tokens::TurnContext,
};

#[derive(Debug, Clone)]
pub struct ToolRequest {
    pub tool_id: String,
    pub input: Value,
    /// The agent and turn the call acts for.
    pub context: TurnContext,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolResult {
    pub output: Value,
    pub summary: String,
}

#[async_trait]
pub trait Tool: Send + Sync {
    fn id(&self) -> &'static str;
    fn description(&self) -> &'static str;
    /// JSON Schema of the input object.
    fn input_contract(&self) -> Value;
    fn required_permission(&self) -> PermissionLevel;
    /// Action identifier evaluated by the approval policy before the tool runs.
    fn action_id(&self) -> &'static str;
    async fn execute(&self, request: ToolRequest) -> AppResult<ToolResult>;
}

/// Parses a tool input into its typed form; the error names what is wrong for the agent.
pub fn parse_input<T: for<'de> Deserialize<'de>>(input: Value) -> AppResult<T> {
    serde_json::from_value(input)
        .map_err(|error| AppError::Validation(format!("invalid tool input: {error}")))
}

#[derive(Default)]
pub struct ToolRegistry {
    tools: BTreeMap<String, Arc<dyn Tool>>,
}

impl ToolRegistry {
    pub fn register(&mut self, tool: Arc<dyn Tool>) -> AppResult<()> {
        let id = tool.id().to_owned();
        if self.tools.insert(id.clone(), tool).is_some() {
            return Err(AppError::Validation(format!(
                "tool '{id}' is already registered"
            )));
        }
        Ok(())
    }
    pub fn get(&self, id: &str) -> AppResult<Arc<dyn Tool>> {
        self.tools
            .get(id)
            .cloned()
            .ok_or_else(|| AppError::NotFound(format!("tool '{id}'")))
    }
    /// Registered tools in identifier order.
    pub fn list(&self) -> Vec<Arc<dyn Tool>> {
        self.tools.values().cloned().collect()
    }
}

/// What a tool call returns to the provider.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolCallOutcome {
    pub text: String,
    pub structured: Option<Value>,
    pub is_error: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ToolDescriptor {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

/// Runs runtime tools for a transport such as the local MCP server.
#[async_trait]
pub trait ToolHost: Send + Sync {
    fn describe(&self) -> Vec<ToolDescriptor>;
    async fn call(&self, context: TurnContext, name: &str, input: Value) -> ToolCallOutcome;
}
