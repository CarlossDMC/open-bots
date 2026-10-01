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
    #[serde(flatten)]
    pub model_selection: ModelSelection,
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
    #[serde(default, flatten)]
    pub model_selection: ModelSelection,
}

/// Longest model or reasoning-effort identifier accepted from a provider catalog.
const MAX_MODEL_IDENTIFIER_LENGTH: usize = 128;

/// The provider model an agent runs with. `None` keeps the provider's own default, and a
/// reasoning effort can only be chosen together with an explicit model.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModelSelection {
    model: Option<String>,
    reasoning_effort: Option<String>,
}

impl ModelSelection {
    pub fn new(model: Option<String>, reasoning_effort: Option<String>) -> DomainResult<Self> {
        let model = normalize_identifier(model, "model")?;
        let reasoning_effort = normalize_identifier(reasoning_effort, "reasoning effort")?;
        if model.is_none() && reasoning_effort.is_some() {
            return Err(DomainError::Validation(
                "a reasoning effort requires an explicit model".into(),
            ));
        }
        Ok(Self {
            model,
            reasoning_effort,
        })
    }

    pub fn model(&self) -> Option<&str> {
        self.model.as_deref()
    }

    pub fn reasoning_effort(&self) -> Option<&str> {
        self.reasoning_effort.as_deref()
    }

    pub fn is_provider_default(&self) -> bool {
        self.model.is_none()
    }
}

/// Identifiers reach provider command lines, so only catalog-style characters are accepted.
fn normalize_identifier(value: Option<String>, label: &str) -> DomainResult<Option<String>> {
    let Some(value) = value.map(|value| value.trim().to_owned()) else {
        return Ok(None);
    };
    if value.is_empty() {
        return Ok(None);
    }
    if value.len() > MAX_MODEL_IDENTIFIER_LENGTH {
        return Err(DomainError::Validation(format!(
            "{label} must contain at most {MAX_MODEL_IDENTIFIER_LENGTH} characters"
        )));
    }
    if !value
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || "._:/-".contains(character))
    {
        return Err(DomainError::Validation(format!(
            "{label} may only contain letters, digits, '.', '_', ':', '/' and '-'"
        )));
    }
    Ok(Some(value))
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

/// How much a provider may change the agent's workspace during a turn.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum WorkspaceAccess {
    ReadOnly,
    WorkspaceWrite,
}

impl AgentPermissions {
    /// Writing requires both workspace-scoped files and unattended shell access, because
    /// providers apply edits by running commands. Anything stricter stays read-only.
    pub fn workspace_access(&self) -> WorkspaceAccess {
        if self.filesystem == PermissionLevel::WorkspaceOnly
            && self.shell == PermissionLevel::Allowed
        {
            WorkspaceAccess::WorkspaceWrite
        } else {
            WorkspaceAccess::ReadOnly
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
        let model_selection = ModelSelection::new(
            input.model_selection.model,
            input.model_selection.reasoning_effort,
        )?;
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
            model_selection,
            current_task: None,
            created_at: now,
            updated_at: now,
        })
    }

    /// A running turn already started with the current model, so changes wait until it ends.
    pub fn change_model(&mut self, selection: ModelSelection) -> DomainResult<()> {
        if self.status == AgentStatus::Working {
            return Err(DomainError::Validation(
                "the model cannot change while the agent is working".into(),
            ));
        }
        self.model_selection = selection;
        self.updated_at = Utc::now();
        Ok(())
    }

    pub fn transition_to(&mut self, next: AgentStatus) -> DomainResult<()> {
        let allowed = matches!(
            (self.status, next),
            // An idle agent waits while one of its approval requests is pending.
            (
                AgentStatus::Idle,
                AgentStatus::Working | AgentStatus::Waiting | AgentStatus::Paused
            ) | (
                AgentStatus::Working,
                AgentStatus::Idle
                    | AgentStatus::Waiting
                    | AgentStatus::Paused
                    | AgentStatus::Failed
                    | AgentStatus::Completed
            ) | (
                AgentStatus::Waiting,
                AgentStatus::Working
                    | AgentStatus::Idle
                    | AgentStatus::Paused
                    | AgentStatus::Failed
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
            model_selection: ModelSelection::default(),
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
    fn returns_to_idle_after_a_turn() {
        let mut agent = Agent::create(input()).expect("valid agent");
        agent.transition_to(AgentStatus::Working).expect("start");
        agent.transition_to(AgentStatus::Idle).expect("finish");
        assert_eq!(agent.status, AgentStatus::Idle);
    }
    #[test]
    fn grants_workspace_writes_only_with_unattended_shell() {
        let mut permissions = AgentPermissions::default();
        assert_eq!(permissions.workspace_access(), WorkspaceAccess::ReadOnly);
        permissions.shell = PermissionLevel::Allowed;
        assert_eq!(
            permissions.workspace_access(),
            WorkspaceAccess::WorkspaceWrite
        );
        permissions.filesystem = PermissionLevel::Denied;
        assert_eq!(permissions.workspace_access(), WorkspaceAccess::ReadOnly);
    }
    #[test]
    fn idle_agents_wait_for_approvals_and_resume() {
        let mut agent = Agent::create(input()).expect("valid agent");
        agent.transition_to(AgentStatus::Waiting).expect("wait");
        agent.transition_to(AgentStatus::Working).expect("resume");
        agent
            .transition_to(AgentStatus::Waiting)
            .expect("wait again");
        agent.transition_to(AgentStatus::Idle).expect("released");
        assert!(agent.transition_to(AgentStatus::Completed).is_err());
    }
    #[test]
    fn allows_working_agent_to_wait() {
        let mut agent = Agent::create(input()).expect("valid agent");
        agent.transition_to(AgentStatus::Working).expect("start");
        agent.transition_to(AgentStatus::Waiting).expect("wait");
        assert_eq!(agent.status, AgentStatus::Waiting);
    }
    #[test]
    fn normalizes_and_validates_model_selections() {
        let selection = ModelSelection::new(Some(" gpt-5.5 ".into()), Some("high".into()))
            .expect("valid selection");
        assert_eq!(selection.model(), Some("gpt-5.5"));
        assert_eq!(selection.reasoning_effort(), Some("high"));
        assert!(ModelSelection::new(Some("  ".into()), None)
            .expect("blank is default")
            .is_provider_default());
        assert!(ModelSelection::new(None, Some("high".into())).is_err());
        assert!(ModelSelection::new(Some("gpt\"; rm".into()), None).is_err());
        assert!(ModelSelection::new(Some("m".repeat(129)), None).is_err());
    }
    #[test]
    fn creates_agents_with_a_validated_model() {
        let mut new_agent = input();
        new_agent.model_selection = ModelSelection {
            model: Some("gpt 5.5".into()),
            reasoning_effort: None,
        };
        assert!(Agent::create(new_agent).is_err());
        let mut agent = Agent::create(input()).expect("valid agent");
        assert!(agent.model_selection.is_provider_default());
        let selection =
            ModelSelection::new(Some("gpt-5.5".into()), Some("low".into())).expect("selection");
        agent.change_model(selection.clone()).expect("change");
        assert_eq!(agent.model_selection, selection);
        agent.transition_to(AgentStatus::Working).expect("start");
        assert!(agent.change_model(ModelSelection::default()).is_err());
    }
}
