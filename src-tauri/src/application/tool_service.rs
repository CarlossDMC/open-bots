use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;

use crate::{
    domain::approvals::{ActionContext, ApprovalDecision, ApprovalPolicy},
    runtime::turn_tokens::TurnContext,
    tools::{ToolCallOutcome, ToolDescriptor, ToolHost, ToolRegistry, ToolRequest},
};

/// Runs runtime tools for the calling agent after the approval policy allows the action.
pub struct ToolService {
    registry: ToolRegistry,
    policy: Arc<dyn ApprovalPolicy>,
}

impl ToolService {
    pub fn new(registry: ToolRegistry, policy: Arc<dyn ApprovalPolicy>) -> Self {
        Self { registry, policy }
    }
}

#[async_trait]
impl ToolHost for ToolService {
    fn describe(&self) -> Vec<ToolDescriptor> {
        self.registry
            .list()
            .into_iter()
            .map(|tool| ToolDescriptor {
                name: tool.id().into(),
                description: tool.description().into(),
                input_schema: tool.input_contract(),
            })
            .collect()
    }

    async fn call(&self, context: TurnContext, name: &str, input: Value) -> ToolCallOutcome {
        let tool = match self.registry.get(name) {
            Ok(tool) => tool,
            Err(error) => return failure(error.to_string()),
        };
        let decision = self.policy.evaluate(&ActionContext {
            action_id: tool.action_id().into(),
            outside_workspace: false,
        });
        match decision {
            ApprovalDecision::Allow => {}
            ApprovalDecision::Deny => {
                return failure(format!("{name} is not allowed by the approval policy"))
            }
            ApprovalDecision::Ask => {
                return failure(format!(
                    "{name} needs approval, and approval for this tool is not supported yet"
                ))
            }
        }
        let request = ToolRequest {
            tool_id: name.into(),
            input,
            context,
        };
        match tool.execute(request).await {
            Ok(result) => {
                tracing::info!(agent_id = %context.agent_id, tool = name, "runtime tool completed");
                ToolCallOutcome {
                    text: format!("{}\n{}", result.summary, result.output),
                    structured: Some(result.output),
                    is_error: false,
                }
            }
            Err(error) => {
                tracing::info!(agent_id = %context.agent_id, tool = name, "runtime tool failed");
                failure(error.to_string())
            }
        }
    }
}

fn failure(text: String) -> ToolCallOutcome {
    ToolCallOutcome {
        text,
        structured: None,
        is_error: true,
    }
}
