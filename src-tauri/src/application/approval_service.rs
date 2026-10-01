use std::sync::Arc;

use serde_json::json;
use uuid::Uuid;

use crate::{
    domain::{
        approvals::{ApprovalRequest, ApprovalStatus},
        events::{DomainEvent, EventType},
    },
    error::{AppError, AppResult},
    infrastructure::database::{AgentRepository, ApprovalRepository, EventRepository},
    runtime::event_bus::EventBus,
};

pub struct ApprovalService {
    approvals: Arc<dyn ApprovalRepository>,
    agents: Arc<dyn AgentRepository>,
    events: Arc<dyn EventRepository>,
    event_bus: EventBus,
}

impl ApprovalService {
    pub fn new(
        approvals: Arc<dyn ApprovalRepository>,
        agents: Arc<dyn AgentRepository>,
        events: Arc<dyn EventRepository>,
        event_bus: EventBus,
    ) -> Self {
        Self {
            approvals,
            agents,
            events,
            event_bus,
        }
    }

    pub fn list(&self) -> AppResult<Vec<ApprovalRequest>> {
        self.approvals.list()
    }

    /// Records a policy-gated action that must wait for a human decision.
    pub fn request(
        &self,
        agent_id: Uuid,
        action: &str,
        reason: &str,
    ) -> AppResult<ApprovalRequest> {
        if self.agents.find(agent_id)?.is_none() {
            return Err(AppError::NotFound(format!("agent {agent_id}")));
        }
        let approval = ApprovalRequest::create(agent_id, action, reason)?;
        self.approvals.save(&approval)?;
        self.publish(EventType::ApprovalRequested, &approval)?;
        tracing::info!(approval_id = %approval.id, agent_id = %agent_id, "approval requested");
        Ok(approval)
    }

    pub fn resolve(&self, id: Uuid, decision: ApprovalStatus) -> AppResult<ApprovalRequest> {
        let mut approval = self
            .approvals
            .find(id)?
            .ok_or_else(|| AppError::NotFound(format!("approval {id}")))?;
        approval.resolve(decision)?;
        self.approvals.save(&approval)?;
        let event_type = match approval.status {
            ApprovalStatus::Approved => EventType::ApprovalApproved,
            _ => EventType::ApprovalDenied,
        };
        self.publish(event_type, &approval)?;
        tracing::info!(approval_id = %approval.id, status = ?approval.status, "approval resolved");
        Ok(approval)
    }

    fn publish(&self, event_type: EventType, approval: &ApprovalRequest) -> AppResult<()> {
        let event = DomainEvent::new(
            event_type,
            Some(approval.id),
            json!({ "agentId": approval.agent_id, "action": approval.action }),
        );
        self.events.append(&event)?;
        self.event_bus.publish(event);
        Ok(())
    }
}
