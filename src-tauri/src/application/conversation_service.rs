use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use serde_json::{json, Value};
use tokio::sync::mpsc;
use uuid::Uuid;

use super::conversation_prompt::first_turn_prompt;
use crate::{
    domain::{
        agents::{Agent, AgentStatus},
        conversations::{ConversationMessage, MessageRole, MAX_MESSAGE_LENGTH},
        events::{DomainEvent, EventType},
    },
    error::{AppError, AppResult},
    infrastructure::database::{
        AgentRepository, ConversationRepository, EventRepository, MemoryRepository,
    },
    providers::{AgentProvider, ProviderRegistry, TurnEvent, TurnOutcome, TurnRequest},
    runtime::{
        cancellation::{cancellation_pair, CancellationSignal, Canceller},
        event_bus::EventBus,
    },
};

/// Messages returned when a conversation is opened.
pub const CONVERSATION_HISTORY_LIMIT: usize = 200;
/// Longest action summary kept in an event payload.
const MAX_ACTION_SUMMARY: usize = 200;

pub struct ConversationService {
    agents: Arc<dyn AgentRepository>,
    conversations: Arc<dyn ConversationRepository>,
    memories: Arc<dyn MemoryRepository>,
    providers: Arc<ProviderRegistry>,
    events: Arc<dyn EventRepository>,
    event_bus: EventBus,
    running: Mutex<HashMap<Uuid, Canceller>>,
}

impl ConversationService {
    pub fn new(
        agents: Arc<dyn AgentRepository>,
        conversations: Arc<dyn ConversationRepository>,
        memories: Arc<dyn MemoryRepository>,
        providers: Arc<ProviderRegistry>,
        events: Arc<dyn EventRepository>,
        event_bus: EventBus,
    ) -> Self {
        Self {
            agents,
            conversations,
            memories,
            providers,
            events,
            event_bus,
            running: Mutex::new(HashMap::new()),
        }
    }

    pub fn list(&self, agent_id: Uuid) -> AppResult<Vec<ConversationMessage>> {
        self.conversations
            .list_messages(agent_id, CONVERSATION_HISTORY_LIMIT)
    }

    /// Records the user's message and starts a provider turn in the background. Progress
    /// arrives as `message.created`, `tool.*`, and `agent.*` events. Must be called from
    /// within a Tokio runtime.
    pub fn send(self: &Arc<Self>, agent_id: Uuid, content: &str) -> AppResult<ConversationMessage> {
        let mut agent = self.find_agent(agent_id)?;
        let provider = self.providers.get(&agent.provider_id)?;
        let message = ConversationMessage::new(agent_id, MessageRole::User, content)?;

        let (canceller, signal) = cancellation_pair();
        {
            let mut running = self.lock_running()?;
            if running.contains_key(&agent_id) {
                return Err(AppError::Validation(format!(
                    "{} is already working on a message",
                    agent.name
                )));
            }
            if matches!(agent.status, AgentStatus::Failed | AgentStatus::Completed) {
                agent.transition_to(AgentStatus::Idle)?;
            }
            agent.transition_to(AgentStatus::Working)?;
            running.insert(agent_id, canceller);
        }
        if let Err(error) = self.begin_turn(&agent, &message) {
            self.release(agent_id);
            return Err(error);
        }

        let service = Arc::clone(self);
        let prompt = message.content.clone();
        tokio::spawn(async move {
            service.run_turn(agent, provider, prompt, signal).await;
        });
        Ok(message)
    }

    /// Stops the agent's running turn. The turn then finishes as cancelled.
    pub fn cancel(&self, agent_id: Uuid) -> AppResult<()> {
        let running = self.lock_running()?;
        let canceller = running
            .get(&agent_id)
            .ok_or_else(|| AppError::Validation("the agent is not working".into()))?;
        canceller.cancel();
        tracing::info!(agent_id = %agent_id, "turn cancellation requested");
        Ok(())
    }

    /// Marks turns that were running when the application last closed as failed.
    pub fn recover_interrupted(&self) -> AppResult<usize> {
        let mut recovered = 0;
        for mut agent in self.agents.list()? {
            if agent.status != AgentStatus::Working {
                continue;
            }
            agent.transition_to(AgentStatus::Failed)?;
            self.agents.save(&agent)?;
            self.append_message(
                agent.id,
                MessageRole::System,
                "Interrupted because Open Bots closed while this turn was running.",
            )?;
            self.publish(
                EventType::AgentFailed,
                agent.id,
                json!({ "name": agent.name, "reason": "interrupted" }),
            )?;
            recovered += 1;
        }
        if recovered > 0 {
            tracing::warn!(recovered, "interrupted agent turns marked as failed");
        }
        Ok(recovered)
    }

    fn begin_turn(&self, agent: &Agent, message: &ConversationMessage) -> AppResult<()> {
        self.agents.save(agent)?;
        self.conversations.append_message(message)?;
        self.publish_message(message)?;
        self.publish(
            EventType::AgentStarted,
            agent.id,
            json!({ "name": agent.name, "providerId": agent.provider_id }),
        )?;
        tracing::info!(agent_id = %agent.id, provider_id = %agent.provider_id, "turn started");
        Ok(())
    }

    async fn run_turn(
        self: Arc<Self>,
        agent: Agent,
        provider: Arc<dyn AgentProvider>,
        content: String,
        signal: CancellationSignal,
    ) {
        let result = self
            .execute_turn(&agent, provider.as_ref(), &content, signal)
            .await;
        self.release(agent.id);
        if let Err(error) = self.finish_turn(agent.id, provider.id(), result) {
            tracing::error!(agent_id = %agent.id, %error, "turn result could not be recorded");
        }
    }

    async fn execute_turn(
        &self,
        agent: &Agent,
        provider: &dyn AgentProvider,
        content: &str,
        signal: CancellationSignal,
    ) -> AppResult<TurnOutcome> {
        let session_id = self
            .conversations
            .provider_session(agent.id, provider.id())?;
        let prompt = match session_id {
            Some(_) => content.to_owned(),
            None => {
                let memories = self.memories.list_for_agent(agent.id)?;
                first_turn_prompt(agent, &memories, content)
            }
        };
        let request = TurnRequest {
            session_id,
            prompt,
            workspace: PathBuf::from(&agent.workspace),
            access: agent.permissions.workspace_access(),
            model: agent.model_selection.model().map(str::to_owned),
            reasoning_effort: agent.model_selection.reasoning_effort().map(str::to_owned),
        };
        let (sender, mut receiver) = mpsc::unbounded_channel();
        let drain = async {
            while let Some(event) = receiver.recv().await {
                if let Err(error) = self.record_turn_event(agent.id, provider.id(), event) {
                    tracing::error!(agent_id = %agent.id, %error, "turn event could not be recorded");
                }
            }
        };
        let (outcome, ()) = tokio::join!(provider.run_turn(request, sender, signal), drain);
        outcome
    }

    fn record_turn_event(
        &self,
        agent_id: Uuid,
        provider_id: &str,
        event: TurnEvent,
    ) -> AppResult<()> {
        match event {
            TurnEvent::SessionStarted { session_id } => {
                self.conversations
                    .save_provider_session(agent_id, provider_id, &session_id)
            }
            TurnEvent::Message { text } => {
                self.append_message(agent_id, MessageRole::Agent, &bounded(&text, MAX_MESSAGE_LENGTH))
            }
            TurnEvent::ActionStarted { id, summary } => self.publish(
                EventType::ToolStarted,
                agent_id,
                json!({ "agentId": agent_id, "actionId": id, "detail": bounded(&summary, MAX_ACTION_SUMMARY) }),
            ),
            TurnEvent::ActionCompleted {
                id,
                summary,
                succeeded,
            } => self.publish(
                if succeeded {
                    EventType::ToolCompleted
                } else {
                    EventType::ToolFailed
                },
                agent_id,
                json!({ "agentId": agent_id, "actionId": id, "detail": bounded(&summary, MAX_ACTION_SUMMARY) }),
            ),
        }
    }

    fn finish_turn(
        &self,
        agent_id: Uuid,
        provider_id: &str,
        result: AppResult<TurnOutcome>,
    ) -> AppResult<()> {
        let mut agent = self.find_agent(agent_id)?;
        match result {
            Ok(outcome) => {
                if let Some(session_id) = &outcome.session_id {
                    self.conversations
                        .save_provider_session(agent_id, provider_id, session_id)?;
                }
                agent.transition_to(AgentStatus::Idle)?;
                self.agents.save(&agent)?;
                if outcome.cancelled {
                    self.append_message(agent_id, MessageRole::System, "Stopped.")?;
                    self.publish(
                        EventType::AgentCancelled,
                        agent_id,
                        json!({ "name": agent.name }),
                    )?;
                    tracing::info!(agent_id = %agent_id, "turn cancelled");
                } else {
                    self.publish(
                        EventType::AgentCompleted,
                        agent_id,
                        json!({ "name": agent.name }),
                    )?;
                    tracing::info!(agent_id = %agent_id, "turn completed");
                }
            }
            Err(error) => {
                agent.transition_to(AgentStatus::Failed)?;
                self.agents.save(&agent)?;
                let reason = error.to_string();
                self.append_message(
                    agent_id,
                    MessageRole::System,
                    &bounded(&format!("The turn failed: {reason}"), MAX_MESSAGE_LENGTH),
                )?;
                self.publish(
                    EventType::AgentFailed,
                    agent_id,
                    json!({ "name": agent.name, "detail": bounded(&reason, MAX_ACTION_SUMMARY) }),
                )?;
                tracing::warn!(agent_id = %agent_id, "turn failed");
            }
        }
        Ok(())
    }

    fn append_message(&self, agent_id: Uuid, role: MessageRole, content: &str) -> AppResult<()> {
        let message = ConversationMessage::new(agent_id, role, content)?;
        self.conversations.append_message(&message)?;
        self.publish_message(&message)
    }

    fn publish_message(&self, message: &ConversationMessage) -> AppResult<()> {
        // Message content stays in the conversation table; the event only identifies it.
        self.publish(
            EventType::MessageCreated,
            message.id,
            json!({ "agentId": message.agent_id, "messageId": message.id, "role": message.role }),
        )
    }

    fn publish(&self, event_type: EventType, aggregate_id: Uuid, payload: Value) -> AppResult<()> {
        let event = DomainEvent::new(event_type, Some(aggregate_id), payload);
        self.events.append(&event)?;
        self.event_bus.publish(event);
        Ok(())
    }

    fn find_agent(&self, agent_id: Uuid) -> AppResult<Agent> {
        self.agents
            .find(agent_id)?
            .ok_or_else(|| AppError::NotFound(format!("agent {agent_id}")))
    }

    fn lock_running(&self) -> AppResult<std::sync::MutexGuard<'_, HashMap<Uuid, Canceller>>> {
        self.running
            .lock()
            .map_err(|_| AppError::Validation("turn registry is unavailable".into()))
    }

    fn release(&self, agent_id: Uuid) {
        match self.running.lock() {
            Ok(mut running) => {
                running.remove(&agent_id);
            }
            Err(_) => tracing::error!(agent_id = %agent_id, "turn registry is unavailable"),
        }
    }
}

/// Truncates on a character boundary and marks the cut.
fn bounded(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_owned();
    }
    let mut truncated: String = text.chars().take(max.saturating_sub(1)).collect();
    truncated.push('…');
    truncated
}

#[cfg(test)]
mod tests {
    use super::bounded;

    #[test]
    fn bounds_text_on_character_boundaries() {
        assert_eq!(bounded("short", 10), "short");
        assert_eq!(bounded("ááááá", 3), "áá…");
    }
}
