use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

use super::resolve_agent;
use crate::{
    application::{ApprovalService, MessagingService},
    domain::{agents::PermissionLevel, inbox::MAX_AGENT_MESSAGE_LENGTH},
    error::AppResult,
    infrastructure::database::AgentRepository,
    tools::{parse_input, Tool, ToolRequest, ToolResult},
};

pub struct AgentMessageTool {
    agents: Arc<dyn AgentRepository>,
    messaging: Arc<MessagingService>,
}

impl AgentMessageTool {
    pub fn new(agents: Arc<dyn AgentRepository>, messaging: Arc<MessagingService>) -> Self {
        Self { agents, messaging }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct MessageInput {
    to: String,
    message: String,
    task_id: Option<Uuid>,
}

#[async_trait]
impl Tool for AgentMessageTool {
    fn id(&self) -> &'static str {
        "agent_message"
    }
    fn description(&self) -> &'static str {
        "Send a message to another agent by name or id. The recipient is woken to read it after \
         its current turn. Use it to ask questions, share findings, or hand off context; use \
         task_create instead when you want work done and tracked."
    }
    fn input_contract(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "to": { "type": "string", "description": "Agent name or id." },
                "message": { "type": "string", "maxLength": MAX_AGENT_MESSAGE_LENGTH },
                "taskId": { "type": "string", "description": "Task the message is about." }
            },
            "required": ["to", "message"],
            "additionalProperties": false
        })
    }
    fn required_permission(&self) -> PermissionLevel {
        PermissionLevel::Allowed
    }
    fn action_id(&self) -> &'static str {
        "agent.message"
    }
    async fn execute(&self, request: ToolRequest) -> AppResult<ToolResult> {
        let input: MessageInput = parse_input(request.input)?;
        let recipient = resolve_agent(self.agents.as_ref(), &input.to)?;
        self.messaging
            .send(request.context, recipient.id, &input.message, input.task_id)?;
        Ok(ToolResult {
            output: json!({ "deliveredTo": recipient.id }),
            summary: format!("Message queued for {}.", recipient.name),
        })
    }
}

pub struct ApprovalRequestTool {
    approvals: Arc<ApprovalService>,
}

impl ApprovalRequestTool {
    pub fn new(approvals: Arc<ApprovalService>) -> Self {
        Self { approvals }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ApprovalInput {
    action: String,
    reason: String,
}

#[async_trait]
impl Tool for ApprovalRequestTool {
    fn id(&self) -> &'static str {
        "approval_request"
    }
    fn description(&self) -> &'static str {
        "Ask the user to approve a sensitive action before you take it, such as pushing \
         commits, deleting data, spending money, or contacting people outside the team. After \
         requesting, end your turn without taking the action; you are woken with the decision."
    }
    fn input_contract(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "action": { "type": "string", "description": "Exactly what you want to do, e.g. the command." },
                "reason": { "type": "string", "description": "Why it is needed." }
            },
            "required": ["action", "reason"],
            "additionalProperties": false
        })
    }
    fn required_permission(&self) -> PermissionLevel {
        PermissionLevel::Allowed
    }
    fn action_id(&self) -> &'static str {
        "approval.request"
    }
    async fn execute(&self, request: ToolRequest) -> AppResult<ToolResult> {
        let input: ApprovalInput = parse_input(request.input)?;
        let approval =
            self.approvals
                .request(request.context.agent_id, &input.action, &input.reason)?;
        Ok(ToolResult {
            output: json!({ "approvalId": approval.id, "status": approval.status }),
            summary: "Approval requested. End your turn now; you will be woken with the decision."
                .into(),
        })
    }
}
