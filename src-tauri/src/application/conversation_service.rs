use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use serde_json::{json, Value};
use tokio::sync::mpsc;
use uuid::Uuid;

use super::conversation_prompt::{first_turn_prompt, group_context, group_transcript};
use crate::{
    domain::{
        agents::{Agent, AgentStatus},
        conversations::{ConversationMessage, ConversationScope, MessageRole, MAX_MESSAGE_LENGTH},
        events::{DomainEvent, EventType},
        groups::{Group, GroupAuthor, GroupMessage},
        inbox::{Wake, WakeOrigin},
    },
    error::{AppError, AppResult},
    infrastructure::database::{
        AgentRepository, ConversationRepository, EventRepository, GroupRepository,
        McpCatalogRepository, MemoryRepository,
    },
    providers::{
        AgentProvider, ProviderCapability, ProviderRegistry, RuntimeToolsEndpoint, TurnEvent,
        TurnOutcome, TurnRequest,
    },
    runtime::{
        cancellation::{cancellation_pair, CancellationSignal, Canceller},
        event_bus::EventBus,
        turn_tokens::{TurnContext, TurnTokens},
    },
};

/// Messages returned when a conversation is opened.
pub const CONVERSATION_HISTORY_LIMIT: usize = 200;
/// Longest action summary kept in an event payload.
const MAX_ACTION_SUMMARY: usize = 200;
/// Most unseen group messages sent with one group turn.
const GROUP_TRANSCRIPT_LIMIT: usize = 30;

/// Shown where a reset session begins; earlier messages stay visible but not in context.
const NEW_SESSION_NOTICE: &str =
    "New session started. The agent no longer has the messages above in its context.";

pub struct ConversationService {
    agents: Arc<dyn AgentRepository>,
    conversations: Arc<dyn ConversationRepository>,
    memories: Arc<dyn MemoryRepository>,
    providers: Arc<ProviderRegistry>,
    events: Arc<dyn EventRepository>,
    event_bus: EventBus,
    running: Mutex<HashMap<Uuid, RunningTurn>>,
    chain_depths: Mutex<HashMap<Uuid, u32>>,
    runtime_tools: Option<RuntimeToolsAccess>,
    mcp_catalog: Option<Arc<dyn McpCatalogRepository>>,
    groups: Option<Arc<dyn GroupRepository>>,
}

/// An agent's turn in progress. An agent runs one turn at a time, in whichever
/// conversation woke it, because its workspace and provider CLI are shared.
struct RunningTurn {
    canceller: Canceller,
    scope: ConversationScope,
}

/// The local MCP server and the token registry it authenticates against.
struct RuntimeToolsAccess {
    url: String,
    tokens: Arc<TurnTokens>,
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
            chain_depths: Mutex::new(HashMap::new()),
            runtime_tools: None,
            mcp_catalog: None,
            groups: None,
        }
    }

    /// Lets agents take turns in groups. Without it, group wakes fail to start.
    pub fn with_groups(mut self, groups: Arc<dyn GroupRepository>) -> Self {
        self.groups = Some(groups);
        self
    }

    /// Lets turns use the agent's MCP servers that are still in the global catalog. Without
    /// it, no configured MCP servers reach the provider.
    pub fn with_mcp_catalog(mut self, catalog: Arc<dyn McpCatalogRepository>) -> Self {
        self.mcp_catalog = Some(catalog);
        self
    }

    /// The agent's selected servers that its provider supports and the catalog still lists.
    fn available_mcp_servers(
        &self,
        agent: &Agent,
        provider: &dyn AgentProvider,
    ) -> AppResult<Vec<String>> {
        let Some(catalog) = &self.mcp_catalog else {
            return Ok(Vec::new());
        };
        if agent.mcp_servers.is_empty()
            || !provider
                .capabilities()
                .contains(&ProviderCapability::ConfiguredMcpServers)
        {
            return Ok(Vec::new());
        }
        let entries = catalog.list_for_provider(&agent.provider_id)?;
        Ok(agent
            .mcp_servers
            .names()
            .iter()
            .filter(|name| entries.iter().any(|entry| &entry.name == *name))
            .cloned()
            .collect())
    }

    /// Offers the runtime tools at `url` to providers that support them. Each turn gets
    /// its own token, revoked when the turn ends.
    pub fn with_runtime_tools(mut self, url: String, tokens: Arc<TurnTokens>) -> Self {
        self.runtime_tools = Some(RuntimeToolsAccess { url, tokens });
        self
    }

    pub fn list(&self, agent_id: Uuid) -> AppResult<Vec<ConversationMessage>> {
        self.conversations
            .list_messages(agent_id, CONVERSATION_HISTORY_LIMIT)
    }

    /// Records the user's message and starts a provider turn in the background. Progress
    /// arrives as `message.created`, `tool.*`, and `agent.*` events. Must be called from
    /// within a Tokio runtime.
    pub fn send(self: &Arc<Self>, agent_id: Uuid, content: &str) -> AppResult<ConversationMessage> {
        let message = ConversationMessage::new(agent_id, MessageRole::User, content)?;
        let prompt = message.content.clone();
        // A user message starts a new chain.
        self.start_turn(
            agent_id,
            Some(message.clone()),
            prompt,
            0,
            ConversationScope::Direct,
        )?;
        Ok(message)
    }

    /// Starts a turn for a wake that did not come from the user. The wake's notice is
    /// recorded in the conversation so the user can see why the agent started working.
    /// A group wake runs in the group instead, with the messages the agent has not seen.
    pub fn start_wake(self: &Arc<Self>, wake: &Wake) -> AppResult<()> {
        if let WakeOrigin::GroupTurn { group_id, .. } = &wake.origin {
            return self.start_group_wake(wake, *group_id);
        }
        let prompt = wake.prompt();
        let mut notice = ConversationMessage::new(
            wake.agent_id,
            MessageRole::System,
            &bounded(&prompt, MAX_MESSAGE_LENGTH),
        )?;
        if let WakeOrigin::AgentMessage { from_agent_id, .. } = &wake.origin {
            notice.source_agent_id = Some(*from_agent_id);
        }
        self.start_turn(
            wake.agent_id,
            Some(notice),
            prompt,
            wake.chain_depth,
            ConversationScope::Direct,
        )
    }

    fn start_group_wake(self: &Arc<Self>, wake: &Wake, group_id: Uuid) -> AppResult<()> {
        let group = self.find_group(group_id)?;
        if !group.is_member(wake.agent_id) {
            return Err(AppError::Validation(format!(
                "the agent is not a member of group \"{}\"",
                group.name
            )));
        }
        let unseen = self.groups()?.messages_since_last_from(
            group_id,
            wake.agent_id,
            GROUP_TRANSCRIPT_LIMIT,
        )?;
        let names = self.agent_names()?;
        let entries: Vec<(String, String)> = unseen
            .iter()
            .map(|message| {
                (
                    author_label(&message.author, &names),
                    message.content.clone(),
                )
            })
            .collect();
        let prompt = if entries.is_empty() {
            wake.prompt()
        } else {
            format!(
                "{}\n\nNew messages in the group:\n\n{}",
                wake.prompt(),
                group_transcript(&entries)
            )
        };
        self.start_turn(
            wake.agent_id,
            None,
            prompt,
            wake.chain_depth,
            ConversationScope::Group(group_id),
        )
    }

    /// Stops the turn an agent is running in the group, if any. Returns whether one was
    /// running there.
    pub fn cancel_in_group(&self, agent_id: Uuid, group_id: Uuid) -> AppResult<bool> {
        let running = self.lock_running()?;
        match running.get(&agent_id) {
            Some(turn) if turn.scope == ConversationScope::Group(group_id) => {
                turn.canceller.cancel();
                tracing::info!(agent_id = %agent_id, group_id = %group_id, "group turn cancellation requested");
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    /// Whether the agent has a turn in progress.
    pub fn is_running(&self, agent_id: Uuid) -> bool {
        self.lock_running()
            .map(|running| running.contains_key(&agent_id))
            .unwrap_or(true)
    }

    /// Chained turns behind the agent's latest turn; 0 when it was started by the user.
    pub fn chain_depth(&self, agent_id: Uuid) -> u32 {
        self.chain_depths
            .lock()
            .ok()
            .and_then(|depths| depths.get(&agent_id).copied())
            .unwrap_or(0)
    }

    /// Records a runtime notice in the agent's conversation without starting a turn.
    pub fn append_notice(&self, agent_id: Uuid, content: &str) -> AppResult<()> {
        self.append_message(
            agent_id,
            MessageRole::System,
            &bounded(content, MAX_MESSAGE_LENGTH),
        )
    }

    fn start_turn(
        self: &Arc<Self>,
        agent_id: Uuid,
        message: Option<ConversationMessage>,
        prompt: String,
        chain_depth: u32,
        scope: ConversationScope,
    ) -> AppResult<()> {
        let mut agent = self.find_agent(agent_id)?;
        let provider = self.providers.get(&agent.provider_id)?;

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
            running.insert(agent_id, RunningTurn { canceller, scope });
        }
        if let Ok(mut depths) = self.chain_depths.lock() {
            depths.insert(agent_id, chain_depth);
        }
        if let Err(error) = self.begin_turn(&agent, message.as_ref(), scope) {
            self.release(agent_id);
            return Err(error);
        }

        let service = Arc::clone(self);
        tokio::spawn(async move {
            service
                .run_turn(agent, provider, prompt, signal, scope)
                .await;
        });
        Ok(())
    }

    /// Forgets the agent's provider sessions, so its next turn starts a fresh session with the
    /// full first-turn context: runtime notes, identity, instructions, and memories. The
    /// visible conversation is kept, and a notice marks where the new session begins.
    pub fn reset_session(&self, agent_id: Uuid) -> AppResult<()> {
        let agent = self.find_agent(agent_id)?;
        let cleared = {
            // Holding the lock keeps a turn from starting, or saving its session, meanwhile.
            let running = self.lock_running()?;
            if running.contains_key(&agent_id) {
                return Err(AppError::Validation(format!(
                    "{} is working; stop the turn before starting a new session",
                    agent.name
                )));
            }
            self.conversations.clear_provider_sessions(agent_id)?
        };
        self.append_message(agent_id, MessageRole::System, NEW_SESSION_NOTICE)?;
        self.publish(
            EventType::AgentSessionReset,
            agent_id,
            json!({ "name": agent.name }),
        )?;
        tracing::info!(agent_id = %agent_id, cleared, "agent session reset");
        Ok(())
    }

    /// Empties the agent's direct conversation and forgets its session there, so the next
    /// message starts fresh. Group conversations are kept.
    pub fn clear(&self, agent_id: Uuid) -> AppResult<()> {
        let agent = self.find_agent(agent_id)?;
        {
            // Holding the lock keeps a turn from starting, or saving its session, meanwhile.
            let running = self.lock_running()?;
            if running.contains_key(&agent_id) {
                return Err(AppError::Validation(format!(
                    "{} is working; stop the turn before clearing the conversation",
                    agent.name
                )));
            }
            self.conversations.clear_direct_conversation(agent_id)?;
        }
        self.publish(
            EventType::AgentConversationCleared,
            agent_id,
            json!({ "agentId": agent_id, "name": agent.name }),
        )?;
        tracing::info!(agent_id = %agent_id, "conversation cleared");
        Ok(())
    }

    /// Forgets every member's provider session in the group.
    pub fn forget_group_sessions(&self, group_id: Uuid) -> AppResult<usize> {
        self.conversations.clear_group_sessions(group_id)
    }

    /// Stops the agent's running turn. The turn then finishes as cancelled.
    pub fn cancel(&self, agent_id: Uuid) -> AppResult<()> {
        let running = self.lock_running()?;
        let turn = running
            .get(&agent_id)
            .ok_or_else(|| AppError::Validation("the agent is not working".into()))?;
        turn.canceller.cancel();
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

    fn begin_turn(
        &self,
        agent: &Agent,
        message: Option<&ConversationMessage>,
        scope: ConversationScope,
    ) -> AppResult<()> {
        self.agents.save(agent)?;
        if let Some(message) = message {
            self.conversations.append_message(message)?;
            self.publish_message(message)?;
        }
        self.publish(
            EventType::AgentStarted,
            agent.id,
            json!({
                "name": agent.name,
                "providerId": agent.provider_id,
                "groupId": scope.group_id(),
            }),
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
        scope: ConversationScope,
    ) {
        let mut replies = Vec::new();
        let result = self
            .execute_turn(
                &agent,
                provider.as_ref(),
                &content,
                signal,
                scope,
                &mut replies,
            )
            .await;
        self.release(agent.id);
        if let Err(error) = self.finish_turn(agent.id, provider.id(), scope, &replies, result) {
            tracing::error!(agent_id = %agent.id, %error, "turn result could not be recorded");
        }
    }

    /// The first-turn context of a provider session. In a group, it also describes the
    /// group, so each group session starts knowing its topic and members.
    fn session_prompt(
        &self,
        agent: &Agent,
        content: &str,
        scope: ConversationScope,
        runtime_tools: bool,
    ) -> AppResult<String> {
        let memories = self.memories.list_for_agent(agent.id)?;
        let message = match scope {
            ConversationScope::Direct => content.to_owned(),
            ConversationScope::Group(group_id) => {
                let group = self.find_group(group_id)?;
                let members = group
                    .member_ids
                    .iter()
                    .filter_map(|id| self.agents.find(*id).transpose())
                    .collect::<AppResult<Vec<_>>>()?;
                format!("{}\n\n{content}", group_context(agent, &group, &members))
            }
        };
        Ok(first_turn_prompt(agent, &memories, &message, runtime_tools))
    }

    async fn execute_turn(
        &self,
        agent: &Agent,
        provider: &dyn AgentProvider,
        content: &str,
        signal: CancellationSignal,
        scope: ConversationScope,
        replies: &mut Vec<String>,
    ) -> AppResult<TurnOutcome> {
        let session_id = self
            .conversations
            .provider_session(agent.id, provider.id(), scope)?;
        let runtime_tools = self
            .runtime_tools
            .as_ref()
            .filter(|_| {
                provider
                    .capabilities()
                    .contains(&ProviderCapability::RuntimeTools)
            })
            .map(|access| RuntimeToolsEndpoint {
                url: access.url.clone(),
                token: access.tokens.issue(TurnContext {
                    agent_id: agent.id,
                    chain_depth: self.chain_depth(agent.id),
                    group_id: scope.group_id(),
                }),
            });
        let prompt = match session_id {
            Some(_) => content.to_owned(),
            None => self.session_prompt(agent, content, scope, runtime_tools.is_some())?,
        };
        let request = TurnRequest {
            session_id,
            prompt,
            workspace: PathBuf::from(&agent.workspace),
            access: agent.permissions.workspace_access(),
            network: agent.permissions.network_access(),
            model: agent.model_selection.model().map(str::to_owned),
            reasoning_effort: agent.model_selection.reasoning_effort().map(str::to_owned),
            runtime_tools: runtime_tools.clone(),
            mcp_servers: self.available_mcp_servers(agent, provider)?,
        };
        let (sender, mut receiver) = mpsc::unbounded_channel();
        let drain = async {
            while let Some(event) = receiver.recv().await {
                if let TurnEvent::Message { text } = &event {
                    replies.push(text.clone());
                }
                if let Err(error) = self.record_turn_event(agent.id, provider.id(), scope, event) {
                    tracing::error!(agent_id = %agent.id, %error, "turn event could not be recorded");
                }
            }
        };
        let (outcome, ()) = tokio::join!(provider.run_turn(request, sender, signal), drain);
        if let (Some(access), Some(endpoint)) = (&self.runtime_tools, &runtime_tools) {
            access.tokens.revoke(&endpoint.token);
        }
        outcome
    }

    fn record_turn_event(
        &self,
        agent_id: Uuid,
        provider_id: &str,
        scope: ConversationScope,
        event: TurnEvent,
    ) -> AppResult<()> {
        let group_id = scope.group_id();
        match event {
            TurnEvent::SessionStarted { session_id } => {
                self.conversations
                    .save_provider_session(agent_id, provider_id, scope, &session_id)
            }
            TurnEvent::Message { text } => {
                let text = bounded(&text, MAX_MESSAGE_LENGTH);
                match group_id {
                    None => self.append_message(agent_id, MessageRole::Agent, &text),
                    Some(group_id) => self.append_group_message(
                        group_id,
                        GroupAuthor::Agent { agent_id },
                        &text,
                    ),
                }
            }
            TurnEvent::ActionStarted { id, summary } => self.publish(
                EventType::ToolStarted,
                agent_id,
                json!({ "agentId": agent_id, "groupId": group_id, "actionId": id, "detail": bounded(&summary, MAX_ACTION_SUMMARY) }),
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
                json!({ "agentId": agent_id, "groupId": group_id, "actionId": id, "detail": bounded(&summary, MAX_ACTION_SUMMARY) }),
            ),
        }
    }

    fn finish_turn(
        &self,
        agent_id: Uuid,
        provider_id: &str,
        scope: ConversationScope,
        replies: &[String],
        result: AppResult<TurnOutcome>,
    ) -> AppResult<()> {
        let mut agent = self.find_agent(agent_id)?;
        let group_id = scope.group_id();
        match result {
            Ok(outcome) => {
                if let Some(session_id) = &outcome.session_id {
                    self.conversations.save_provider_session(
                        agent_id,
                        provider_id,
                        scope,
                        session_id,
                    )?;
                }
                agent.transition_to(AgentStatus::Idle)?;
                self.agents.save(&agent)?;
                if outcome.cancelled {
                    self.append_scoped_notice(agent_id, scope, "Stopped.")?;
                    self.publish(
                        EventType::AgentCancelled,
                        agent_id,
                        json!({ "name": agent.name, "groupId": group_id }),
                    )?;
                    tracing::info!(agent_id = %agent_id, "turn cancelled");
                } else {
                    let mentioned = self.mentioned_in_replies(agent_id, scope, replies)?;
                    self.publish(
                        EventType::AgentCompleted,
                        agent_id,
                        json!({
                            "name": agent.name,
                            "groupId": group_id,
                            "mentionedAgentIds": mentioned,
                        }),
                    )?;
                    tracing::info!(agent_id = %agent_id, "turn completed");
                }
            }
            Err(error) => {
                agent.transition_to(AgentStatus::Failed)?;
                self.agents.save(&agent)?;
                let reason = error.to_string();
                let notice = match group_id {
                    None => format!("The turn failed: {reason}"),
                    Some(_) => format!("{}'s turn failed: {reason}", agent.name),
                };
                self.append_scoped_notice(agent_id, scope, &bounded(&notice, MAX_MESSAGE_LENGTH))?;
                self.publish(
                    EventType::AgentFailed,
                    agent_id,
                    json!({
                        "name": agent.name,
                        "groupId": group_id,
                        "detail": bounded(&reason, MAX_ACTION_SUMMARY),
                    }),
                )?;
                tracing::warn!(agent_id = %agent_id, "turn failed");
            }
        }
        Ok(())
    }

    /// Group members the agent asked to respond with `@Name` in this turn's replies.
    fn mentioned_in_replies(
        &self,
        agent_id: Uuid,
        scope: ConversationScope,
        replies: &[String],
    ) -> AppResult<Vec<Uuid>> {
        let Some(group_id) = scope.group_id() else {
            return Ok(Vec::new());
        };
        if replies.is_empty() {
            return Ok(Vec::new());
        }
        let group = self.find_group(group_id)?;
        let names: Vec<(Uuid, String)> = self.agent_names()?.into_iter().collect();
        Ok(group.speakers_for(
            GroupAuthor::Agent { agent_id },
            &replies.join("\n\n"),
            &names,
            false,
        ))
    }

    /// Records a runtime notice where the turn ran: the agent's conversation or the group.
    fn append_scoped_notice(
        &self,
        agent_id: Uuid,
        scope: ConversationScope,
        content: &str,
    ) -> AppResult<()> {
        match scope {
            ConversationScope::Direct => {
                self.append_message(agent_id, MessageRole::System, content)
            }
            ConversationScope::Group(group_id) => {
                self.append_group_message(group_id, GroupAuthor::System, content)
            }
        }
    }

    fn append_group_message(
        &self,
        group_id: Uuid,
        author: GroupAuthor,
        content: &str,
    ) -> AppResult<()> {
        let message = GroupMessage::new(group_id, author, content)?;
        self.groups()?.append_message(&message)?;
        publish_group_message(&*self.events, &self.event_bus, &message)
    }

    fn groups(&self) -> AppResult<&Arc<dyn GroupRepository>> {
        self.groups
            .as_ref()
            .ok_or_else(|| AppError::Validation("groups are not available".into()))
    }

    fn find_group(&self, group_id: Uuid) -> AppResult<Group> {
        self.groups()?
            .find(group_id)?
            .ok_or_else(|| AppError::NotFound(format!("group {group_id}")))
    }

    fn agent_names(&self) -> AppResult<HashMap<Uuid, String>> {
        Ok(self
            .agents
            .list()?
            .into_iter()
            .map(|agent| (agent.id, agent.name))
            .collect())
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
            json!({
                "agentId": message.agent_id,
                "messageId": message.id,
                "role": message.role,
                "sourceAgentId": message.source_agent_id,
            }),
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

    fn lock_running(&self) -> AppResult<std::sync::MutexGuard<'_, HashMap<Uuid, RunningTurn>>> {
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

/// Publishes a stored group message. The content stays in the group table; the event only
/// identifies it.
pub(crate) fn publish_group_message(
    events: &dyn EventRepository,
    event_bus: &EventBus,
    message: &GroupMessage,
) -> AppResult<()> {
    let event = DomainEvent::new(
        EventType::GroupMessageCreated,
        Some(message.group_id),
        json!({
            "groupId": message.group_id,
            "messageId": message.id,
            "authorAgentId": message.author.agent_id(),
        }),
    );
    events.append(&event)?;
    event_bus.publish(event);
    Ok(())
}

/// How a group message's author appears in a transcript sent to a provider.
fn author_label(author: &GroupAuthor, names: &HashMap<Uuid, String>) -> String {
    match author {
        GroupAuthor::User => "User".into(),
        GroupAuthor::System => "Open Bots".into(),
        GroupAuthor::Agent { agent_id } => names
            .get(agent_id)
            .cloned()
            .unwrap_or_else(|| "A former member".into()),
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
