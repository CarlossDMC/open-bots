use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{DomainError, DomainResult};

pub const MAX_MEMORY_LENGTH: usize = 2_000;
/// Keeps the first-turn context bounded as agents learn.
pub const MAX_MEMORIES_PER_AGENT: usize = 200;

/// Who wrote a memory: the user in the agent's settings, or the agent itself.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum MemorySource {
    #[default]
    User,
    Agent,
}

/// A durable note an agent keeps across sessions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AgentMemory {
    pub id: Uuid,
    pub agent_id: Uuid,
    pub content: String,
    pub source: MemorySource,
    pub created_at: DateTime<Utc>,
}

impl AgentMemory {
    /// A memory the user wrote.
    pub fn create(agent_id: Uuid, content: &str) -> DomainResult<Self> {
        Self::new(agent_id, content, MemorySource::User)
    }

    /// A memory the agent saved for itself.
    pub fn learned(agent_id: Uuid, content: &str) -> DomainResult<Self> {
        Self::new(agent_id, content, MemorySource::Agent)
    }

    /// Rejects a new memory once the agent already keeps the maximum.
    pub fn ensure_capacity(existing: usize) -> DomainResult<()> {
        if existing >= MAX_MEMORIES_PER_AGENT {
            return Err(DomainError::Validation(format!(
                "this agent already keeps {MAX_MEMORIES_PER_AGENT} memories; remove one first"
            )));
        }
        Ok(())
    }

    fn new(agent_id: Uuid, content: &str, source: MemorySource) -> DomainResult<Self> {
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
            source,
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
    fn records_who_wrote_the_memory() {
        let agent_id = Uuid::new_v4();
        assert_eq!(
            AgentMemory::create(agent_id, "a").expect("memory").source,
            MemorySource::User
        );
        assert_eq!(
            AgentMemory::learned(agent_id, "b").expect("memory").source,
            MemorySource::Agent
        );
    }
    #[test]
    fn caps_memories_per_agent() {
        assert!(AgentMemory::ensure_capacity(MAX_MEMORIES_PER_AGENT - 1).is_ok());
        assert!(AgentMemory::ensure_capacity(MAX_MEMORIES_PER_AGENT).is_err());
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
