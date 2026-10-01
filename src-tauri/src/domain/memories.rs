use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{DomainError, DomainResult};

pub const MAX_MEMORY_LENGTH: usize = 2_000;

/// A durable note an agent keeps across sessions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AgentMemory {
    pub id: Uuid,
    pub agent_id: Uuid,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

impl AgentMemory {
    pub fn create(agent_id: Uuid, content: &str) -> DomainResult<Self> {
        let content = content.trim();
        if content.is_empty() {
            return Err(DomainError::Validation("memory cannot be empty".into()));
        }
        if content.chars().count() > MAX_MEMORY_LENGTH {
            return Err(DomainError::Validation(format!(
                "memory cannot exceed {MAX_MEMORY_LENGTH} characters"
            )));
        }
        Ok(Self {
            id: Uuid::new_v4(),
            agent_id,
            content: content.to_owned(),
            created_at: Utc::now(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn trims_memory_content() {
        let memory = AgentMemory::create(Uuid::new_v4(), "  Prefers pnpm  ").expect("memory");
        assert_eq!(memory.content, "Prefers pnpm");
    }
    #[test]
    fn rejects_empty_memories() {
        assert!(AgentMemory::create(Uuid::new_v4(), " \n ").is_err());
    }
    #[test]
    fn rejects_memories_over_the_length_limit() {
        let content = "a".repeat(MAX_MEMORY_LENGTH + 1);
        assert!(AgentMemory::create(Uuid::new_v4(), &content).is_err());
        let content = "a".repeat(MAX_MEMORY_LENGTH);
        assert!(AgentMemory::create(Uuid::new_v4(), &content).is_ok());
    }
}
