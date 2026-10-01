use chrono::{DateTime, SubsecRound, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{DomainError, DomainResult};

pub const MAX_MESSAGE_LENGTH: usize = 20_000;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    User,
    Agent,
    /// Runtime notices such as failures or cancellations; never sent to a provider.
    System,
}

/// One entry in the conversation between the user and an agent.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ConversationMessage {
    pub id: Uuid,
    pub agent_id: Uuid,
    pub role: MessageRole,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

impl ConversationMessage {
    pub fn new(agent_id: Uuid, role: MessageRole, content: &str) -> DomainResult<Self> {
        let content = content.trim();
        if content.is_empty() {
            return Err(DomainError::Validation("message cannot be empty".into()));
        }
        if content.chars().count() > MAX_MESSAGE_LENGTH {
            return Err(DomainError::Validation(format!(
                "message cannot exceed {MAX_MESSAGE_LENGTH} characters"
            )));
        }
        Ok(Self {
            id: Uuid::new_v4(),
            agent_id,
            role,
            content: content.to_owned(),
            // Millisecond precision matches what persistence keeps.
            created_at: Utc::now().trunc_subsecs(3),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn trims_message_content() {
        let message =
            ConversationMessage::new(Uuid::new_v4(), MessageRole::User, "  hi  ").expect("message");
        assert_eq!(message.content, "hi");
        assert_eq!(message.role, MessageRole::User);
    }
    #[test]
    fn rejects_empty_and_oversized_messages() {
        assert!(ConversationMessage::new(Uuid::new_v4(), MessageRole::User, "  ").is_err());
        let long = "a".repeat(MAX_MESSAGE_LENGTH + 1);
        assert!(ConversationMessage::new(Uuid::new_v4(), MessageRole::Agent, &long).is_err());
    }
}
