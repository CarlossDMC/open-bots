use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{DomainError, DomainResult};

/// Default for how many turns may chain from one user message or routine before the
/// runtime stops waking agents and asks the user to step in.
pub const DEFAULT_MAX_CHAIN_TURNS: u32 = 5;
pub const MAX_CHAIN_TURNS_LIMIT: u32 = 50;
pub const MAX_AGENT_MESSAGE_LENGTH: usize = 4_000;

/// Why an agent is woken without a user message.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum WakeOrigin {
    #[serde(rename_all = "camelCase")]
    Routine { routine_id: Uuid, name: String },
    #[serde(rename_all = "camelCase")]
    TaskAssigned { task_id: Uuid, title: String },
    #[serde(rename_all = "camelCase")]
    TaskFinished {
        task_id: Uuid,
        title: String,
        status: String,
    },
    #[serde(rename_all = "camelCase")]
    AgentMessage {
        from_agent_id: Uuid,
        from_name: String,
    },
    #[serde(rename_all = "camelCase")]
    ApprovalResolved { approval_id: Uuid, approved: bool },
}

impl WakeOrigin {
    /// One line shown in the agent's conversation and sent ahead of the wake content.
    pub fn notice(&self) -> String {
        match self {
            Self::Routine { name, .. } => format!("Routine \"{name}\" triggered."),
            Self::TaskAssigned { title, task_id } => {
                format!("Task assigned to you: \"{title}\" (task {task_id}).")
            }
            Self::TaskFinished {
                title,
                status,
                task_id,
            } => format!("A task you delegated is {status}: \"{title}\" (task {task_id})."),
            Self::AgentMessage { from_name, .. } => format!(
                "Message from {from_name}. To reply, you must use the agent_message tool; a normal response is only shown in your own conversation."
            ),
            Self::ApprovalResolved {
                approval_id,
                approved,
            } => format!(
                "Your approval request {approval_id} was {}.",
                if *approved { "approved" } else { "denied" }
            ),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WakeOutcome {
    Started,
    SkippedChainLimit,
    /// The wake no longer applies, for example because its routine was deleted.
    Discarded,
}

/// A pending reason to wake an agent. Wakes queue while the agent is busy and survive
/// restarts; each one starts at most one turn.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Wake {
    pub id: Uuid,
    pub agent_id: Uuid,
    pub origin: WakeOrigin,
    pub content: String,
    /// Turns already chained before this one; user messages and routines start at 0.
    pub chain_depth: u32,
    pub created_at: DateTime<Utc>,
    pub consumed_at: Option<DateTime<Utc>>,
    pub outcome: Option<WakeOutcome>,
}

impl Wake {
    pub fn new(
        agent_id: Uuid,
        origin: WakeOrigin,
        content: &str,
        chain_depth: u32,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            agent_id,
            origin,
            content: content.trim().to_owned(),
            chain_depth,
            created_at: now,
            consumed_at: None,
            outcome: None,
        }
    }

    /// Whether running this wake would exceed the chain limit.
    pub fn exceeds(&self, max_chain_turns: u32) -> bool {
        self.chain_depth >= max_chain_turns
    }

    /// The text sent to the provider.
    pub fn prompt(&self) -> String {
        if self.content.is_empty() {
            self.origin.notice()
        } else {
            format!("{}\n\n{}", self.origin.notice(), self.content)
        }
    }

    pub fn consume(&mut self, outcome: WakeOutcome, now: DateTime<Utc>) -> DomainResult<()> {
        if self.consumed_at.is_some() {
            return Err(DomainError::Validation(
                "the wake was already handled".into(),
            ));
        }
        self.consumed_at = Some(now);
        self.outcome = Some(outcome);
        Ok(())
    }
}

/// Checks a message one agent sends another and returns it trimmed.
pub fn validate_agent_message(from: Uuid, to: Uuid, message: &str) -> DomainResult<String> {
    if from == to {
        return Err(DomainError::Validation(
            "an agent cannot message itself".into(),
        ));
    }
    let message = message.trim();
    if message.is_empty() || message.chars().count() > MAX_AGENT_MESSAGE_LENGTH {
        return Err(DomainError::Validation(format!(
            "messages must contain 1 to {MAX_AGENT_MESSAGE_LENGTH} characters"
        )));
    }
    Ok(message.to_owned())
}

pub fn validate_max_chain_turns(value: u32) -> DomainResult<u32> {
    if (1..=MAX_CHAIN_TURNS_LIMIT).contains(&value) {
        Ok(value)
    } else {
        Err(DomainError::Validation(format!(
            "chained turns must be between 1 and {MAX_CHAIN_TURNS_LIMIT}"
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn routine_wake(depth: u32) -> Wake {
        Wake::new(
            Uuid::new_v4(),
            WakeOrigin::Routine {
                routine_id: Uuid::new_v4(),
                name: "Daily triage".into(),
            },
            "  Review new issues. ",
            depth,
            Utc::now(),
        )
    }

    #[test]
    fn prompt_leads_with_the_notice() {
        assert_eq!(
            routine_wake(0).prompt(),
            "Routine \"Daily triage\" triggered.\n\nReview new issues."
        );
    }

    #[test]
    fn chain_limit_counts_previous_turns() {
        assert!(!routine_wake(4).exceeds(5));
        assert!(routine_wake(5).exceeds(5));
    }

    #[test]
    fn wakes_are_consumed_once() {
        let mut wake = routine_wake(0);
        wake.consume(WakeOutcome::Started, Utc::now())
            .expect("consume");
        assert!(wake.consume(WakeOutcome::Discarded, Utc::now()).is_err());
    }

    #[test]
    fn agent_messages_need_another_recipient_and_text() {
        let (from, to) = (Uuid::new_v4(), Uuid::new_v4());
        assert_eq!(
            validate_agent_message(from, to, " Ready for review ").expect("valid"),
            "Ready for review"
        );
        assert!(validate_agent_message(from, from, "hi").is_err());
        assert!(validate_agent_message(from, to, "   ").is_err());
    }

    #[test]
    fn agent_message_notice_explains_how_to_reply() {
        let origin = WakeOrigin::AgentMessage {
            from_agent_id: Uuid::new_v4(),
            from_name: "Atlas".into(),
        };
        let notice = origin.notice();
        assert!(notice.starts_with("Message from Atlas."));
        assert!(notice.contains("must use the agent_message tool"));
        assert!(notice.contains("normal response is only shown in your own conversation"));
    }

    #[test]
    fn validates_the_chain_limit_setting() {
        assert!(validate_max_chain_turns(0).is_err());
        assert_eq!(validate_max_chain_turns(5).expect("valid"), 5);
        assert!(validate_max_chain_turns(MAX_CHAIN_TURNS_LIMIT + 1).is_err());
    }
}
