pub mod application;
mod commands;
pub mod domain;
pub mod error;
pub mod infrastructure;
pub mod providers;
pub mod runtime;
pub mod tools;

use std::sync::Arc;

use application::AgentService;
use infrastructure::database::{Database, SqliteAgentRepository, SqliteEventRepository};
use providers::{MockProvider, ProviderRegistry};
use runtime::event_bus::EventBus;
use tauri::Manager;
use tracing_subscriber::EnvFilter;

pub struct AppState {
    agents: AgentService,
    providers: Arc<ProviderRegistry>,
}

pub fn run() {
    initialize_logging();
    let builder = tauri::Builder::default().plugin(tauri_plugin_process::init());
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
            let event_repository = Arc::new(SqliteEventRepository::new(database));
            let mut registry = ProviderRegistry::new();
            registry.register(Arc::new(MockProvider))?;
            let providers = Arc::new(registry);
            let agents = AgentService::new(
                agent_repository,
                event_repository,
                EventBus::new(128),
                Arc::clone(&providers),
            );
            app.manage(AppState { agents, providers });
            tracing::info!(storage = %data_directory.display(), "local runtime initialized");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_agents,
            commands::create_agent,
            commands::list_providers
        ])
        .run(tauri::generate_context!())
        .expect("Tauri application failed to start");
}

fn initialize_logging() {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("open_bots=info,warn"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .json()
        .try_init();
}
