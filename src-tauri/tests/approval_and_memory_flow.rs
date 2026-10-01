use std::sync::Arc;

use open_bots_lib::{
    application::{ActivityService, ApprovalService, MemoryService},
    domain::{
        agents::{Agent, IdentityColor, NewAgent},
        approvals::ApprovalStatus,
        events::EventType,
    },
    error::AppError,
    infrastructure::database::{
        AgentRepository, ApprovalRepository, Database, SqliteAgentRepository,
        SqliteApprovalRepository, SqliteEventRepository, SqliteMemoryRepository,
    },
    runtime::event_bus::EventBus,
};
use tempfile::TempDir;
use uuid::Uuid;

struct TestHarness {
    _directory: TempDir,
    agent: Agent,
    approvals: ApprovalService,
    approval_repository: Arc<SqliteApprovalRepository>,
    memories: MemoryService,
    activity: ActivityService,
    event_bus: EventBus,
}

impl TestHarness {
    fn new() -> Self {
        let directory = tempfile::tempdir().expect("temporary directory");
        let database = Arc::new(
            Database::open(&directory.path().join("flow.sqlite3")).expect("test database"),
        );
        let agents = Arc::new(SqliteAgentRepository::new(Arc::clone(&database)));
        let approval_repository = Arc::new(SqliteApprovalRepository::new(Arc::clone(&database)));
        let memory_repository = Arc::new(SqliteMemoryRepository::new(Arc::clone(&database)));
        let events = Arc::new(SqliteEventRepository::new(database));
        let event_bus = EventBus::new(16);
        let agent = Agent::create(NewAgent {
            name: "Atlas".into(),
            role: "Backend Engineer".into(),
            description: String::new(),
            provider_id: "mock".into(),
            identity_color: IdentityColor::Indigo,
            workspace: "/tmp/open-bots-workspace".into(),
            instructions: String::new(),
            model_selection: Default::default(),
        })
        .expect("agent");
        agents.save(&agent).expect("save agent");
        Self {
            _directory: directory,
            agent,
            approvals: ApprovalService::new(
                approval_repository.clone(),
                agents.clone(),
                events.clone(),
                event_bus.clone(),
            ),
            approval_repository,
            memories: MemoryService::new(
                memory_repository,
                agents,
                events.clone(),
                event_bus.clone(),
            ),
            activity: ActivityService::new(events),
            event_bus,
        }
    }

    fn event_types(&self) -> Vec<EventType> {
        self.activity
            .recent(10)
            .expect("activity")
            .into_iter()
            .map(|event| event.event_type)
            .collect()
    }
}

#[test]
fn requests_and_resolves_an_approval_with_persisted_events() {
    let harness = TestHarness::new();
    let mut subscriber = harness.event_bus.subscribe();

    let requested = harness
        .approvals
        .request(harness.agent.id, "git push origin main", "Ship the fix")
        .expect("approval request");
    let resolved = harness
        .approvals
        .resolve(requested.id, ApprovalStatus::Approved)
        .expect("approval resolution");

    assert_eq!(resolved.status, ApprovalStatus::Approved);
    assert_eq!(
        harness
            .approval_repository
            .find(requested.id)
            .expect("find"),
        Some(resolved)
    );
    let published = subscriber.try_recv().expect("requested event");
    assert_eq!(published.event_type, EventType::ApprovalRequested);
    assert_eq!(published.payload["agentId"], harness.agent.id.to_string());
    assert_eq!(
        subscriber.try_recv().expect("resolved event").event_type,
        EventType::ApprovalApproved
    );
    let types = harness.event_types();
    assert!(types.contains(&EventType::ApprovalRequested));
    assert!(types.contains(&EventType::ApprovalApproved));
}

#[test]
fn rejects_a_second_decision_without_writing_events() {
    let harness = TestHarness::new();
    let approval = harness
        .approvals
        .request(harness.agent.id, "git push", "Ship the fix")
        .expect("approval request");
    harness
        .approvals
        .resolve(approval.id, ApprovalStatus::Denied)
        .expect("first decision");

    let result = harness
        .approvals
        .resolve(approval.id, ApprovalStatus::Approved);

    assert!(matches!(result, Err(AppError::Domain(_))));
    assert_eq!(harness.activity.recent(10).expect("activity").len(), 2);
}

#[test]
fn rejects_approvals_for_unknown_agents() {
    let harness = TestHarness::new();

    let result = harness
        .approvals
        .request(Uuid::new_v4(), "git push", "reason");

    assert!(matches!(result, Err(AppError::NotFound(_))));
    assert!(harness.approvals.list().expect("approvals").is_empty());
}

#[test]
fn adds_and_removes_agent_memories() {
    let harness = TestHarness::new();

    let memory = harness
        .memories
        .add(harness.agent.id, "Prefers small pull requests")
        .expect("memory");
    assert_eq!(
        harness.memories.list(harness.agent.id).expect("list"),
        vec![memory.clone()]
    );

    harness.memories.remove(memory.id).expect("remove");

    assert!(harness
        .memories
        .list(harness.agent.id)
        .expect("list")
        .is_empty());
    let events = harness.activity.recent(10).expect("activity");
    assert_eq!(events.len(), 2);
    assert!(events
        .iter()
        .all(|event| event.payload.get("content").is_none()));
    assert!(matches!(
        harness.memories.remove(memory.id),
        Err(AppError::NotFound(_))
    ));
}

#[test]
fn rejects_empty_memories_without_writing_events() {
    let harness = TestHarness::new();

    let result = harness.memories.add(harness.agent.id, "   ");

    assert!(matches!(result, Err(AppError::Domain(_))));
    assert!(harness.activity.recent(10).expect("activity").is_empty());
}
