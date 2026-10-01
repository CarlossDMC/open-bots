use std::sync::Arc;

use open_bots_lib::{
    application::{TaskActor, TaskService},
    domain::{
        agents::{Agent, IdentityColor, NewAgent},
        events::EventType,
        tasks::{NewTask, TaskStatus},
    },
    error::AppError,
    infrastructure::database::{
        AgentRepository, Database, SqliteAgentRepository, SqliteEventRepository,
        SqliteTaskRepository,
    },
    runtime::event_bus::EventBus,
};
use tempfile::TempDir;

struct TestHarness {
    _directory: TempDir,
    atlas: Agent,
    nova: Agent,
    tasks: TaskService,
    event_bus: EventBus,
}

fn agent(agents: &SqliteAgentRepository, name: &str, workspace: &str) -> Agent {
    let agent = Agent::create(NewAgent {
        name: name.into(),
        role: "Engineer".into(),
        description: String::new(),
        provider_id: "mock".into(),
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
        let database = Arc::new(
            Database::open(&directory.path().join("tasks.sqlite3")).expect("test database"),
        );
        let agents = Arc::new(SqliteAgentRepository::new(Arc::clone(&database)));
        let atlas = agent(&agents, "Atlas", "/work/atlas");
        let nova = agent(&agents, "Nova", "/work/nova");
        let event_bus = EventBus::new(64);
        let tasks = TaskService::new(
            Arc::new(SqliteTaskRepository::new(Arc::clone(&database))),
            agents,
            Arc::new(SqliteEventRepository::new(database)),
            event_bus.clone(),
        );
        Self {
            _directory: directory,
            atlas,
            nova,
            tasks,
            event_bus,
        }
    }

    fn delegate(&self) -> NewTask {
        NewTask {
            title: "Review the API contract".into(),
            description: "Check pagination".into(),
            assigned_agent_id: Some(self.nova.id),
            created_by_agent_id: Some(self.atlas.id),
            parent_task_id: None,
            workspace_id: String::new(),
        }
    }
}

#[test]
fn delegated_tasks_publish_created_and_assigned_events() {
    let harness = TestHarness::new();
    let mut subscriber = harness.event_bus.subscribe();

    let task = harness.tasks.create(harness.delegate()).expect("task");

    assert_eq!(task.status, TaskStatus::Queued);
    assert_eq!(task.workspace_id, "/work/nova");
    let created = subscriber.try_recv().expect("created");
    assert_eq!(created.event_type, EventType::TaskCreated);
    let assigned = subscriber.try_recv().expect("assigned");
    assert_eq!(assigned.event_type, EventType::TaskAssigned);
    assert_eq!(assigned.payload["agentId"], harness.nova.id.to_string());
    assert_eq!(
        assigned.payload["createdByAgentId"],
        harness.atlas.id.to_string()
    );
    assert!(assigned.payload.get("description").is_none());
}

#[test]
fn only_the_assignee_creator_or_user_can_change_a_task() {
    let harness = TestHarness::new();
    let task = harness.tasks.create(harness.delegate()).expect("task");
    let outsider = {
        let mut input = harness.delegate();
        input.assigned_agent_id = None;
        input.created_by_agent_id = None;
        harness.tasks.create(input).expect("unrelated task")
    };

    let started = harness
        .tasks
        .update_status(
            task.id,
            TaskStatus::Running,
            None,
            TaskActor::Agent(harness.nova.id),
        )
        .expect("assignee starts");
    assert_eq!(started.status, TaskStatus::Running);

    let denied = harness.tasks.update_status(
        outsider.id,
        TaskStatus::Cancelled,
        None,
        TaskActor::Agent(harness.nova.id),
    );
    assert!(matches!(denied, Err(AppError::Validation(_))));

    let mut subscriber = harness.event_bus.subscribe();
    harness
        .tasks
        .update_status(
            task.id,
            TaskStatus::Completed,
            Some("Contract approved".into()),
            TaskActor::Agent(harness.nova.id),
        )
        .expect("complete");
    assert_eq!(
        subscriber.try_recv().expect("completed").event_type,
        EventType::TaskCompleted
    );
    harness
        .tasks
        .update_status(outsider.id, TaskStatus::Cancelled, None, TaskActor::User)
        .expect("user cancels");
    assert_eq!(
        harness
            .tasks
            .list_for_agent(harness.atlas.id)
            .expect("list")[0]
            .result
            .as_deref(),
        Some("Contract approved")
    );
}

#[test]
fn rejects_unknown_assignees() {
    let harness = TestHarness::new();
    let mut input = harness.delegate();
    input.assigned_agent_id = Some(uuid::Uuid::new_v4());
    assert!(matches!(
        harness.tasks.create(input),
        Err(AppError::NotFound(_))
    ));
}
