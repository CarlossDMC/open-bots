use std::sync::Arc;

use serde_json::json;
use tokio::sync::broadcast::{error::RecvError, Receiver};
use uuid::Uuid;

use super::{routine_service::now, ConversationService, SettingsService};
use crate::{
    domain::{
        agents::AgentStatus,
        events::{DomainEvent, EventType},
        inbox::{Wake, WakeOrigin, WakeOutcome},
        tasks::Task,
    },
    error::AppResult,
    infrastructure::database::{
        AgentRepository, EventRepository, RoutineRepository, TaskRepository, WakeRepository,
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
    settings: Arc<SettingsService>,
    conversations: Arc<ConversationService>,
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
            settings,
            conversations,
            events,
            event_bus,
        }
    }

    /// Queues a wake for the event, if it wakes anyone, and runs whatever an affected
    /// agent can start now.
    pub fn handle(self: &Arc<Self>, event: &DomainEvent) -> AppResult<()> {
        if let Some(wake) = self.wake_for(event)? {
            self.wakes.enqueue(&wake)?;
            tracing::info!(agent_id = %wake.agent_id, chain_depth = wake.chain_depth, "agent wake queued");
            self.drain(wake.agent_id)?;
        }
        if matches!(
            event.event_type,
            EventType::AgentCompleted | EventType::AgentFailed | EventType::AgentCancelled
        ) {
            if let Some(agent_id) = event.aggregate_id {
                self.drain(agent_id)?;
            }
        }
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
                    self.conversations.append_notice(
                        agent_id,
                        &format!("{} The turn could not start: {error}", wake.origin.notice()),
                    )?;
                }
            }
        }
        Ok(())
    }

    fn skip(&self, wake: &mut Wake, max_chain_turns: u32) -> AppResult<()> {
        wake.consume(WakeOutcome::SkippedChainLimit, now())?;
        self.wakes.save_outcome(wake)?;
        self.conversations.append_notice(
            wake.agent_id,
            &format!(
                "{} Not started: {max_chain_turns} turns already ran without a user message. \
                 Send a message to continue.",
                wake.origin.notice()
            ),
        )?;
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
