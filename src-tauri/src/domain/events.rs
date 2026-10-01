use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DomainEvent {
    pub id: Uuid,
    pub event_type: EventType,
    pub aggregate_id: Option<Uuid>,
    pub payload: Value,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EventType {
    #[serde(rename = "agent.created")]
    AgentCreated,
    #[serde(rename = "agent.updated")]
    AgentUpdated,
    #[serde(rename = "agent.session_reset")]
    AgentSessionReset,
    #[serde(rename = "agent.started")]
    AgentStarted,
    #[serde(rename = "agent.paused")]
    AgentPaused,
    #[serde(rename = "agent.resumed")]
    AgentResumed,
    #[serde(rename = "agent.waiting")]
    AgentWaiting,
    #[serde(rename = "agent.failed")]
    AgentFailed,
    #[serde(rename = "agent.completed")]
    AgentCompleted,
    #[serde(rename = "agent.cancelled")]
    AgentCancelled,
    /// A wake was not run because the chain of turns reached its limit.
    #[serde(rename = "agent.wake_skipped")]
    AgentWakeSkipped,
    #[serde(rename = "message.created")]
    MessageCreated,
    #[serde(rename = "group.created")]
    GroupCreated,
    #[serde(rename = "group.deleted")]
    GroupDeleted,
    #[serde(rename = "group.message_created")]
    GroupMessageCreated,
    /// A member was queued to answer in a group; the runtime starts it when the member is free.
    #[serde(rename = "group.turn_queued")]
    GroupTurnQueued,
    /// Every queued member has answered, or the user stopped the group.
    #[serde(rename = "group.round_completed")]
    GroupRoundCompleted,
    #[serde(rename = "task.created")]
    TaskCreated,
    #[serde(rename = "task.assigned")]
    TaskAssigned,
    #[serde(rename = "task.started")]
    TaskStarted,
    #[serde(rename = "task.completed")]
    TaskCompleted,
    #[serde(rename = "task.failed")]
    TaskFailed,
    #[serde(rename = "task.cancelled")]
    TaskCancelled,
    /// A status change that neither starts nor finishes the task, such as blocked.
    #[serde(rename = "task.updated")]
    TaskUpdated,
    #[serde(rename = "agent.message")]
    AgentMessage,
    #[serde(rename = "tool.requested")]
    ToolRequested,
    #[serde(rename = "tool.started")]
    ToolStarted,
    #[serde(rename = "tool.completed")]
    ToolCompleted,
    #[serde(rename = "tool.failed")]
    ToolFailed,
    #[serde(rename = "artifact.created")]
    ArtifactCreated,
    #[serde(rename = "approval.requested")]
    ApprovalRequested,
    #[serde(rename = "approval.approved")]
    ApprovalApproved,
    #[serde(rename = "approval.denied")]
    ApprovalDenied,
    #[serde(rename = "memory.added")]
    MemoryAdded,
    #[serde(rename = "memory.removed")]
    MemoryRemoved,
    #[serde(rename = "mcp_catalog.updated")]
    McpCatalogUpdated,
    #[serde(rename = "routine.created")]
    RoutineCreated,
    #[serde(rename = "routine.updated")]
    RoutineUpdated,
    #[serde(rename = "routine.deleted")]
    RoutineDeleted,
    #[serde(rename = "routine.triggered")]
    RoutineTriggered,
    #[serde(rename = "process.started")]
    ProcessStarted,
    #[serde(rename = "process.completed")]
    ProcessCompleted,
    #[serde(rename = "process.failed")]
    ProcessFailed,
}

impl DomainEvent {
    pub fn new(event_type: EventType, aggregate_id: Option<Uuid>, payload: Value) -> Self {
        Self {
            id: Uuid::new_v4(),
            event_type,
            aggregate_id,
            payload,
            occurred_at: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AgentMessage {
    pub from_agent_id: Uuid,
    pub to_agent_id: Uuid,
    pub message: String,
    pub artifact_ids: Vec<Uuid>,
    pub task_id: Option<Uuid>,
    pub requests_action: bool,
}
