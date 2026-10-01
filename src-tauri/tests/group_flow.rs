use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};

use async_trait::async_trait;
use open_bots_lib::{
    application::{
        run_agent_runtime, AgentRuntime, ConversationService, GroupService, RuntimeSettings,
        SettingsService,
    },
    domain::{
        agents::{Agent, IdentityColor, NewAgent},
        conversations::ConversationScope,
        events::{DomainEvent, EventType},
        groups::{Group, GroupAuthor, NewGroup},
    },
    error::AppResult,
    infrastructure::database::{
        AgentRepository, ConversationRepository, Database, SqliteAgentRepository,
        SqliteApprovalRepository, SqliteConversationRepository, SqliteEventRepository,
        SqliteGroupRepository, SqliteMemoryRepository, SqliteRoutineRepository,
        SqliteSettingsRepository, SqliteTaskRepository, SqliteWakeRepository,
    },
    providers::{
        AgentProvider, DetectionStatus, ProviderCapability, ProviderKind, ProviderRegistry,
        ProviderSummary, TurnEvent, TurnEvents, TurnOutcome, TurnRequest,
    },
    runtime::{cancellation::CancellationSignal, event_bus::EventBus},
};
use tempfile::TempDir;
use tokio::sync::broadcast;

/// Replies with a scripted text per workspace and records each prompt. A workspace scripted
/// as `HANG` waits until its turn is cancelled.
#[derive(Default)]
struct ScriptedProvider {
    replies: Mutex<HashMap<String, String>>,
    prompts: Mutex<Vec<(String, String)>>,
    turns: Mutex<usize>,
}

const HANG: &str = "<hang>";

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
        vec![ProviderCapability::Sessions]
    }
    async fn run_turn(
        &self,
        request: TurnRequest,
        events: TurnEvents,
        mut cancellation: CancellationSignal,
    ) -> AppResult<TurnOutcome> {
        let workspace = request.workspace.to_string_lossy().into_owned();
        self.prompts
            .lock()
            .expect("prompts")
            .push((workspace.clone(), request.prompt));
        let reply = self
            .replies
            .lock()
            .expect("replies")
            .get(&workspace)
            .cloned()
            .unwrap_or_else(|| format!("Reply from {workspace}."));
        if reply == HANG {
            cancellation.cancelled().await;
            return Ok(TurnOutcome {
                session_id: None,
                cancelled: true,
            });
        }
        let _ = events.send(TurnEvent::Message { text: reply });
        let session_id = request.session_id.unwrap_or_else(|| {
            let mut turns = self.turns.lock().expect("turns");
            *turns += 1;
            format!("session-{turns}")
        });
        Ok(TurnOutcome {
            session_id: Some(session_id),
            cancelled: false,
        })
    }
}

struct TestHarness {
    _directory: TempDir,
    atlas: Agent,
    nova: Agent,
    orion: Agent,
    groups: Arc<GroupService>,
    conversations: Arc<ConversationService>,
    sessions: Arc<SqliteConversationRepository>,
    settings: Arc<SettingsService>,
    provider: Arc<ScriptedProvider>,
    subscriber: broadcast::Receiver<DomainEvent>,
}

fn agent(agents: &SqliteAgentRepository, name: &str) -> Agent {
    let agent = Agent::create(NewAgent {
        name: name.into(),
        role: "Engineer".into(),
        description: String::new(),
        provider_id: "scripted".into(),
        identity_color: IdentityColor::Indigo,
        workspace: format!("/work/{name}"),
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
            Arc::new(Database::open(&directory.path().join("groups.sqlite3")).expect("database"));
        let agents = Arc::new(SqliteAgentRepository::new(Arc::clone(&database)));
        let (atlas, nova, orion) = (
            agent(&agents, "Atlas"),
            agent(&agents, "Nova"),
            agent(&agents, "Orion"),
        );
        let events = Arc::new(SqliteEventRepository::new(Arc::clone(&database)));
        let event_bus = EventBus::new(256);
        let provider = Arc::new(ScriptedProvider::default());
        let mut registry = ProviderRegistry::new();
        registry.register(provider.clone()).expect("register");
        let sessions = Arc::new(SqliteConversationRepository::new(Arc::clone(&database)));
        let group_repository = Arc::new(SqliteGroupRepository::new(Arc::clone(&database)));
        let conversations = Arc::new(
            ConversationService::new(
                agents.clone(),
                sessions.clone(),
                Arc::new(SqliteMemoryRepository::new(Arc::clone(&database))),
                Arc::new(registry),
                events.clone(),
                event_bus.clone(),
            )
            .with_groups(group_repository.clone()),
        );
        let settings = Arc::new(SettingsService::new(Arc::new(
            SqliteSettingsRepository::new(Arc::clone(&database)),
        )));
        let wakes = Arc::new(SqliteWakeRepository::new(Arc::clone(&database)));
        let groups = Arc::new(GroupService::new(
            group_repository,
            agents.clone(),
            wakes.clone(),
            Arc::clone(&conversations),
            events.clone(),
            event_bus.clone(),
        ));
        let runtime = Arc::new(
            AgentRuntime::new(
                wakes,
                agents.clone(),
                Arc::new(SqliteRoutineRepository::new(Arc::clone(&database))),
                Arc::new(SqliteTaskRepository::new(Arc::clone(&database))),
                Arc::new(SqliteApprovalRepository::new(database)),
                Arc::clone(&settings),
                Arc::clone(&conversations),
                events,
                event_bus.clone(),
            )
            .with_groups(Arc::clone(&groups)),
        );
        let subscriber = event_bus.subscribe();
        tokio::spawn(run_agent_runtime(runtime, event_bus.subscribe()));
        Self {
            _directory: directory,
            atlas,
            nova,
            orion,
            groups,
            conversations,
            sessions,
            settings,
            provider,
            subscriber,
        }
    }

    fn script(&self, agent: &Agent, reply: &str) {
        self.provider
            .replies
            .lock()
            .expect("replies")
            .insert(agent.workspace.clone(), reply.into());
    }

    fn group(&self, members: &[&Agent]) -> Group {
        self.groups
            .create(NewGroup {
                name: "Release".into(),
                topic: "Ship version 0.4".into(),
                member_ids: members.iter().map(|agent| agent.id).collect(),
            })
            .expect("group")
    }

    /// Waits for the next event of `event_type` about `aggregate`.
    async fn next(&mut self, event_type: EventType, aggregate: uuid::Uuid) -> DomainEvent {
        let wait = async {
            loop {
                let event = self.subscriber.recv().await.expect("event");
                if event.event_type == event_type && event.aggregate_id == Some(aggregate) {
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

    /// Authors and contents of the group's messages, oldest first.
    fn transcript(&self, group: &Group) -> Vec<(GroupAuthor, String)> {
        self.groups
            .messages(group.id)
            .expect("messages")
            .into_iter()
            .map(|message| (message.author, message.content))
            .collect()
    }
}

fn by(agent: &Agent) -> GroupAuthor {
    GroupAuthor::Agent { agent_id: agent.id }
}

#[tokio::test]
async fn members_answer_in_order_and_see_earlier_replies() {
    let mut harness = TestHarness::new();
    let group = harness.group(&[&harness.atlas, &harness.nova, &harness.orion]);
    harness.script(&harness.atlas, "The build is green.");

    harness
        .groups
        .post_user_message(group.id, "Status?")
        .expect("post");
    harness.next(EventType::GroupRoundCompleted, group.id).await;

    assert_eq!(
        harness.transcript(&group),
        vec![
            (GroupAuthor::User, "Status?".into()),
            (by(&harness.atlas), "The build is green.".into()),
            (by(&harness.nova), "Reply from /work/Nova.".into()),
            (by(&harness.orion), "Reply from /work/Orion.".into()),
        ]
    );
    let atlas = harness.prompts_for(&harness.atlas);
    assert!(atlas[0].contains("<group>"), "{}", atlas[0]);
    assert!(atlas[0].contains("Topic: Ship version 0.4"));
    assert!(atlas[0].contains("User: Status?"));
    let nova = harness.prompts_for(&harness.nova);
    assert!(
        nova[0].contains("Atlas: The build is green."),
        "{}",
        nova[0]
    );
    // Group turns stay out of the members' own conversations.
    assert!(harness
        .conversations
        .list(harness.atlas.id)
        .expect("direct")
        .is_empty());
    assert!(!harness
        .groups
        .find(group.id)
        .expect("group")
        .round
        .is_active());
}

#[tokio::test]
async fn mentions_choose_who_answers_and_chain_from_agent_replies() {
    let mut harness = TestHarness::new();
    let group = harness.group(&[&harness.atlas, &harness.nova, &harness.orion]);
    harness.script(&harness.atlas, "@Orion can you check the migration?");

    harness
        .groups
        .post_user_message(group.id, "@atlas please start")
        .expect("post");
    harness.next(EventType::GroupRoundCompleted, group.id).await;

    let authors: Vec<GroupAuthor> = harness
        .transcript(&group)
        .into_iter()
        .map(|(author, _)| author)
        .collect();
    assert_eq!(
        authors,
        vec![GroupAuthor::User, by(&harness.atlas), by(&harness.orion)]
    );
    assert!(harness.prompts_for(&harness.nova).is_empty());
    assert!(harness.prompts_for(&harness.orion)[0]
        .contains("Atlas: @Orion can you check the migration?"));
}

#[tokio::test]
async fn the_chain_limit_ends_agent_to_agent_turns_in_the_group() {
    let mut harness = TestHarness::new();
    harness
        .settings
        .update_runtime(RuntimeSettings { max_chain_turns: 1 })
        .expect("settings");
    let group = harness.group(&[&harness.atlas, &harness.nova]);
    harness.script(&harness.atlas, "@Nova over to you");

    harness
        .groups
        .post_user_message(group.id, "@Atlas go")
        .expect("post");
    harness.next(EventType::GroupRoundCompleted, group.id).await;

    let transcript = harness.transcript(&group);
    assert_eq!(transcript.len(), 3);
    assert_eq!(transcript[2].0, GroupAuthor::System);
    assert!(
        transcript[2].1.starts_with("Nova did not answer"),
        "{}",
        transcript[2].1
    );
    assert!(harness.prompts_for(&harness.nova).is_empty());
}

#[tokio::test]
async fn stopping_a_group_cancels_the_speaker_and_drops_the_rest_of_the_round() {
    let mut harness = TestHarness::new();
    let group = harness.group(&[&harness.atlas, &harness.nova]);
    harness.script(&harness.atlas, HANG);

    harness
        .groups
        .post_user_message(group.id, "Status?")
        .expect("post");
    harness
        .next(EventType::AgentStarted, harness.atlas.id)
        .await;
    harness.groups.stop(group.id).expect("stop");
    harness
        .next(EventType::AgentCancelled, harness.atlas.id)
        .await;

    assert_eq!(
        harness.transcript(&group).last(),
        Some(&(GroupAuthor::System, "Stopped.".to_string()))
    );
    assert!(!harness
        .groups
        .find(group.id)
        .expect("group")
        .round
        .is_active());
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(harness.prompts_for(&harness.nova).is_empty());
}

#[tokio::test]
async fn each_group_keeps_its_own_provider_session() {
    let mut harness = TestHarness::new();
    let group = harness.group(&[&harness.atlas]);

    harness
        .groups
        .post_user_message(group.id, "Kickoff")
        .expect("post");
    harness.next(EventType::GroupRoundCompleted, group.id).await;
    harness
        .conversations
        .send(harness.atlas.id, "Hello directly")
        .expect("send");
    harness
        .next(EventType::AgentCompleted, harness.atlas.id)
        .await;

    let in_group = harness
        .sessions
        .provider_session(
            harness.atlas.id,
            "scripted",
            ConversationScope::Group(group.id),
        )
        .expect("group session");
    let direct = harness
        .sessions
        .provider_session(harness.atlas.id, "scripted", ConversationScope::Direct)
        .expect("direct session");
    assert!(in_group.is_some() && direct.is_some());
    assert_ne!(in_group, direct);
    // The direct conversation started its own session with the full first-turn context.
    let prompts = harness.prompts_for(&harness.atlas);
    assert!(prompts[1].starts_with("<agent_context>"));
    assert!(!prompts[1].contains("<group>"));
}
