use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{DomainError, DomainResult};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ApprovalRequest {
    pub id: Uuid,
    pub agent_id: Uuid,
    pub action: String,
    pub reason: String,
    pub status: ApprovalStatus,
    pub created_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ApprovalStatus {
    Pending,
    Approved,
    Denied,
}

impl ApprovalRequest {
    pub fn create(agent_id: Uuid, action: &str, reason: &str) -> DomainResult<Self> {
        let action = action.trim();
        let reason = reason.trim();
        if action.is_empty() {
            return Err(DomainError::Validation(
                "approval action cannot be empty".into(),
            ));
        }
        if reason.is_empty() {
            return Err(DomainError::Validation(
                "approval reason cannot be empty".into(),
            ));
        }
        Ok(Self {
            id: Uuid::new_v4(),
            agent_id,
            action: action.to_owned(),
            reason: reason.to_owned(),
            status: ApprovalStatus::Pending,
            created_at: Utc::now(),
            resolved_at: None,
        })
    }

    /// Records a human decision. Only pending requests can be resolved, and only to a final status.
    pub fn resolve(&mut self, decision: ApprovalStatus) -> DomainResult<()> {
        if self.status != ApprovalStatus::Pending {
            return Err(DomainError::Validation(
                "approval request has already been resolved".into(),
            ));
        }
        if decision == ApprovalStatus::Pending {
            return Err(DomainError::Validation(
                "approval decision must be approved or denied".into(),
            ));
        }
        self.status = decision;
        self.resolved_at = Some(Utc::now());
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum ApprovalDecision {
    Allow,
    Ask,
    Deny,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionContext {
    pub action_id: String,
    pub outside_workspace: bool,
}

pub trait ApprovalPolicy: Send + Sync {
    fn evaluate(&self, context: &ActionContext) -> ApprovalDecision;
}

#[derive(Debug, Default)]
pub struct DefaultApprovalPolicy;

impl ApprovalPolicy for DefaultApprovalPolicy {
    fn evaluate(&self, context: &ActionContext) -> ApprovalDecision {
        if context.outside_workspace {
            return ApprovalDecision::Deny;
        }
        match context.action_id.as_str() {
            "filesystem.read" | "test.run" => ApprovalDecision::Allow,
            "git.push" | "deploy.production" => ApprovalDecision::Ask,
            _ => ApprovalDecision::Ask,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn allows_safe_workspace_reads() {
        assert_eq!(
            DefaultApprovalPolicy.evaluate(&ActionContext {
                action_id: "filesystem.read".into(),
                outside_workspace: false
            }),
            ApprovalDecision::Allow
        );
    }
    #[test]
    fn asks_before_push() {
        assert_eq!(
            DefaultApprovalPolicy.evaluate(&ActionContext {
                action_id: "git.push".into(),
                outside_workspace: false
            }),
            ApprovalDecision::Ask
        );
    }
    #[test]
    fn creates_pending_requests_with_trimmed_fields() {
        let request = ApprovalRequest::create(Uuid::new_v4(), " git push ", " ship it ")
            .expect("approval request");
        assert_eq!(request.status, ApprovalStatus::Pending);
        assert_eq!(request.action, "git push");
        assert_eq!(request.reason, "ship it");
        assert_eq!(request.resolved_at, None);
    }
    #[test]
    fn rejects_requests_without_action_or_reason() {
        assert!(ApprovalRequest::create(Uuid::new_v4(), " ", "reason").is_err());
        assert!(ApprovalRequest::create(Uuid::new_v4(), "git push", "").is_err());
    }
    #[test]
    fn resolves_pending_requests_once() {
        let mut request =
            ApprovalRequest::create(Uuid::new_v4(), "git push", "ship it").expect("request");
        request
            .resolve(ApprovalStatus::Approved)
            .expect("resolution");
        assert_eq!(request.status, ApprovalStatus::Approved);
        assert!(request.resolved_at.is_some());
        assert!(request.resolve(ApprovalStatus::Denied).is_err());
        assert_eq!(request.status, ApprovalStatus::Approved);
    }
    #[test]
    fn rejects_pending_as_a_decision() {
        let mut request =
            ApprovalRequest::create(Uuid::new_v4(), "git push", "ship it").expect("request");
        assert!(request.resolve(ApprovalStatus::Pending).is_err());
        assert_eq!(request.status, ApprovalStatus::Pending);
    }
    #[test]
    fn denies_actions_outside_workspace() {
        assert_eq!(
            DefaultApprovalPolicy.evaluate(&ActionContext {
                action_id: "filesystem.read".into(),
                outside_workspace: true
            }),
            ApprovalDecision::Deny
        );
    }
}
