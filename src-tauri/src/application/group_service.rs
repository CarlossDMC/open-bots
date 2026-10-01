use std::sync::{Arc, Mutex, MutexGuard};

use serde_json::json;
use uuid::Uuid;

use super::{
    conversation_service::publish_group_message, routine_service::now, ConversationService,
};
use crate::{
    domain::{
        events::{DomainEvent, EventType},
        groups::{Group, GroupAuthor, GroupMessage, NewGroup},
        inbox::{Wake, WakeOrigin},
    },
    error::{AppError, AppResult},
    infrastructure::database::{AgentRepository, EventRepository, GroupRepository, WakeRepository},
    runtime::{event_bus::EventBus, turn_tokens::TurnContext},
};

/// Messages returned when a group is opened.
pub const GROUP_HISTORY_LIMIT: usize = 200;

/// Runs group conversations. A message queues the members who should answer, and the
/// members answer one at a time: each turn is a durable wake for the current speaker, so
/// a busy member answers after its current turn and a round survives restarts.
pub struct GroupService {
    groups: Arc<dyn GroupRepository>,
    agents: Arc<dyn AgentRepository>,
    wakes: Arc<dyn WakeRepository>,
    conversations: Arc<ConversationService>,
    events: Arc<dyn EventRepository>,
    event_bus: EventBus,
    /// Serializes round changes, which read, modify, and save the group.
    rounds: Mutex<()>,
}

impl GroupService {
    pub fn new(
        groups: Arc<dyn GroupRepository>,
        agents: Arc<dyn AgentRepository>,
        wakes: Arc<dyn WakeRepository>,
        conversations: Arc<ConversationService>,
        events: Arc<dyn EventRepository>,
        event_bus: EventBus,
    ) -> Self {
        Self {
            groups,
            agents,
            wakes,
            conversations,
            events,
            event_bus,
            rounds: Mutex::new(()),
        }
    }

    pub fn list(&self) -> AppResult<Vec<Group>> {
        self.groups.list()
    }

    /// Groups the agent belongs to.
    pub fn list_for_agent(&self, agent_id: Uuid) -> AppResult<Vec<Group>> {
        Ok(self
            .groups
            .list()?
            .into_iter()
            .filter(|group| group.is_member(agent_id))
            .collect())
    }

    pub fn find(&self, group_id: Uuid) -> AppResult<Group> {
        self.groups
            .find(group_id)?
            .ok_or_else(|| AppError::NotFound(format!("group {group_id}")))
    }

    pub fn messages(&self, group_id: Uuid) -> AppResult<Vec<GroupMessage>> {
        self.find(group_id)?;
        self.groups.list_messages(group_id, GROUP_HISTORY_LIMIT)
    }

    /// Creates a group for the user.
    pub fn create(&self, input: NewGroup) -> AppResult<Group> {
        self.create_as(input, GroupAuthor::User)
    }

    /// Creates a group for an agent, which always joins it. An opening message is posted
    /// for the agent and wakes the other members, one chained turn deeper.
    pub fn create_for_agent(
        &self,
        creator: TurnContext,
        mut input: NewGroup,
        message: Option<&str>,
    ) -> AppResult<Group> {
        if !input.member_ids.contains(&creator.agent_id) {
            input.member_ids.insert(0, creator.agent_id);
        }
        let group = self.create_as(
            input,
            GroupAuthor::Agent {
                agent_id: creator.agent_id,
            },
        )?;
        if let Some(message) = message.filter(|message| !message.trim().is_empty()) {
            self.post(
                group.id,
                GroupAuthor::Agent {
                    agent_id: creator.agent_id,
                },
                message,
                creator.chain_depth + 1,
            )?;
        }
        Ok(group)
    }

    fn create_as(&self, input: NewGroup, author: GroupAuthor) -> AppResult<Group> {
        for agent_id in &input.member_ids {
            if self.agents.find(*agent_id)?.is_none() {
                return Err(AppError::NotFound(format!("agent {agent_id}")));
            }
        }
        let group = Group::create(input, author)?;
        self.groups.save(&group)?;
        self.publish(
            EventType::GroupCreated,
            group.id,
            json!({
                "groupId": group.id,
                "name": group.name,
                "createdByAgentId": author.agent_id(),
            }),
        )?;
        tracing::info!(group_id = %group.id, members = group.member_ids.len(), "group created");
        Ok(group)
    }

    /// Posts the user's message. Mentioned members answer, or every member when nobody is
    /// mentioned; the chain starts over.
    pub fn post_user_message(&self, group_id: Uuid, content: &str) -> AppResult<GroupMessage> {
        self.post(group_id, GroupAuthor::User, content, 0)
    }

    /// Posts a message for an agent into a group it belongs to, from outside that group.
    /// Inside a group turn, the agent's normal reply is already posted there.
    pub fn post_agent_message(
        &self,
        sender: TurnContext,
        group_id: Uuid,
        content: &str,
    ) -> AppResult<GroupMessage> {
        if sender.group_id == Some(group_id) {
            return Err(AppError::Validation(
                "you are answering in this group already; your normal reply is posted here".into(),
            ));
        }
        let group = self.find(group_id)?;
        if !group.is_member(sender.agent_id) {
            return Err(AppError::Validation(format!(
                "only members can post in group \"{}\"",
                group.name
            )));
        }
        self.post(
            group_id,
            GroupAuthor::Agent {
                agent_id: sender.agent_id,
            },
            content,
            sender.chain_depth + 1,
        )
    }

    fn post(
        &self,
        group_id: Uuid,
        author: GroupAuthor,
        content: &str,
        chain_depth: u32,
    ) -> AppResult<GroupMessage> {
        let _rounds = self.lock_rounds()?;
        let mut group = self.find(group_id)?;
        let message = GroupMessage::new(group_id, author, content)?;
        self.groups.append_message(&message)?;
        publish_group_message(&*self.events, &self.event_bus, &message)?;
        let speakers = group.speakers_for(author, &message.content, &self.member_names()?, true);
        group.round.enqueue(&speakers, chain_depth);
        self.advance(&mut group)?;
        Ok(message)
    }

    /// Moves the round on after the agent's group turn ended. Members it mentioned answer
    /// next, one chained turn deeper.
    pub fn on_turn_finished(
        &self,
        agent_id: Uuid,
        group_id: Uuid,
        mentioned: &[Uuid],
    ) -> AppResult<()> {
        let _rounds = self.lock_rounds()?;
        let Some(mut group) = self.groups.find(group_id)? else {
            return Ok(());
        };
        let Some(speaker) = group.round.current() else {
            return Ok(());
        };
        if !group.round.finish(agent_id) {
            return Ok(());
        }
        let mentioned: Vec<Uuid> = mentioned
            .iter()
            .copied()
            .filter(|id| *id != agent_id && group.is_member(*id))
            .collect();
        group.round.enqueue(&mentioned, speaker.chain_depth + 1);
        self.advance(&mut group)
    }

    /// Whether the wake is the one queued for the group's current speaker. Other wakes for
    /// the group are left over from a stopped or restarted round.
    pub fn is_current_wake(&self, group_id: Uuid, wake_id: Uuid) -> AppResult<bool> {
        Ok(self
            .groups
            .find(group_id)?
            .is_some_and(|group| group.round.active_wake_id == Some(wake_id)))
    }

    /// Passes over the current speaker whose wake will not run, noting why in the group.
    pub fn skip_turn(&self, wake: &Wake, group_id: Uuid, reason: &str) -> AppResult<()> {
        let _rounds = self.lock_rounds()?;
        let Some(mut group) = self.groups.find(group_id)? else {
            return Ok(());
        };
        if group.round.active_wake_id != Some(wake.id) {
            return Ok(());
        }
        self.post_notice(group_id, reason)?;
        group.round.finish(wake.agent_id);
        self.advance(&mut group)
    }

    /// Ends the round: nobody else answers until the next message, and a member answering
    /// now is stopped.
    pub fn stop(&self, group_id: Uuid) -> AppResult<()> {
        let _rounds = self.lock_rounds()?;
        let mut group = self.find(group_id)?;
        let Some(speaker) = group.round.current() else {
            return Ok(());
        };
        group.round.clear();
        self.groups.save(&group)?;
        // A cancelled turn records "Stopped." itself; otherwise note it here.
        if !self
            .conversations
            .cancel_in_group(speaker.agent_id, group_id)?
        {
            self.post_notice(group_id, "Stopped.")?;
        }
        self.publish_round_completed(&group)?;
        tracing::info!(group_id = %group_id, "group round stopped");
        Ok(())
    }

    /// Deletes the group's messages and its members' sessions in it, keeping the group and
    /// its members. A group that is answering must be stopped first.
    pub fn clear(&self, group_id: Uuid) -> AppResult<()> {
        let _rounds = self.lock_rounds()?;
        let group = self.find(group_id)?;
        if group.round.is_active() {
            return Err(AppError::Validation(
                "members are answering; stop the group before clearing it".into(),
            ));
        }
        self.groups.clear_messages(group_id)?;
        self.conversations.forget_group_sessions(group_id)?;
        self.publish(
            EventType::GroupCleared,
            group_id,
            json!({ "groupId": group_id, "name": group.name }),
        )?;
        tracing::info!(group_id = %group_id, "group cleared");
        Ok(())
    }

    /// Takes a deleted agent out of every round, so the next member answers in its place.
    pub fn forget_agent(&self, agent_id: Uuid) -> AppResult<()> {
        let _rounds = self.lock_rounds()?;
        for mut group in self.groups.list()? {
            if !group
                .round
                .queue
                .iter()
                .any(|speaker| speaker.agent_id == agent_id)
            {
                continue;
            }
            group.round.remove(agent_id);
            self.advance(&mut group)?;
        }
        Ok(())
    }

    pub fn delete(&self, group_id: Uuid) -> AppResult<()> {
        self.stop(group_id)?;
        let _rounds = self.lock_rounds()?;
        let group = self.find(group_id)?;
        self.groups.delete(group_id)?;
        self.publish(
            EventType::GroupDeleted,
            group_id,
            json!({ "groupId": group_id, "name": group.name }),
        )?;
        tracing::info!(group_id = %group_id, "group deleted");
        Ok(())
    }

    /// Requeues the current speaker of every unfinished round, e.g. after a restart
    /// interrupted its turn. The previous wake becomes stale.
    pub fn resume_interrupted(&self) -> AppResult<usize> {
        let _rounds = self.lock_rounds()?;
        let mut resumed = 0;
        for mut group in self.groups.list()? {
            if !group.round.is_active() {
                continue;
            }
            group.round.active_wake_id = None;
            self.advance(&mut group)?;
            resumed += 1;
        }
        if resumed > 0 {
            tracing::info!(resumed, "group rounds resumed");
        }
        Ok(resumed)
    }

    /// Queues a wake for the current speaker when none is queued, saves the round, and
    /// announces either the queued turn or the end of the round.
    fn advance(&self, group: &mut Group) -> AppResult<()> {
        let queued = match group.round.current() {
            Some(speaker) if group.round.active_wake_id.is_none() => {
                let wake = Wake::new(
                    speaker.agent_id,
                    WakeOrigin::GroupTurn {
                        group_id: group.id,
                        group_name: group.name.clone(),
                    },
                    "",
                    speaker.chain_depth,
                    now(),
                );
                self.wakes.enqueue(&wake)?;
                group.round.active_wake_id = Some(wake.id);
                Some(speaker.agent_id)
            }
            _ => None,
        };
        self.groups.save(group)?;
        if let Some(agent_id) = queued {
            // The runtime starts the wake when the member is free.
            self.publish(
                EventType::GroupTurnQueued,
                agent_id,
                json!({ "groupId": group.id, "agentId": agent_id }),
            )?;
        } else if !group.round.is_active() {
            self.publish_round_completed(group)?;
        }
        Ok(())
    }

    fn publish_round_completed(&self, group: &Group) -> AppResult<()> {
        self.publish(
            EventType::GroupRoundCompleted,
            group.id,
            json!({ "groupId": group.id, "name": group.name }),
        )
    }

    fn post_notice(&self, group_id: Uuid, content: &str) -> AppResult<()> {
        let message = GroupMessage::new(group_id, GroupAuthor::System, content)?;
        self.groups.append_message(&message)?;
        publish_group_message(&*self.events, &self.event_bus, &message)
    }

    fn member_names(&self) -> AppResult<Vec<(Uuid, String)>> {
        Ok(self
            .agents
            .list()?
            .into_iter()
            .map(|agent| (agent.id, agent.name))
            .collect())
    }

    fn publish(
        &self,
        event_type: EventType,
        aggregate_id: Uuid,
        payload: serde_json::Value,
    ) -> AppResult<()> {
        let event = DomainEvent::new(event_type, Some(aggregate_id), payload);
        self.events.append(&event)?;
        self.event_bus.publish(event);
        Ok(())
    }

    fn lock_rounds(&self) -> AppResult<MutexGuard<'_, ()>> {
        self.rounds
            .lock()
            .map_err(|_| AppError::Validation("group rounds are unavailable".into()))
    }
}
