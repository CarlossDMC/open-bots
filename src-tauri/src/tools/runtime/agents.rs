use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::{
    domain::agents::PermissionLevel,
    error::AppResult,
    infrastructure::database::AgentRepository,
    tools::{parse_input, Tool, ToolRequest, ToolResult},
};

pub struct AgentListTool {
    agents: Arc<dyn AgentRepository>,
}

impl AgentListTool {
    pub fn new(agents: Arc<dyn AgentRepository>) -> Self {
        Self { agents }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {}

#[async_trait]
impl Tool for AgentListTool {
    fn id(&self) -> &'static str {
        "agent_list"
    }
    fn description(&self) -> &'static str {
        "List the other agents in this Open Bots workspace with their role and current status, \
         so you can delegate tasks or message the right one."
    }
    fn input_contract(&self) -> Value {
        json!({ "type": "object", "properties": {}, "additionalProperties": false })
    }
    fn required_permission(&self) -> PermissionLevel {
        PermissionLevel::Allowed
    }
    fn action_id(&self) -> &'static str {
        "agent.read"
    }
    async fn execute(&self, request: ToolRequest) -> AppResult<ToolResult> {
        let _: Input = parse_input(request.input)?;
        let agents: Vec<Value> = self
            .agents
            .list()?
            .into_iter()
            .filter(|agent| agent.id != request.context.agent_id)
            .map(|agent| {
                json!({
                    "id": agent.id,
                    "name": agent.name,
                    "role": agent.role,
                    "description": agent.description,
                    "status": agent.status,
                })
            })
            .collect();
        let summary = format!("{} other agent(s).", agents.len());
        Ok(ToolResult {
            output: json!({ "agents": agents }),
            summary,
        })
    }
}
