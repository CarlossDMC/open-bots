use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{DomainError, DomainResult};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Agent {
    pub id: Uuid,
    pub name: String,
    pub role: String,
    pub description: String,
    pub provider_id: String,
    pub identity_color: IdentityColor,
    pub avatar_variant: String,
    pub workspace: String,
    pub status: AgentStatus,
    pub instructions: String,
    pub permissions: AgentPermissions,
    pub current_task: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewAgent {
    pub name: String,
    pub role: String,
    pub description: String,
    pub provider_id: String,
    pub identity_color: IdentityColor,
    pub workspace: String,
    pub instructions: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum IdentityColor {
    Indigo,
    Cyan,
    Emerald,
    Amber,
    Rose,
    Violet,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AgentStatus {
    Idle,
    Working,
    Waiting,
    Paused,
    Failed,
    Completed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AgentPermissions {
    pub filesystem: PermissionLevel,
    pub shell: PermissionLevel,
    pub git: PermissionLevel,
    pub network: PermissionLevel,
    pub browser: PermissionLevel,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum PermissionLevel {
    WorkspaceOnly,
    Allowed,
    ApprovalRequired,
    Restricted,
    Denied,
}

impl Default for AgentPermissions {
    fn default() -> Self {
        Self {
            filesystem: PermissionLevel::WorkspaceOnly,
            shell: PermissionLevel::ApprovalRequired,
            git: PermissionLevel::ApprovalRequired,
            network: PermissionLevel::Restricted,
            browser: PermissionLevel::Denied,
        }
    }
}

impl Agent {
    pub fn create(input: NewAgent) -> DomainResult<Self> {
        if input.name.trim().len() < 2 {
            return Err(DomainError::Validation(
                "agent name must contain at least two characters".into(),
            ));
        }
        if input.role.trim().is_empty() {
            return Err(DomainError::Validation("agent role is required".into()));
        }
        if input.workspace.trim().is_empty() {
            return Err(DomainError::Validation(
                "agent workspace is required".into(),
            ));
        }
        if input.provider_id.trim().is_empty() {
            return Err(DomainError::Validation("provider is required".into()));
        }
        let now = Utc::now();
        Ok(Self {
            id: Uuid::new_v4(),
            name: input.name.trim().into(),
            role: input.role.trim().into(),
            description: input.description.trim().into(),
            provider_id: input.provider_id,
            identity_color: input.identity_color,
            avatar_variant: "orbital".into(),
            workspace: input.workspace.trim().into(),
            status: AgentStatus::Idle,
            instructions: input.instructions.trim().into(),
            permissions: AgentPermissions::default(),
            current_task: None,
            created_at: now,
            updated_at: now,
        })
    }

    pub fn transition_to(&mut self, next: AgentStatus) -> DomainResult<()> {
        let allowed = matches!(
            (self.status, next),
            (
                AgentStatus::Idle,
                AgentStatus::Working | AgentStatus::Paused
            ) | (
                AgentStatus::Working,
                AgentStatus::Waiting
                    | AgentStatus::Paused
                    | AgentStatus::Failed
                    | AgentStatus::Completed
            ) | (
                AgentStatus::Waiting,
                AgentStatus::Working | AgentStatus::Paused | AgentStatus::Failed
            ) | (
                AgentStatus::Paused,
                AgentStatus::Idle | AgentStatus::Working
            ) | (AgentStatus::Failed, AgentStatus::Idle)
                | (AgentStatus::Completed, AgentStatus::Idle)
        );
        if !allowed {
            return Err(DomainError::Validation(format!(
                "invalid agent status transition from {:?} to {:?}",
                self.status, next
            )));
        }
        self.status = next;
        self.updated_at = Utc::now();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn input() -> NewAgent {
        NewAgent {
            name: "Atlas".into(),
            role: "Engineer".into(),
            description: String::new(),
            provider_id: "mock".into(),
            identity_color: IdentityColor::Indigo,
            workspace: "/tmp/workspace".into(),
            instructions: String::new(),
        }
    }
    #[test]
    fn creates_an_idle_agent_with_safe_permissions() {
        let agent = Agent::create(input()).expect("valid agent");
        assert_eq!(agent.status, AgentStatus::Idle);
        assert_eq!(agent.permissions.git, PermissionLevel::ApprovalRequired);
    }
    #[test]
    fn rejects_invalid_status_transitions() {
        let mut agent = Agent::create(input()).expect("valid agent");
        assert!(agent.transition_to(AgentStatus::Completed).is_err());
    }
    #[test]
    fn allows_working_agent_to_wait() {
        let mut agent = Agent::create(input()).expect("valid agent");
        agent.transition_to(AgentStatus::Working).expect("start");
        agent.transition_to(AgentStatus::Waiting).expect("wait");
        assert_eq!(agent.status, AgentStatus::Waiting);
    }
}
