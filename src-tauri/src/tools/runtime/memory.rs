use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::{
    application::MemoryService,
    domain::{agents::PermissionLevel, memories::MAX_MEMORY_LENGTH},
    error::AppResult,
    tools::{parse_input, Tool, ToolRequest, ToolResult},
};

pub struct MemorySaveTool {
    memories: Arc<MemoryService>,
}

impl MemorySaveTool {
    pub fn new(memories: Arc<MemoryService>) -> Self {
        Self { memories }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    content: String,
}

#[async_trait]
impl Tool for MemorySaveTool {
    fn id(&self) -> &'static str {
        "memory_save"
    }
    fn description(&self) -> &'static str {
        "Save a durable note to your own memory. Use it for lasting preferences, corrections, \
         and facts about the user or project that should shape future sessions. Keep each \
         note short and specific; do not save secrets or one-off details."
    }
    fn input_contract(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "content": { "type": "string", "maxLength": MAX_MEMORY_LENGTH, "description": "The note to remember." }
            },
            "required": ["content"],
            "additionalProperties": false
        })
    }
    fn required_permission(&self) -> PermissionLevel {
        PermissionLevel::Allowed
    }
    fn action_id(&self) -> &'static str {
        "memory.write"
    }
    async fn execute(&self, request: ToolRequest) -> AppResult<ToolResult> {
        let input: Input = parse_input(request.input)?;
        let memory = self
            .memories
            .add_learned(request.context.agent_id, &input.content)?;
        Ok(ToolResult {
            output: json!({ "memoryId": memory.id }),
            summary: "Saved to memory.".into(),
        })
    }
}
