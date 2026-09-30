use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::HashMap, sync::Arc};

use crate::{
    domain::agents::PermissionLevel,
    error::{AppError, AppResult},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolRequest {
    pub tool_id: String,
    pub input: Value,
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
    fn input_contract(&self) -> Value;
    fn required_permission(&self) -> PermissionLevel;
    async fn execute(&self, request: ToolRequest) -> AppResult<ToolResult>;
}

#[derive(Default)]
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
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
}
