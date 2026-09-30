use std::{path::Path, sync::Arc};

use open_bots_lib::{
    application::AgentService,
    domain::{
        agents::{IdentityColor, NewAgent},
        events::EventType,
    },
    error::AppError,
    infrastructure::database::{
        AgentRepository, Database, EventRepository, SqliteAgentRepository, SqliteEventRepository,
    },
    providers::{MockProvider, ProviderRegistry},
    runtime::event_bus::EventBus,
};
use tempfile::TempDir;

struct TestHarness {
    _directory: TempDir,
    database_path: std::path::PathBuf,
    service: AgentService,
    agents: Arc<SqliteAgentRepository>,
    events: Arc<SqliteEventRepository>,
    event_bus: EventBus,
}

impl TestHarness {
    fn new() -> Self {
        let directory = tempfile::tempdir().expect("temporary directory");
        let database_path = directory.path().join("application-flow.sqlite3");
        let database = Arc::new(Database::open(&database_path).expect("test database"));
        let agents = Arc::new(SqliteAgentRepository::new(Arc::clone(&database)));
        let events = Arc::new(SqliteEventRepository::new(database));
        let event_bus = EventBus::new(16);
        let mut registry = ProviderRegistry::new();
        registry
            .register(Arc::new(MockProvider))
            .expect("mock provider registration");
        let service = AgentService::new(
            agents.clone(),
            events.clone(),
            event_bus.clone(),
            Arc::new(registry),
        );
        Self {
            _directory: directory,
            database_path,
            service,
            agents,
            events,
            event_bus,
        }
    }
}

fn agent_input(provider_id: &str) -> NewAgent {
    NewAgent {
        name: "Atlas".into(),
        role: "Backend Engineer".into(),
        description: "Builds durable runtime services.".into(),
        provider_id: provider_id.into(),
        identity_color: IdentityColor::Indigo,
        workspace: "/tmp/open-bots-workspace".into(),
        instructions: "Prefer small, testable changes.".into(),
    }
}

#[test]
fn creates_an_agent_and_persists_its_activity() {
    let harness = TestHarness::new();

    let created = harness
        .service
        .create(agent_input("mock"))
        .expect("agent creation");

    let reopened_database = Arc::new(
        Database::open(Path::new(&harness.database_path)).expect("reopened test database"),
    );
    let reopened_agents = SqliteAgentRepository::new(Arc::clone(&reopened_database));
    let reopened_events = SqliteEventRepository::new(reopened_database);

    let persisted = reopened_agents
        .find(created.id)
        .expect("agent lookup")
        .expect("persisted agent");
    let events = reopened_events.list_recent(10).expect("persisted events");

    assert_eq!(persisted, created);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_type, EventType::AgentCreated);
    assert_eq!(events[0].aggregate_id, Some(created.id));
    assert_eq!(events[0].payload["name"], "Atlas");
    assert_eq!(events[0].payload["providerId"], "mock");
}

#[test]
fn publishes_the_persisted_event_to_runtime_subscribers() {
    let harness = TestHarness::new();
    let mut subscriber = harness.event_bus.subscribe();

    let created = harness
        .service
        .create(agent_input("mock"))
        .expect("agent creation");
    let published = subscriber.try_recv().expect("published event");

    assert_eq!(published.event_type, EventType::AgentCreated);
    assert_eq!(published.aggregate_id, Some(created.id));
    assert_eq!(published.payload["name"], created.name);
}

#[test]
fn rejects_an_unknown_provider_without_writing_state() {
    let harness = TestHarness::new();

    let result = harness.service.create(agent_input("missing-provider"));

    assert!(matches!(result, Err(AppError::NotFound(_))));
    assert!(harness.agents.list().expect("agent list").is_empty());
    assert!(harness
        .events
        .list_recent(10)
        .expect("event list")
        .is_empty());
}

#[test]
fn rejects_invalid_agent_input_without_writing_state() {
    let harness = TestHarness::new();
    let mut input = agent_input("mock");
    input.name = " ".into();

    let result = harness.service.create(input);

    assert!(matches!(result, Err(AppError::Domain(_))));
    assert!(harness.agents.list().expect("agent list").is_empty());
    assert!(harness
        .events
        .list_recent(10)
        .expect("event list")
        .is_empty());
}
