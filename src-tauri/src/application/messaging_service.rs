use std::sync::Arc;

use serde_json::json;
use uuid::Uuid;

use super::routine_service::now;
use crate::{
    domain::{
        events::{DomainEvent, EventType},
        inbox::{validate_agent_message, Wake, WakeOrigin},
    },
    error::{AppError, AppResult},
    infrastructure::database::{AgentRepository, EventRepository, WakeRepository},
    runtime::{event_bus::EventBus, turn_tokens::TurnContext},
};

/// Delivers messages between agents. A message is queued as a wake for the recipient, so
/// it survives restarts and runs after the recipient's current turn.
pub struct MessagingService {
    agents: Arc<dyn AgentRepository>,
    wakes: Arc<dyn WakeRepository>,
    events: Arc<dyn EventRepository>,
    event_bus: EventBus,
}

impl MessagingService {
    pub fn new(
        agents: Arc<dyn AgentRepository>,
        wakes: Arc<dyn WakeRepository>,
        events: Arc<dyn EventRepository>,
        event_bus: EventBus,
    ) -> Self {
        Self {
            agents,
            wakes,
            events,
            event_bus,
        }
    }

    pub fn send(
        &self,
        sender: TurnContext,
        recipient_id: Uuid,
        message: &str,
        task_id: Option<Uuid>,
    ) -> AppResult<Wake> {
        let from = self
            .agents
            .find(sender.agent_id)?
            .ok_or_else(|| AppError::NotFound(format!("agent {}", sender.agent_id)))?;
        if self.agents.find(recipient_id)?.is_none() {
            return Err(AppError::NotFound(format!("agent {recipient_id}")));
        }
        let mut content = validate_agent_message(from.id, recipient_id, message)?;
        if let Some(task_id) = task_id {
            content.push_str(&format!("\n\n(About task {task_id}.)"));
        }
        let wake = Wake::new(
            recipient_id,
            WakeOrigin::AgentMessage {
                from_agent_id: from.id,
                from_name: from.name.clone(),
            },
            &content,
            sender.chain_depth + 1,
            now(),
        );
        self.wakes.enqueue(&wake)?;
        // The text stays in the wake; the event only identifies sender and recipient.
        let event = DomainEvent::new(
            EventType::AgentMessage,
            Some(recipient_id),
            json!({
                "agentId": recipient_id,
                "fromAgentId": from.id,
                "fromName": from.name,
                "taskId": task_id,
            }),
        );
        self.events.append(&event)?;
        self.event_bus.publish(event);
        tracing::info!(from = %from.id, to = %recipient_id, "agent message queued");
        Ok(wake)
    }
}
