pub mod application;
mod commands;
pub mod domain;
pub mod error;
pub mod infrastructure;
pub mod providers;
pub mod runtime;
pub mod tools;

use std::sync::Arc;

use application::{
    run_routine_scheduler, ActivityService, AgentService, ApprovalService, MemoryService,
    RoutineService,
};
use infrastructure::database::{
    Database, SqliteAgentRepository, SqliteApprovalRepository, SqliteEventRepository,
    SqliteMemoryRepository, SqliteRoutineRepository,
};
use providers::{MockProvider, ProviderRegistry};
use runtime::event_bus::EventBus;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::broadcast::error::RecvError;
use tracing_subscriber::EnvFilter;

/// Frontend event channel that mirrors every runtime event published on the in-process bus.
const RUNTIME_EVENT_CHANNEL: &str = "runtime-event";

pub struct AppState {
    agents: AgentService,
    approvals: ApprovalService,
    memories: MemoryService,
    activity: ActivityService,
    routines: Arc<RoutineService>,
    providers: Arc<ProviderRegistry>,
}

pub fn run() {
    initialize_logging();
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_notification::init());
    #[cfg(desktop)]
    let builder = builder.plugin(tauri_plugin_updater::Builder::new().build());
    builder
        .setup(|app| {
            let data_directory = app
                .path()
                .app_data_dir()
                .map_err(|error| format!("application data directory is unavailable: {error}"))?;
            let database = Arc::new(Database::open(&data_directory.join("open-bots.sqlite3"))?);
            let agent_repository = Arc::new(SqliteAgentRepository::new(Arc::clone(&database)));
            let approval_repository =
                Arc::new(SqliteApprovalRepository::new(Arc::clone(&database)));
            let memory_repository = Arc::new(SqliteMemoryRepository::new(Arc::clone(&database)));
            let routine_repository = Arc::new(SqliteRoutineRepository::new(Arc::clone(&database)));
            let event_repository = Arc::new(SqliteEventRepository::new(database));
            let mut registry = ProviderRegistry::new();
            registry.register(Arc::new(MockProvider))?;
            let providers = Arc::new(registry);
            let event_bus = EventBus::new(128);
            forward_runtime_events(app.handle().clone(), &event_bus);
            let agents = AgentService::new(
                agent_repository.clone(),
                event_repository.clone(),
                event_bus.clone(),
                Arc::clone(&providers),
            );
            let approvals = ApprovalService::new(
                approval_repository,
                agent_repository.clone(),
                event_repository.clone(),
                event_bus.clone(),
            );
            let memories = MemoryService::new(
                memory_repository,
                agent_repository.clone(),
                event_repository.clone(),
                event_bus.clone(),
            );
            let routines = Arc::new(RoutineService::new(
                routine_repository,
                agent_repository,
                event_repository.clone(),
                event_bus,
            ));
            tauri::async_runtime::spawn(run_routine_scheduler(Arc::clone(&routines)));
            let activity = ActivityService::new(event_repository);
            app.manage(AppState {
                agents,
                approvals,
                memories,
                activity,
                routines,
                providers,
            });
            tracing::info!(storage = %data_directory.display(), "local runtime initialized");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_agents,
            commands::create_agent,
            commands::list_providers,
            commands::list_events,
            commands::list_approvals,
            commands::resolve_approval,
            commands::list_memories,
            commands::add_memory,
            commands::remove_memory,
            commands::list_routines,
            commands::create_routine,
            commands::set_routine_enabled,
            commands::delete_routine
        ])
        .run(tauri::generate_context!())
        .expect("Tauri application failed to start");
}

fn forward_runtime_events(app: AppHandle, event_bus: &EventBus) {
    let mut receiver = event_bus.subscribe();
    tauri::async_runtime::spawn(async move {
        loop {
            match receiver.recv().await {
                Ok(event) => {
                    if let Err(error) = app.emit(RUNTIME_EVENT_CHANNEL, &event) {
                        tracing::warn!(%error, event_id = %event.id, "runtime event was not forwarded");
                    }
                }
                Err(RecvError::Lagged(skipped)) => {
                    tracing::warn!(skipped, "runtime event forwarder lagged behind the bus");
                }
                Err(RecvError::Closed) => break,
            }
        }
    });
}

fn initialize_logging() {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("open_bots=info,warn"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .json()
        .try_init();
}
