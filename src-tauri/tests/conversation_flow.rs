use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use async_trait::async_trait;
use open_bots_lib::{
    application::{ConversationService, MemoryService},
    domain::{
        agents::{Agent, AgentStatus, IdentityColor, ModelSelection, NewAgent, WorkspaceAccess},
        conversations::{ConversationScope, MessageRole},
        events::{DomainEvent, EventType},
    },
    error::{AppError, AppResult},
    infrastructure::database::{
        AgentRepository, ConversationRepository, Database, SqliteAgentRepository,
        SqliteConversationRepository, SqliteEventRepository, SqliteMemoryRepository,
    },
    providers::{
        AgentProvider, DetectionStatus, ProviderCapability, ProviderKind, ProviderRegistry,
        ProviderSummary, TurnEvent, TurnEvents, TurnOutcome, TurnRequest,
    },
    runtime::{cancellation::CancellationSignal, event_bus::EventBus},
};
use tempfile::TempDir;
use tokio::sync::broadcast;

#[derive(Clone, Copy)]
enum Behavior {
    Reply,
    Fail,
    WaitForCancel,
}

/// Deterministic provider that records every request it receives.
struct ScriptedProvider {
    behavior: Mutex<Behavior>,
    requests: Mutex<Vec<TurnRequest>>,
}

#[async_trait]
impl AgentProvider for ScriptedProvider {
    fn id(&self) -> &'static str {
        "scripted"
    }
    fn name(&self) -> &'static str {
        "Scripted Provider"
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
        vec![ProviderCapability::Sessions, ProviderCapability::Resume]
    }
    async fn run_turn(
        &self,
        request: TurnRequest,
        events: TurnEvents,
        mut cancellation: CancellationSignal,
    ) -> AppResult<TurnOutcome> {
        let behavior = *self.behavior.lock().expect("behavior");
        let session_id = request
            .session_id
            .clone()
            .unwrap_or_else(|| "session-1".into());
        self.requests.lock().expect("requests").push(request);
        let _ = events.send(TurnEvent::SessionStarted {
            session_id: session_id.clone(),
        });
        match behavior {
            Behavior::Reply => {
                let _ = events.send(TurnEvent::ActionStarted {
                    id: "a1".into(),
                    summary: "ls".into(),
                });
                let _ = events.send(TurnEvent::ActionCompleted {
                    id: "a1".into(),
                    summary: "ls".into(),
                    succeeded: true,
                });
                let _ = events.send(TurnEvent::Message {
                    text: "Here is the answer.".into(),
                });
                Ok(TurnOutcome {
                    session_id: Some(session_id),
                    cancelled: false,
                })
            }
            Behavior::Fail => Err(AppError::Provider("usage limit reached".into())),
            Behavior::WaitForCancel => {
                cancellation.cancelled().await;
                Ok(TurnOutcome {
                    session_id: Some(session_id),
                    cancelled: true,
                })
            }
        }
    }
}

struct TestHarness {
    _directory: TempDir,
    agent: Agent,
    agents: Arc<SqliteAgentRepository>,
    conversations: Arc<SqliteConversationRepository>,
    service: Arc<ConversationService>,
    memories: MemoryService,
    provider: Arc<ScriptedProvider>,
    subscriber: broadcast::Receiver<DomainEvent>,
}

impl TestHarness {
    fn new(behavior: Behavior) -> Self {
        let directory = tempfile::tempdir().expect("temporary directory");
        let database = Arc::new(
            Database::open(&directory.path().join("conversation.sqlite3")).expect("database"),
        );
        let agents = Arc::new(SqliteAgentRepository::new(Arc::clone(&database)));
        let conversations = Arc::new(SqliteConversationRepository::new(Arc::clone(&database)));
        let memory_repository = Arc::new(SqliteMemoryRepository::new(Arc::clone(&database)));
        let events = Arc::new(SqliteEventRepository::new(database));
        let event_bus = EventBus::new(256);
        let provider = Arc::new(ScriptedProvider {
            behavior: Mutex::new(behavior),
            requests: Mutex::new(Vec::new()),
        });
        let mut registry = ProviderRegistry::new();
        registry.register(provider.clone()).expect("register");
        let agent = Agent::create(NewAgent {
            name: "Atlas".into(),
            role: "Backend Engineer".into(),
            description: String::new(),
            provider_id: "scripted".into(),
            identity_color: IdentityColor::Indigo,
            workspace: directory.path().to_string_lossy().into_owned(),
            instructions: "Prefer small changes.".into(),
            model_selection: ModelSelection::new(Some("model-a".into()), Some("high".into()))
                .expect("model"),
        })
        .expect("agent");
        agents.save(&agent).expect("save agent");
        let subscriber = event_bus.subscribe();
        Self {
            agent,
            service: Arc::new(ConversationService::new(
                agents.clone(),
                conversations.clone(),
                memory_repository.clone(),
                Arc::new(registry),
                events.clone(),
                event_bus.clone(),
            )),
            memories: MemoryService::new(memory_repository, agents.clone(), events, event_bus),
            agents,
            conversations,
            provider,
            subscriber,
            _directory: directory,
        }
    }

    /// Waits for the turn to end and returns the final agent event type.
    async fn finished_turn(&mut self) -> EventType {
        let wait = async {
            loop {
                let event = self.subscriber.recv().await.expect("event");
                if matches!(
                    event.event_type,
                    EventType::AgentCompleted | EventType::AgentFailed | EventType::AgentCancelled
                ) {
                    return event.event_type;
                }
            }
        };
        tokio::time::timeout(Duration::from_secs(5), wait)
            .await
            .expect("turn finished")
    }

    fn status(&self) -> AgentStatus {
        self.agents
            .find(self.agent.id)
            .expect("find")
            .expect("agent")
            .status
    }

    fn transcript(&self) -> Vec<(MessageRole, String)> {
        self.service
            .list(self.agent.id)
            .expect("messages")
            .into_iter()
            .map(|message| (message.role, message.content))
            .collect()
    }
}

#[tokio::test]
async fn runs_a_turn_and_resumes_the_session_next_time() {
    let mut harness = TestHarness::new(Behavior::Reply);
    harness
        .memories
        .add(harness.agent.id, "Uses pnpm")
        .expect("memory");

    let sent = harness
        .service
        .send(harness.agent.id, "What is in the repo?")
        .expect("send");
    assert_eq!(sent.role, MessageRole::User);
    assert_eq!(harness.status(), AgentStatus::Working);
    assert_eq!(harness.finished_turn().await, EventType::AgentCompleted);

    assert_eq!(harness.status(), AgentStatus::Idle);
    assert_eq!(
        harness.transcript(),
        [
            (MessageRole::User, "What is in the repo?".into()),
            (MessageRole::Agent, "Here is the answer.".into())
        ]
    );
    assert_eq!(
        harness
            .conversations
            .provider_session(harness.agent.id, "scripted", ConversationScope::Direct)
            .expect("session"),
        Some("session-1".into())
    );

    harness
        .service
        .send(harness.agent.id, "Thanks")
        .expect("send");
    harness.finished_turn().await;

    let requests = harness.provider.requests.lock().expect("requests");
    assert_eq!(requests[0].session_id, None);
    assert!(requests[0].prompt.contains("Prefer small changes."));
    assert!(requests[0].prompt.contains("- Uses pnpm"));
    assert!(requests[0].prompt.ends_with("What is in the repo?"));
    // New agents may write to their workspace and use the internet.
    assert_eq!(requests[0].access, WorkspaceAccess::WorkspaceWrite);
    assert!(requests[0].network);
    assert_eq!(requests[0].model.as_deref(), Some("model-a"));
    assert_eq!(requests[0].reasoning_effort.as_deref(), Some("high"));
    assert_eq!(requests[1].session_id.as_deref(), Some("session-1"));
    assert_eq!(requests[1].prompt, "Thanks");
}

#[tokio::test]
async fn records_failures_and_recovers_on_the_next_message() {
    let mut harness = TestHarness::new(Behavior::Fail);
    harness.service.send(harness.agent.id, "Hi").expect("send");
    assert_eq!(harness.finished_turn().await, EventType::AgentFailed);

    assert_eq!(harness.status(), AgentStatus::Failed);
    let transcript = harness.transcript();
    assert_eq!(transcript[1].0, MessageRole::System);
    assert!(transcript[1].1.contains("usage limit reached"));

    *harness.provider.behavior.lock().expect("behavior") = Behavior::Reply;
    harness
        .service
        .send(harness.agent.id, "Try again")
        .expect("send after failure");
    assert_eq!(harness.finished_turn().await, EventType::AgentCompleted);
    assert_eq!(harness.status(), AgentStatus::Idle);
}

#[tokio::test]
async fn rejects_concurrent_turns_and_cancels_the_running_one() {
    let mut harness = TestHarness::new(Behavior::WaitForCancel);
    harness
        .service
        .send(harness.agent.id, "Long task")
        .expect("send");

    let busy = harness.service.send(harness.agent.id, "Another");
    assert!(
        matches!(busy, Err(AppError::Validation(message)) if message.contains("already working"))
    );

    harness.service.cancel(harness.agent.id).expect("cancel");
    assert_eq!(harness.finished_turn().await, EventType::AgentCancelled);
    assert_eq!(harness.status(), AgentStatus::Idle);
    assert_eq!(
        harness.transcript().last(),
        Some(&(MessageRole::System, "Stopped.".into()))
    );
    assert!(matches!(
        harness.service.cancel(harness.agent.id),
        Err(AppError::Validation(_))
    ));
}

#[tokio::test]
async fn rejects_empty_messages_without_starting_a_turn() {
    let harness = TestHarness::new(Behavior::Reply);

    let result = harness.service.send(harness.agent.id, "   ");

    assert!(matches!(result, Err(AppError::Domain(_))));
    assert_eq!(harness.status(), AgentStatus::Idle);
    assert!(harness.transcript().is_empty());
}

#[tokio::test]
async fn marks_turns_interrupted_by_a_restart_as_failed() {
    let harness = TestHarness::new(Behavior::Reply);
    let mut agent = harness.agent.clone();
    agent.transition_to(AgentStatus::Working).expect("working");
    harness.agents.save(&agent).expect("save");

    assert_eq!(harness.service.recover_interrupted().expect("recover"), 1);

    assert_eq!(harness.status(), AgentStatus::Failed);
    assert!(harness.transcript()[0].1.starts_with("Interrupted"));
    assert_eq!(harness.service.recover_interrupted().expect("recover"), 0);
}

#[tokio::test]
async fn resets_the_session_so_the_next_turn_starts_fresh() {
    let mut harness = TestHarness::new(Behavior::Reply);
    harness
        .service
        .send(harness.agent.id, "Remember this")
        .expect("send");
    harness.finished_turn().await;

    harness
        .service
        .reset_session(harness.agent.id)
        .expect("reset");
    assert_eq!(
        harness
            .conversations
            .provider_session(harness.agent.id, "scripted", ConversationScope::Direct)
            .expect("session"),
        None
    );
    assert!(matches!(
        harness.transcript().last(),
        Some((MessageRole::System, notice)) if notice.starts_with("New session started.")
    ));

    harness
        .service
        .send(harness.agent.id, "Start over")
        .expect("send after reset");
    harness.finished_turn().await;
    let requests = harness.provider.requests.lock().expect("requests");
    assert_eq!(requests[1].session_id, None);
    assert!(requests[1].prompt.contains("Prefer small changes."));
    assert!(requests[1].prompt.ends_with("Start over"));
}

#[tokio::test]
async fn rejects_a_session_reset_while_the_agent_is_working() {
    let mut harness = TestHarness::new(Behavior::WaitForCancel);
    harness
        .service
        .send(harness.agent.id, "Long task")
        .expect("send");

    assert!(matches!(
        harness.service.reset_session(harness.agent.id),
        Err(AppError::Validation(message)) if message.contains("stop the turn")
    ));

    harness.service.cancel(harness.agent.id).expect("cancel");
    harness.finished_turn().await;
}
