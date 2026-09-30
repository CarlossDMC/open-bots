use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
