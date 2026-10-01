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
    run_agent_runtime, run_routine_scheduler, ActivityService, AgentRuntime, AgentService,
    ApprovalService, ConversationService, McpCatalogService, MemoryService, MessagingService,
    ProviderService, RoutineService, SettingsService, TaskService, ToolService,
};
use domain::approvals::DefaultApprovalPolicy;
use infrastructure::{
    database::{
        Database, SqliteAgentRepository, SqliteApprovalRepository, SqliteConversationRepository,
        SqliteEventRepository, SqliteMcpCatalogRepository, SqliteMemoryRepository,
        SqliteRoutineRepository, SqliteSettingsRepository, SqliteTaskRepository,
        SqliteWakeRepository,
    },
    mcp::McpListener,
    process::{TokioJsonRpcProcessClient, TokioLineProcessRunner},
};
use providers::{ClaudeProvider, CodexProvider, MockProvider, ProviderRegistry};
use runtime::{event_bus::EventBus, turn_tokens::TurnTokens};
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::broadcast::error::RecvError;
use tracing_subscriber::EnvFilter;

/// Frontend event channel that mirrors every runtime event published on the in-process bus.
const RUNTIME_EVENT_CHANNEL: &str = "runtime-event";

pub struct AppState {
    agents: AgentService,
    approvals: Arc<ApprovalService>,
    memories: Arc<MemoryService>,
    activity: ActivityService,
    routines: Arc<RoutineService>,
    tasks: Arc<TaskService>,
    settings: Arc<SettingsService>,
    conversations: Arc<ConversationService>,
    providers: Arc<ProviderRegistry>,
    provider_catalog: ProviderService,
    mcp_catalog: McpCatalogService,
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
            let task_repository = Arc::new(SqliteTaskRepository::new(Arc::clone(&database)));
            let wake_repository = Arc::new(SqliteWakeRepository::new(Arc::clone(&database)));
            let mcp_catalog_repository =
                Arc::new(SqliteMcpCatalogRepository::new(Arc::clone(&database)));
            let settings = Arc::new(SettingsService::new(Arc::new(
                SqliteSettingsRepository::new(Arc::clone(&database)),
            )));
            let conversation_repository =
                Arc::new(SqliteConversationRepository::new(Arc::clone(&database)));
            let event_repository = Arc::new(SqliteEventRepository::new(database));
            let mut registry = ProviderRegistry::new();
            registry.register(Arc::new(MockProvider))?;
            registry.register(Arc::new(CodexProvider::new(
                Arc::new(TokioLineProcessRunner),
                Arc::new(TokioJsonRpcProcessClient),
            )))?;
            registry.register(Arc::new(ClaudeProvider::new(Arc::new(
                TokioLineProcessRunner,
            ))))?;
            let providers = Arc::new(registry);
            let event_bus = EventBus::new(512);
            forward_runtime_events(app.handle().clone(), &event_bus);
            let agents = AgentService::new(
                agent_repository.clone(),
                event_repository.clone(),
                event_bus.clone(),
                Arc::clone(&providers),
            )
            .with_mcp_catalog(mcp_catalog_repository.clone());
            let approvals = Arc::new(ApprovalService::new(
                approval_repository.clone(),
                agent_repository.clone(),
                event_repository.clone(),
                event_bus.clone(),
            ));
            let turn_tokens = TurnTokens::new();
            let mcp_listener = McpListener::bind()?;
            let conversations = Arc::new(
                ConversationService::new(
                    agent_repository.clone(),
                    conversation_repository,
                    memory_repository.clone(),
                    Arc::clone(&providers),
                    event_repository.clone(),
                    event_bus.clone(),
                )
                .with_runtime_tools(mcp_listener.url(), Arc::clone(&turn_tokens))
                .with_mcp_catalog(mcp_catalog_repository.clone()),
            );
            if let Err(error) = conversations.recover_interrupted() {
                tracing::error!(%error, "interrupted turns could not be recovered");
            }
            let memories = Arc::new(MemoryService::new(
                memory_repository,
                agent_repository.clone(),
                event_repository.clone(),
                event_bus.clone(),
            ));
            let tasks = Arc::new(TaskService::new(
                task_repository.clone(),
                agent_repository.clone(),
                event_repository.clone(),
                event_bus.clone(),
            ));
            let routines = Arc::new(RoutineService::new(
                routine_repository.clone(),
                agent_repository.clone(),
                event_repository.clone(),
                event_bus.clone(),
            ));
            let mut tool_registry = tools::ToolRegistry::default();
            tools::runtime::register_runtime_tools(
                &mut tool_registry,
                &tools::runtime::RuntimeToolServices {
                    agents: agent_repository.clone(),
                    memories: Arc::clone(&memories),
                    tasks: Arc::clone(&tasks),
                    messaging: Arc::new(MessagingService::new(
                        agent_repository.clone(),
                        wake_repository.clone(),
                        event_repository.clone(),
                        event_bus.clone(),
                    )),
                    approvals: Arc::clone(&approvals),
                },
            )?;
            let tool_service = Arc::new(ToolService::new(
                tool_registry,
                Arc::new(DefaultApprovalPolicy),
            ));
            tauri::async_runtime::spawn(mcp_listener.serve(tool_service, turn_tokens));
            let agent_runtime = Arc::new(AgentRuntime::new(
                wake_repository,
                agent_repository,
                routine_repository,
                task_repository,
                approval_repository,
                Arc::clone(&settings),
                Arc::clone(&conversations),
                event_repository.clone(),
                event_bus.clone(),
            ));
            tauri::async_runtime::spawn(run_agent_runtime(agent_runtime, event_bus.subscribe()));
            tauri::async_runtime::spawn(run_routine_scheduler(Arc::clone(&routines)));
            let mcp_catalog = McpCatalogService::new(
                mcp_catalog_repository,
                event_repository.clone(),
                event_bus.clone(),
                Arc::clone(&providers),
            );
            let activity = ActivityService::new(event_repository);
            let provider_catalog = ProviderService::new(Arc::clone(&providers));
            app.manage(AppState {
                agents,
                approvals,
                memories,
                activity,
                routines,
                tasks,
                settings,
                conversations,
                providers,
                provider_catalog,
                mcp_catalog,
            });
            tracing::info!(storage = %data_directory.display(), "local runtime initialized");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_agents,
            commands::create_agent,
            commands::list_providers,
            commands::list_provider_models,
            commands::read_provider_usage,
            commands::update_agent_model,
            commands::update_agent_mcp_servers,
            commands::list_mcp_catalog,
            commands::discover_mcp_servers,
            commands::save_mcp_catalog,
            commands::list_events,
            commands::list_approvals,
            commands::resolve_approval,
            commands::list_memories,
            commands::add_memory,
            commands::remove_memory,
            commands::list_routines,
            commands::create_routine,
            commands::set_routine_enabled,
            commands::delete_routine,
            commands::list_tasks,
            commands::create_task,
            commands::assign_task,
            commands::update_task_status,
            commands::get_runtime_settings,
            commands::update_runtime_settings,
            commands::list_messages,
            commands::send_message,
            commands::cancel_turn
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
