use std::sync::Arc;

use serde_json::json;
use tokio::sync::broadcast::{error::RecvError, Receiver};
use uuid::Uuid;

use super::{routine_service::now, ConversationService, GroupService, SettingsService};
use crate::{
    domain::{
        agents::AgentStatus,
        approvals::ApprovalStatus,
        events::{DomainEvent, EventType},
        inbox::{Wake, WakeOrigin, WakeOutcome},
        tasks::Task,
    },
    error::AppResult,
    infrastructure::database::{
        AgentRepository, ApprovalRepository, EventRepository, RoutineRepository, TaskRepository,
        WakeRepository,
    },
    runtime::event_bus::EventBus,
};

/// Wakes agents for structured events instead of user messages. Each relevant event is
/// queued as a durable wake, and an idle agent runs its oldest wake as one turn. Nothing
/// here calls a model while waiting; the loop sleeps on the event bus.
pub struct AgentRuntime {
    wakes: Arc<dyn WakeRepository>,
    agents: Arc<dyn AgentRepository>,
    routines: Arc<dyn RoutineRepository>,
    tasks: Arc<dyn TaskRepository>,
    approvals: Arc<dyn ApprovalRepository>,
    settings: Arc<SettingsService>,
    conversations: Arc<ConversationService>,
    groups: Option<Arc<GroupService>>,
    events: Arc<dyn EventRepository>,
    event_bus: EventBus,
}

impl AgentRuntime {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        wakes: Arc<dyn WakeRepository>,
        agents: Arc<dyn AgentRepository>,
        routines: Arc<dyn RoutineRepository>,
        tasks: Arc<dyn TaskRepository>,
        approvals: Arc<dyn ApprovalRepository>,
        settings: Arc<SettingsService>,
        conversations: Arc<ConversationService>,
        events: Arc<dyn EventRepository>,
        event_bus: EventBus,
    ) -> Self {
        Self {
            wakes,
            agents,
            routines,
            tasks,
            approvals,
            settings,
            conversations,
            groups: None,
            events,
            event_bus,
        }
    }

    /// Moves group rounds on as members finish their group turns.
    pub fn with_groups(mut self, groups: Arc<GroupService>) -> Self {
        self.groups = Some(groups);
        self
    }

    /// Queues a wake for the event, if it wakes anyone, and runs whatever an affected
    /// agent can start now.
    pub fn handle(self: &Arc<Self>, event: &DomainEvent) -> AppResult<()> {
        if let Some(wake) = self.wake_for(event)? {
            self.wakes.enqueue(&wake)?;
            tracing::info!(agent_id = %wake.agent_id, chain_depth = wake.chain_depth, "agent wake queued");
            self.drain(wake.agent_id)?;
        }
        match event.event_type {
            EventType::AgentCompleted | EventType::AgentFailed | EventType::AgentCancelled => {
                if let Some(agent_id) = event.aggregate_id {
                    self.finish_group_turn(agent_id, event)?;
                    self.wait_for_pending_approvals(agent_id)?;
                    self.drain(agent_id)?;
                }
            }
            // Messaging and groups queue the wake themselves; the event says there is one
            // to run.
            EventType::AgentMessage | EventType::GroupTurnQueued => {
                if let Some(agent_id) = event.aggregate_id {
                    self.drain(agent_id)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Hands a finished group turn back to its group, which queues the next speaker.
    fn finish_group_turn(&self, agent_id: Uuid, event: &DomainEvent) -> AppResult<()> {
        let Some(groups) = &self.groups else {
            return Ok(());
        };
        let Some(group_id) = payload_uuid(&event.payload, "groupId") else {
            return Ok(());
        };
        let mentioned: Vec<Uuid> = event
            .payload
            .get("mentionedAgentIds")
            .and_then(|value| value.as_array())
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| id.as_str()?.parse().ok())
                    .collect()
            })
            .unwrap_or_default();
        groups.on_turn_finished(agent_id, group_id, &mentioned)
    }

    /// Shows an agent that ended its turn with a pending approval as waiting until the
    /// user decides.
    fn wait_for_pending_approvals(&self, agent_id: Uuid) -> AppResult<()> {
        let pending = self.approvals.list()?.into_iter().any(|approval| {
            approval.agent_id == agent_id && approval.status == ApprovalStatus::Pending
        });
        let Some(mut agent) = self.agents.find(agent_id)? else {
            return Ok(());
        };
        if !pending || agent.status != AgentStatus::Idle {
            return Ok(());
        }
        agent.transition_to(AgentStatus::Waiting)?;
        self.agents.save(&agent)?;
        let event = DomainEvent::new(
            EventType::AgentWaiting,
            Some(agent_id),
            json!({ "agentId": agent_id, "name": agent.name, "reason": "approval" }),
        );
        self.events.append(&event)?;
        self.event_bus.publish(event);
        Ok(())
    }

    /// Runs pending wakes for every agent, e.g. after a restart.
    pub fn drain_all(self: &Arc<Self>) -> AppResult<()> {
        for agent_id in self.wakes.agents_with_pending()? {
            self.drain(agent_id)?;
        }
        Ok(())
    }

    /// Starts the agent's oldest pending wake when it is free. Wakes past the chain limit
    /// are skipped with a notice so a loop between agents ends with the user.
    pub fn drain(self: &Arc<Self>, agent_id: Uuid) -> AppResult<()> {
        if self.conversations.is_running(agent_id) {
            return Ok(());
        }
        let Some(agent) = self.agents.find(agent_id)? else {
            return Ok(());
        };
        if agent.status == AgentStatus::Paused {
            return Ok(());
        }
        let max_chain_turns = self.settings.runtime()?.max_chain_turns;
        while let Some(mut wake) = self.wakes.next_pending(agent_id)? {
            if let Some(group_id) = wake_group(&wake) {
                if !self.group_wake_is_current(&wake, group_id)? {
                    wake.consume(WakeOutcome::Discarded, now())?;
                    self.wakes.save_outcome(&wake)?;
                    continue;
                }
            }
            if wake.exceeds(max_chain_turns) {
                self.skip(&mut wake, max_chain_turns)?;
                continue;
            }
            match self.conversations.start_wake(&wake) {
                Ok(()) => {
                    wake.consume(WakeOutcome::Started, now())?;
                    self.wakes.save_outcome(&wake)?;
                    return Ok(());
                }
                // The user started a turn first; the wake runs after it.
                Err(_) if self.conversations.is_running(agent_id) => return Ok(()),
                Err(error) => {
                    tracing::warn!(agent_id = %agent_id, %error, "agent wake could not start");
                    wake.consume(WakeOutcome::Discarded, now())?;
                    self.wakes.save_outcome(&wake)?;
                    match (wake_group(&wake), &self.groups) {
                        (Some(group_id), Some(groups)) => groups.skip_turn(
                            &wake,
                            group_id,
                            &format!("{}'s turn could not start: {error}", agent.name),
                        )?,
                        _ => self.conversations.append_notice(
                            agent_id,
                            &format!("{} The turn could not start: {error}", wake.origin.notice()),
                        )?,
                    }
                }
            }
        }
        Ok(())
    }

    /// Whether a group wake still belongs to its group's round. Without groups, no group
    /// wake can run.
    fn group_wake_is_current(&self, wake: &Wake, group_id: Uuid) -> AppResult<bool> {
        match &self.groups {
            Some(groups) => groups.is_current_wake(group_id, wake.id),
            None => Ok(false),
        }
    }

    fn skip(&self, wake: &mut Wake, max_chain_turns: u32) -> AppResult<()> {
        wake.consume(WakeOutcome::SkippedChainLimit, now())?;
        self.wakes.save_outcome(wake)?;
        match (wake_group(wake), &self.groups) {
            (Some(group_id), Some(groups)) => {
                let name = self
                    .agents
                    .find(wake.agent_id)?
                    .map_or_else(|| "A member".to_owned(), |agent| agent.name);
                groups.skip_turn(
                    wake,
                    group_id,
                    &format!(
                        "{name} did not answer: {max_chain_turns} turns already ran without a \
                         user message. Send a message to continue."
                    ),
                )?;
            }
            _ => self.conversations.append_notice(
                wake.agent_id,
                &format!(
                    "{} Not started: {max_chain_turns} turns already ran without a user message. \
                     Send a message to continue.",
                    wake.origin.notice()
                ),
            )?,
        }
        let event = DomainEvent::new(
            EventType::AgentWakeSkipped,
            Some(wake.agent_id),
            json!({ "agentId": wake.agent_id, "reason": "chain_limit", "maxChainTurns": max_chain_turns }),
        );
        self.events.append(&event)?;
        self.event_bus.publish(event);
        tracing::warn!(agent_id = %wake.agent_id, "agent wake skipped at the chain limit");
        Ok(())
    }

    /// Maps an event to the wake it causes. The chain depth grows by one whenever an
    /// agent's own turn causes another agent to wake.
    fn wake_for(&self, event: &DomainEvent) -> AppResult<Option<Wake>> {
        let payload_id = |key: &str| {
            event
                .payload
                .get(key)
                .and_then(|value| value.as_str())
                .and_then(|value| value.parse::<Uuid>().ok())
        };
        let caused_by = |agent: Option<Uuid>| {
            agent.map_or(0, |agent_id| self.conversations.chain_depth(agent_id) + 1)
        };
        let wake = match event.event_type {
            EventType::RoutineTriggered => {
                let Some(routine) = event
                    .aggregate_id
                    .map(|id| self.routines.find(id))
                    .transpose()?
                    .flatten()
                else {
                    return Ok(None);
                };
                Some(Wake::new(
                    routine.agent_id,
                    WakeOrigin::Routine {
                        routine_id: routine.id,
                        name: routine.name.clone(),
                    },
                    &routine.instructions,
                    0,
                    now(),
                ))
            }
            EventType::TaskAssigned => {
                let Some(task) = self.find_task(event)? else {
                    return Ok(None);
                };
                match task.assigned_agent_id {
                    // Agents already know about tasks they assign to themselves.
                    Some(assignee) if task.created_by_agent_id != Some(assignee) => {
                        Some(Wake::new(
                            assignee,
                            WakeOrigin::TaskAssigned {
                                task_id: task.id,
                                title: task.title.clone(),
                            },
                            &task.description,
                            caused_by(task.created_by_agent_id),
                            now(),
                        ))
                    }
                    _ => None,
                }
            }
            EventType::TaskCompleted | EventType::TaskFailed => {
                let Some(task) = self.find_task(event)? else {
                    return Ok(None);
                };
                match task.created_by_agent_id {
                    Some(creator) if task.assigned_agent_id != Some(creator) => Some(Wake::new(
                        creator,
                        WakeOrigin::TaskFinished {
                            task_id: task.id,
                            title: task.title.clone(),
                            status: status_word(&task),
                        },
                        task.result.as_deref().unwrap_or_default(),
                        caused_by(task.assigned_agent_id.or(payload_id("agentId"))),
                        now(),
                    )),
                    _ => None,
                }
            }
            // The user's decision starts a new chain.
            EventType::ApprovalApproved | EventType::ApprovalDenied => {
                let Some(approval) = event
                    .aggregate_id
                    .map(|id| self.approvals.find(id))
                    .transpose()?
                    .flatten()
                else {
                    return Ok(None);
                };
                let approved = approval.status == ApprovalStatus::Approved;
                let guidance = if approved {
                    "You may go ahead with it."
                } else {
                    "Do not do it. Choose another approach or ask the user."
                };
                Some(Wake::new(
                    approval.agent_id,
                    WakeOrigin::ApprovalResolved {
                        approval_id: approval.id,
                        approved,
                    },
                    &format!("Requested action: {}\n{guidance}", approval.action),
                    0,
                    now(),
                ))
            }
            _ => None,
        };
        Ok(wake)
    }

    fn find_task(&self, event: &DomainEvent) -> AppResult<Option<Task>> {
        event
            .aggregate_id
            .map(|id| self.tasks.find(id))
            .transpose()
            .map(Option::flatten)
    }
}

fn wake_group(wake: &Wake) -> Option<Uuid> {
    match &wake.origin {
        WakeOrigin::GroupTurn { group_id, .. } => Some(*group_id),
        _ => None,
    }
}

fn payload_uuid(payload: &serde_json::Value, key: &str) -> Option<Uuid> {
    payload.get(key)?.as_str()?.parse().ok()
}

fn status_word(task: &Task) -> String {
    serde_json::to_value(task.status)
        .ok()
        .and_then(|value| value.as_str().map(str::to_owned))
        .unwrap_or_else(|| "finished".into())
}

/// Feeds bus events to the runtime until the bus closes. Pending wakes left from a
/// previous run start first.
pub async fn run_agent_runtime(runtime: Arc<AgentRuntime>, mut receiver: Receiver<DomainEvent>) {
    if let Err(error) = runtime.drain_all() {
        tracing::error!(%error, "pending agent wakes could not be started");
    }
    loop {
        match receiver.recv().await {
            Ok(event) => {
                if let Err(error) = runtime.handle(&event) {
                    tracing::error!(%error, event_id = %event.id, "runtime event was not handled");
                }
            }
            Err(RecvError::Lagged(skipped)) => {
                tracing::warn!(skipped, "agent runtime lagged behind the event bus");
                if let Err(error) = runtime.drain_all() {
                    tracing::error!(%error, "pending agent wakes could not be started");
                }
            }
            Err(RecvError::Closed) => break,
        }
    }
}
