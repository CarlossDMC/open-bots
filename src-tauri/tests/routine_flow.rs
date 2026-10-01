use std::sync::Arc;

use chrono::{Duration, Utc};
use open_bots_lib::{
    application::{ActivityService, RoutineService},
    domain::{
        agents::{Agent, IdentityColor, NewAgent},
        events::EventType,
        routines::{NewRoutine, RoutineSchedule, MAX_ROUTINES_PER_AGENT},
    },
    error::AppError,
    infrastructure::database::{
        AgentRepository, Database, SqliteAgentRepository, SqliteEventRepository,
        SqliteRoutineRepository,
    },
    runtime::event_bus::EventBus,
};
use tempfile::TempDir;
use uuid::Uuid;

struct TestHarness {
    _directory: TempDir,
    agent: Agent,
    routines: RoutineService,
    activity: ActivityService,
    event_bus: EventBus,
}

impl TestHarness {
    fn new() -> Self {
        let directory = tempfile::tempdir().expect("temporary directory");
        let database = Arc::new(
            Database::open(&directory.path().join("routines.sqlite3")).expect("test database"),
        );
        let agents = Arc::new(SqliteAgentRepository::new(Arc::clone(&database)));
        let routine_repository = Arc::new(SqliteRoutineRepository::new(Arc::clone(&database)));
        let events = Arc::new(SqliteEventRepository::new(database));
        let event_bus = EventBus::new(256);
        let agent = Agent::create(NewAgent {
            name: "Orbit".into(),
            role: "Release Manager".into(),
            description: String::new(),
            provider_id: "mock".into(),
            identity_color: IdentityColor::Violet,
            workspace: "/tmp/open-bots-workspace".into(),
            instructions: String::new(),
            model_selection: Default::default(),
        })
        .expect("agent");
        agents.save(&agent).expect("save agent");
        Self {
            _directory: directory,
            agent,
            routines: RoutineService::new(
                routine_repository,
                agents,
                events.clone(),
                event_bus.clone(),
            ),
            activity: ActivityService::new(events),
            event_bus,
        }
    }

    fn input(&self, minutes: u32) -> NewRoutine {
        NewRoutine {
            agent_id: self.agent.id,
            name: "Release check".into(),
            instructions: "Review the release queue and report blockers.".into(),
            schedule: RoutineSchedule::Interval { minutes },
        }
    }
}

#[test]
fn fires_due_routines_once_and_reschedules_them() {
    let harness = TestHarness::new();
    let routine = harness.routines.create(harness.input(30)).expect("routine");
    let mut subscriber = harness.event_bus.subscribe();

    assert!(harness
        .routines
        .fire_due(routine.next_run_at - Duration::seconds(1))
        .expect("not yet due")
        .is_empty());

    let fired_at = routine.next_run_at + Duration::hours(3);
    let fired = harness.routines.fire_due(fired_at).expect("fire");

    assert_eq!(fired.len(), 1);
    assert_eq!(fired[0].last_run_at, Some(fired_at));
    assert_eq!(fired[0].next_run_at, fired_at + Duration::minutes(30));
    assert_eq!(
        harness.routines.next_due_at().expect("next"),
        Some(fired[0].next_run_at)
    );
    let event = subscriber.try_recv().expect("triggered event");
    assert_eq!(event.event_type, EventType::RoutineTriggered);
    assert_eq!(event.payload["agentId"], harness.agent.id.to_string());
    assert_eq!(event.payload["name"], "Release check");
    assert!(event.payload.get("instructions").is_none());
    assert!(harness
        .routines
        .fire_due(fired_at)
        .expect("already fired")
        .is_empty());
}

#[test]
fn paused_routines_do_not_fire() {
    let harness = TestHarness::new();
    let routine = harness.routines.create(harness.input(5)).expect("routine");

    let paused = harness
        .routines
        .set_enabled(routine.id, false)
        .expect("pause");

    assert!(!paused.enabled);
    assert_eq!(harness.routines.next_due_at().expect("next"), None);
    assert!(harness
        .routines
        .fire_due(Utc::now() + Duration::days(1))
        .expect("fire")
        .is_empty());
}

#[test]
fn records_lifecycle_events_and_deletes_routines() {
    let harness = TestHarness::new();
    let routine = harness.routines.create(harness.input(15)).expect("routine");
    harness
        .routines
        .set_enabled(routine.id, false)
        .expect("pause");
    harness.routines.delete(routine.id).expect("delete");

    assert!(harness
        .routines
        .list(harness.agent.id)
        .expect("list")
        .is_empty());
    let types: Vec<_> = harness
        .activity
        .recent(10)
        .expect("activity")
        .into_iter()
        .map(|event| event.event_type)
        .collect();
    for expected in [
        EventType::RoutineCreated,
        EventType::RoutineUpdated,
        EventType::RoutineDeleted,
    ] {
        assert!(types.contains(&expected), "missing {expected:?}");
    }
    assert!(matches!(
        harness.routines.delete(routine.id),
        Err(AppError::NotFound(_))
    ));
}

#[test]
fn enforces_the_per_agent_routine_limit() {
    let harness = TestHarness::new();
    for _ in 0..MAX_ROUTINES_PER_AGENT {
        harness.routines.create(harness.input(60)).expect("routine");
    }

    let result = harness.routines.create(harness.input(60));

    assert!(matches!(result, Err(AppError::Domain(_))));
}

#[test]
fn rejects_routines_for_unknown_agents() {
    let harness = TestHarness::new();
    let mut input = harness.input(30);
    input.agent_id = Uuid::new_v4();

    assert!(matches!(
        harness.routines.create(input),
        Err(AppError::NotFound(_))
    ));
}

#[tokio::test]
async fn notifies_the_scheduler_when_routines_change() {
    let harness = TestHarness::new();
    let changes = harness.routines.changes();

    harness.routines.create(harness.input(30)).expect("routine");

    tokio::time::timeout(std::time::Duration::from_secs(1), changes.notified())
        .await
        .expect("change notification");
}
