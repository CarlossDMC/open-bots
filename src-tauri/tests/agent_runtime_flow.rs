use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use async_trait::async_trait;
use chrono::Duration as ChronoDuration;
use open_bots_lib::{
    application::{
        run_agent_runtime, AgentRuntime, ApprovalService, ConversationService, MessagingService,
        RoutineService, RuntimeSettings, SettingsService, TaskActor, TaskService,
    },
    domain::{
        agents::{Agent, AgentStatus, IdentityColor, NewAgent},
        approvals::ApprovalStatus,
        conversations::MessageRole,
        events::{DomainEvent, EventType},
        routines::{NewRoutine, RoutineSchedule},
        tasks::{NewTask, TaskStatus},
    },
    error::AppResult,
    infrastructure::database::{
        AgentRepository, Database, SqliteAgentRepository, SqliteApprovalRepository,
        SqliteConversationRepository, SqliteEventRepository, SqliteMemoryRepository,
        SqliteRoutineRepository, SqliteSettingsRepository, SqliteTaskRepository,
        SqliteWakeRepository,
    },
    providers::{
        AgentProvider, DetectionStatus, ProviderCapability, ProviderKind, ProviderRegistry,
        ProviderSummary, TurnEvent, TurnEvents, TurnOutcome, TurnRequest,
    },
    runtime::{cancellation::CancellationSignal, event_bus::EventBus, turn_tokens::TurnContext},
};
use tempfile::TempDir;
use tokio::sync::broadcast;

/// Replies to every turn and records the prompt it received, keyed by workspace.
#[derive(Default)]
struct RecordingProvider {
    prompts: Mutex<Vec<(String, String)>>,
}

#[async_trait]
impl AgentProvider for RecordingProvider {
    fn id(&self) -> &'static str {
        "recording"
    }
    fn name(&self) -> &'static str {
        "Recording Provider"
    }
    async fn detect(&self) -> AppResult<ProviderSummary> {
        Ok(ProviderSummary {
            id: self.id().into(),
            name: self.name().into(),
            kind: ProviderKind::Mock,
            status: DetectionStatus::Available,
            detail: String::new(),
            capabilities: self.capabilities(),
        })
    }
    fn capabilities(&self) -> Vec<ProviderCapability> {
        vec![ProviderCapability::Sessions]
    }
    async fn run_turn(
        &self,
        request: TurnRequest,
        events: TurnEvents,
        _cancellation: CancellationSignal,
    ) -> AppResult<TurnOutcome> {
        self.prompts.lock().expect("prompts").push((
            request.workspace.to_string_lossy().into_owned(),
            request.prompt,
        ));
        let _ = events.send(TurnEvent::Message {
            text: "Done.".into(),
        });
        Ok(TurnOutcome {
            session_id: Some("session".into()),
            cancelled: false,
        })
    }
}

struct TestHarness {
    _directory: TempDir,
    atlas: Agent,
    nova: Agent,
    conversations: Arc<ConversationService>,
    routines: RoutineService,
    tasks: TaskService,
    settings: Arc<SettingsService>,
    approvals: ApprovalService,
    messaging: MessagingService,
    agents: Arc<SqliteAgentRepository>,
    provider: Arc<RecordingProvider>,
    subscriber: broadcast::Receiver<DomainEvent>,
}

fn agent(agents: &SqliteAgentRepository, name: &str, workspace: &str) -> Agent {
    let agent = Agent::create(NewAgent {
        name: name.into(),
        role: "Engineer".into(),
        description: String::new(),
        provider_id: "recording".into(),
        identity_color: IdentityColor::Indigo,
        workspace: workspace.into(),
        instructions: String::new(),
        model_selection: Default::default(),
    })
    .expect("agent");
    agents.save(&agent).expect("save agent");
    agent
}

impl TestHarness {
    fn new() -> Self {
        let directory = tempfile::tempdir().expect("temporary directory");
        let database =
            Arc::new(Database::open(&directory.path().join("runtime.sqlite3")).expect("database"));
        let agents = Arc::new(SqliteAgentRepository::new(Arc::clone(&database)));
        let atlas = agent(&agents, "Atlas", "/work/atlas");
        let nova = agent(&agents, "Nova", "/work/nova");
        let routine_repository = Arc::new(SqliteRoutineRepository::new(Arc::clone(&database)));
        let task_repository = Arc::new(SqliteTaskRepository::new(Arc::clone(&database)));
        let events = Arc::new(SqliteEventRepository::new(Arc::clone(&database)));
        let event_bus = EventBus::new(256);
        let provider = Arc::new(RecordingProvider::default());
        let mut registry = ProviderRegistry::new();
        registry.register(provider.clone()).expect("register");
        let conversations = Arc::new(ConversationService::new(
            agents.clone(),
            Arc::new(SqliteConversationRepository::new(Arc::clone(&database))),
            Arc::new(SqliteMemoryRepository::new(Arc::clone(&database))),
            Arc::new(registry),
            events.clone(),
            event_bus.clone(),
        ));
        let settings = Arc::new(SettingsService::new(Arc::new(
            SqliteSettingsRepository::new(Arc::clone(&database)),
        )));
        let wakes = Arc::new(SqliteWakeRepository::new(Arc::clone(&database)));
        let approval_repository = Arc::new(SqliteApprovalRepository::new(database));
        let runtime = Arc::new(AgentRuntime::new(
            wakes.clone(),
            agents.clone(),
            routine_repository.clone(),
            task_repository.clone(),
            approval_repository.clone(),
            Arc::clone(&settings),
            Arc::clone(&conversations),
            events.clone(),
            event_bus.clone(),
        ));
        tokio::spawn(run_agent_runtime(runtime, event_bus.subscribe()));
        Self {
            _directory: directory,
            routines: RoutineService::new(
                routine_repository,
                agents.clone(),
                events.clone(),
                event_bus.clone(),
            ),
            tasks: TaskService::new(
                task_repository,
                agents.clone(),
                events.clone(),
                event_bus.clone(),
            ),
            approvals: ApprovalService::new(
                approval_repository,
                agents.clone(),
                events.clone(),
                event_bus.clone(),
            ),
            messaging: MessagingService::new(agents.clone(), wakes, events, event_bus.clone()),
            agents,
            subscriber: event_bus.subscribe(),
            atlas,
            nova,
            conversations,
            settings,
            provider,
        }
    }

    /// Waits for the next event of `event_type` about `agent`.
    async fn next(&mut self, event_type: EventType, agent: &Agent) -> DomainEvent {
        let wait = async {
            loop {
                let event = self.subscriber.recv().await.expect("event");
                if event.event_type == event_type && event.aggregate_id == Some(agent.id) {
                    return event;
                }
            }
        };
        tokio::time::timeout(Duration::from_secs(5), wait)
            .await
            .expect("expected event")
    }

    fn prompts_for(&self, agent: &Agent) -> Vec<String> {
        self.provider
            .prompts
            .lock()
            .expect("prompts")
            .iter()
            .filter(|(workspace, _)| *workspace == agent.workspace)
            .map(|(_, prompt)| prompt.clone())
            .collect()
    }

    fn delegated_task(&self) -> NewTask {
        NewTask {
            title: "Review the API".into(),
            description: "Check the pagination contract.".into(),
            assigned_agent_id: Some(self.nova.id),
            created_by_agent_id: Some(self.atlas.id),
            parent_task_id: None,
            workspace_id: String::new(),
        }
    }
}

#[tokio::test]
async fn routines_run_their_instructions_as_a_turn() {
    let mut harness = TestHarness::new();
    let routine = harness
        .routines
        .create(NewRoutine {
            agent_id: harness.atlas.id,
            name: "Daily triage".into(),
            instructions: "Review new issues.".into(),
            schedule: RoutineSchedule::Interval { minutes: 60 },
        })
        .expect("routine");

    harness
        .routines
        .fire_due(routine.next_run_at + ChronoDuration::seconds(1))
        .expect("fire");
    let atlas = harness.atlas.clone();
    harness.next(EventType::AgentCompleted, &atlas).await;

    let prompts = harness.prompts_for(&atlas);
    assert_eq!(prompts.len(), 1);
    assert!(prompts[0].contains("Routine \"Daily triage\" triggered.\n\nReview new issues."));
    let transcript = harness.conversations.list(atlas.id).expect("messages");
    assert_eq!(transcript[0].role, MessageRole::System);
    assert!(transcript[0]
        .content
        .starts_with("Routine \"Daily triage\""));
    assert_eq!(transcript[1].role, MessageRole::Agent);
}

#[tokio::test]
async fn delegation_wakes_the_assignee_and_completion_wakes_the_creator() {
    let mut harness = TestHarness::new();
    let task = harness
        .tasks
        .create(harness.delegated_task())
        .expect("task");
    let (atlas, nova) = (harness.atlas.clone(), harness.nova.clone());
    harness.next(EventType::AgentCompleted, &nova).await;
    assert!(harness.prompts_for(&nova)[0].contains("Task assigned to you: \"Review the API\""));

    harness
        .tasks
        .update_status(
            task.id,
            TaskStatus::Completed,
            Some("Pagination is fine.".into()),
            TaskActor::Agent(nova.id),
        )
        .expect("complete");
    harness.next(EventType::AgentCompleted, &atlas).await;

    let prompt = &harness.prompts_for(&atlas)[0];
    assert!(prompt.contains("A task you delegated is completed"));
    assert!(prompt.contains("Pagination is fine."));
}

#[tokio::test]
async fn wakes_past_the_chain_limit_are_skipped() {
    let mut harness = TestHarness::new();
    harness
        .settings
        .update_runtime(RuntimeSettings { max_chain_turns: 1 })
        .expect("settings");
    let (atlas, nova) = (harness.atlas.clone(), harness.nova.clone());
    // Atlas's user turn is depth 0, so a task it delegates wakes Nova at depth 1.
    harness
        .conversations
        .send(atlas.id, "Plan the API work")
        .expect("send");
    harness.next(EventType::AgentCompleted, &atlas).await;

    harness
        .tasks
        .create(harness.delegated_task())
        .expect("task");
    let skipped = harness.next(EventType::AgentWakeSkipped, &nova).await;

    assert_eq!(skipped.payload["maxChainTurns"], 1);
    assert!(harness.prompts_for(&nova).is_empty());
    let notice = harness.conversations.list(nova.id).expect("messages");
    assert!(notice[0].content.contains("Not started"));
}

#[tokio::test]
async fn wakes_wait_for_a_running_turn() {
    let mut harness = TestHarness::new();
    let nova = harness.nova.clone();
    harness
        .conversations
        .send(nova.id, "Start on the docs")
        .expect("send");
    harness
        .tasks
        .create(NewTask {
            created_by_agent_id: None,
            ..harness.delegated_task()
        })
        .expect("task");

    harness.next(EventType::AgentCompleted, &nova).await;
    harness.next(EventType::AgentCompleted, &nova).await;

    let prompts = harness.prompts_for(&nova);
    assert_eq!(prompts.len(), 2);
    assert!(prompts[0].contains("Start on the docs"));
    assert!(prompts[1].contains("Task assigned to you"));
}

#[tokio::test]
async fn pending_approvals_hold_the_agent_until_the_user_decides() {
    let mut harness = TestHarness::new();
    let atlas = harness.atlas.clone();
    harness
        .conversations
        .send(atlas.id, "Ship the release")
        .expect("send");
    // The approval arrives during the turn, as an approval_request tool call would.
    let approval = harness
        .approvals
        .request(atlas.id, "git push origin main", "Release 1.2")
        .expect("request");
    harness.next(EventType::AgentWaiting, &atlas).await;
    assert_eq!(
        harness
            .agents
            .find(atlas.id)
            .expect("find")
            .expect("agent")
            .status,
        AgentStatus::Waiting
    );

    harness
        .approvals
        .resolve(approval.id, ApprovalStatus::Approved)
        .expect("resolve");
    harness.next(EventType::AgentCompleted, &atlas).await;

    let prompts = harness.prompts_for(&atlas);
    assert_eq!(prompts.len(), 2);
    assert!(prompts[1].contains("was approved"));
    assert!(prompts[1].contains("git push origin main"));
    assert_eq!(
        harness
            .agents
            .find(atlas.id)
            .expect("find")
            .expect("agent")
            .status,
        AgentStatus::Idle
    );
}

#[tokio::test]
async fn agent_messages_wake_the_recipient() {
    let mut harness = TestHarness::new();
    let (atlas, nova) = (harness.atlas.clone(), harness.nova.clone());
    let wake = harness
        .messaging
        .send(
            TurnContext {
                agent_id: atlas.id,
                chain_depth: 2,
                group_id: None,
            },
            nova.id,
            "The schema changed; please rebase.",
            None,
        )
        .expect("send");
    assert_eq!(wake.chain_depth, 3);
    harness.next(EventType::AgentCompleted, &nova).await;

    let prompt = &harness.prompts_for(&nova)[0];
    assert!(prompt.contains("Message from Atlas."));
    assert!(prompt.contains("The schema changed; please rebase."));
}

#[tokio::test]
async fn an_agent_reply_wakes_the_original_sender() {
    let mut harness = TestHarness::new();
    let (atlas, nova) = (harness.atlas.clone(), harness.nova.clone());
    harness
        .messaging
        .send(
            TurnContext {
                agent_id: atlas.id,
                chain_depth: 0,
                group_id: None,
            },
            nova.id,
            "Can you review the schema?",
            None,
        )
        .expect("question");
    harness.next(EventType::AgentCompleted, &nova).await;

    harness
        .messaging
        .send(
            TurnContext {
                agent_id: nova.id,
                chain_depth: 1,
                group_id: None,
            },
            atlas.id,
            "Yes. The schema is ready.",
            None,
        )
        .expect("reply");
    harness.next(EventType::AgentCompleted, &atlas).await;

    let prompt = &harness.prompts_for(&atlas)[0];
    assert!(prompt.contains("Message from Nova."));
    assert!(prompt.contains("Yes. The schema is ready."));
    let transcript = harness.conversations.list(atlas.id).expect("messages");
    assert_eq!(transcript[0].source_agent_id, Some(nova.id));
}
