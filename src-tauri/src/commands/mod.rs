use tauri::State;

use crate::{
    domain::agents::{Agent, NewAgent},
    error::AppResult,
    providers::ProviderSummary,
    AppState,
};

#[tauri::command]
pub fn list_agents(state: State<'_, AppState>) -> AppResult<Vec<Agent>> {
    state.agents.list()
}

#[tauri::command]
pub fn create_agent(input: NewAgent, state: State<'_, AppState>) -> AppResult<Agent> {
    state.agents.create(input)
}

#[tauri::command]
pub async fn list_providers(state: State<'_, AppState>) -> AppResult<Vec<ProviderSummary>> {
    Ok(state.providers.detect_all().await)
}
