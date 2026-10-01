use tauri::State;
use uuid::Uuid;

use crate::{
    application::{ProviderUsageReport, RuntimeSettings, TaskActor},
    domain::{
        agents::{Agent, NewAgent},
        approvals::{ApprovalRequest, ApprovalStatus},
        conversations::ConversationMessage,
        events::DomainEvent,
        groups::{Group, GroupMessage, NewGroup},
        mcp_servers::McpCatalogEntry,
        memories::AgentMemory,
        routines::{NewRoutine, Routine},
        tasks::{NewTask, Task, TaskStatus},
    },
    error::AppResult,
    providers::{ConfiguredMcpServer, ProviderModel, ProviderSummary},
    AppState,
};

#[tauri::command]
pub fn list_agents(state: State<'_, AppState>) -> AppResult<Vec<Agent>> {
    state.agents.list()
}

#[tauri::command]
pub fn create_agent(
    input: NewAgent,
    mcp_servers: Option<Vec<String>>,
    state: State<'_, AppState>,
) -> AppResult<Agent> {
    state
        .agents
        .create_with_mcp_servers(input, mcp_servers.unwrap_or_default())
}

#[tauri::command]
pub fn update_agent_model(
    agent_id: Uuid,
    model: Option<String>,
    reasoning_effort: Option<String>,
    state: State<'_, AppState>,
) -> AppResult<Agent> {
    state.agents.update_model(agent_id, model, reasoning_effort)
}

#[tauri::command]
pub fn update_agent_access(
    agent_id: Uuid,
    workspace_write: bool,
    network: bool,
    state: State<'_, AppState>,
) -> AppResult<Agent> {
    state
        .agents
        .update_access(agent_id, workspace_write, network)
}

#[tauri::command]
pub fn update_agent_mcp_servers(
    agent_id: Uuid,
    servers: Vec<String>,
    state: State<'_, AppState>,
) -> AppResult<Agent> {
    state.agents.update_mcp_servers(agent_id, servers)
}

#[tauri::command]
pub fn list_mcp_catalog(state: State<'_, AppState>) -> AppResult<Vec<McpCatalogEntry>> {
    state.mcp_catalog.list()
}

#[tauri::command]
pub async fn discover_mcp_servers(
    provider_id: String,
    state: State<'_, AppState>,
) -> AppResult<Vec<ConfiguredMcpServer>> {
    state.mcp_catalog.discover(&provider_id).await
}

#[tauri::command]
pub fn save_mcp_catalog(
    provider_id: String,
    servers: Vec<String>,
    state: State<'_, AppState>,
) -> AppResult<Vec<McpCatalogEntry>> {
    state.mcp_catalog.save(&provider_id, servers)
}

#[tauri::command]
pub async fn list_provider_models(
    provider_id: String,
    state: State<'_, AppState>,
) -> AppResult<Vec<ProviderModel>> {
    state.provider_catalog.list_models(&provider_id).await
}

#[tauri::command]
pub async fn read_provider_usage(
    state: State<'_, AppState>,
) -> AppResult<Vec<ProviderUsageReport>> {
    Ok(state.provider_catalog.read_all_usage().await)
}

#[tauri::command]
pub async fn list_providers(state: State<'_, AppState>) -> AppResult<Vec<ProviderSummary>> {
    Ok(state.providers.detect_all().await)
}

#[tauri::command]
pub fn list_events(limit: usize, state: State<'_, AppState>) -> AppResult<Vec<DomainEvent>> {
    state.activity.recent(limit)
}

#[tauri::command]
pub fn list_approvals(state: State<'_, AppState>) -> AppResult<Vec<ApprovalRequest>> {
    state.approvals.list()
}

#[tauri::command]
pub fn resolve_approval(
    id: Uuid,
    decision: ApprovalStatus,
    state: State<'_, AppState>,
) -> AppResult<ApprovalRequest> {
    state.approvals.resolve(id, decision)
}

#[tauri::command]
pub fn list_memories(agent_id: Uuid, state: State<'_, AppState>) -> AppResult<Vec<AgentMemory>> {
    state.memories.list(agent_id)
}

#[tauri::command]
pub fn add_memory(
    agent_id: Uuid,
    content: String,
    state: State<'_, AppState>,
) -> AppResult<AgentMemory> {
    state.memories.add(agent_id, &content)
}

#[tauri::command]
pub fn remove_memory(id: Uuid, state: State<'_, AppState>) -> AppResult<()> {
    state.memories.remove(id)
}

#[tauri::command]
pub fn list_routines(agent_id: Uuid, state: State<'_, AppState>) -> AppResult<Vec<Routine>> {
    state.routines.list(agent_id)
}

#[tauri::command]
pub fn create_routine(input: NewRoutine, state: State<'_, AppState>) -> AppResult<Routine> {
    state.routines.create(input)
}

#[tauri::command]
pub fn set_routine_enabled(
    id: Uuid,
    enabled: bool,
    state: State<'_, AppState>,
) -> AppResult<Routine> {
    state.routines.set_enabled(id, enabled)
}

#[tauri::command]
pub fn delete_routine(id: Uuid, state: State<'_, AppState>) -> AppResult<()> {
    state.routines.delete(id)
}

#[tauri::command]
pub fn list_tasks(state: State<'_, AppState>) -> AppResult<Vec<Task>> {
    state.tasks.list()
}

/// Tasks created from the desktop UI always belong to the user.
#[tauri::command]
pub fn create_task(mut input: NewTask, state: State<'_, AppState>) -> AppResult<Task> {
    input.created_by_agent_id = None;
    state.tasks.create(input)
}

#[tauri::command]
pub fn assign_task(id: Uuid, agent_id: Uuid, state: State<'_, AppState>) -> AppResult<Task> {
    state.tasks.assign(id, agent_id, TaskActor::User)
}

#[tauri::command]
pub fn update_task_status(
    id: Uuid,
    status: TaskStatus,
    result: Option<String>,
    state: State<'_, AppState>,
) -> AppResult<Task> {
    state
        .tasks
        .update_status(id, status, result, TaskActor::User)
}

#[tauri::command]
pub fn get_runtime_settings(state: State<'_, AppState>) -> AppResult<RuntimeSettings> {
    state.settings.runtime()
}

#[tauri::command]
pub fn update_runtime_settings(
    input: RuntimeSettings,
    state: State<'_, AppState>,
) -> AppResult<RuntimeSettings> {
    state.settings.update_runtime(input)
}

#[tauri::command]
pub fn list_messages(
    agent_id: Uuid,
    state: State<'_, AppState>,
) -> AppResult<Vec<ConversationMessage>> {
    state.conversations.list(agent_id)
}

/// Async so the background turn is spawned on the Tauri Tokio runtime.
#[tauri::command]
pub async fn send_message(
    agent_id: Uuid,
    content: String,
    state: State<'_, AppState>,
) -> AppResult<ConversationMessage> {
    state.conversations.send(agent_id, &content)
}

#[tauri::command]
pub fn reset_agent_session(agent_id: Uuid, state: State<'_, AppState>) -> AppResult<()> {
    state.conversations.reset_session(agent_id)
}

#[tauri::command]
pub fn cancel_turn(agent_id: Uuid, state: State<'_, AppState>) -> AppResult<()> {
    state.conversations.cancel(agent_id)
}

#[tauri::command]
pub fn list_groups(state: State<'_, AppState>) -> AppResult<Vec<Group>> {
    state.groups.list()
}

#[tauri::command]
pub fn create_group(input: NewGroup, state: State<'_, AppState>) -> AppResult<Group> {
    state.groups.create(input)
}

#[tauri::command]
pub fn delete_group(group_id: Uuid, state: State<'_, AppState>) -> AppResult<()> {
    state.groups.delete(group_id)
}

#[tauri::command]
pub fn list_group_messages(
    group_id: Uuid,
    state: State<'_, AppState>,
) -> AppResult<Vec<GroupMessage>> {
    state.groups.messages(group_id)
}

#[tauri::command]
pub fn send_group_message(
    group_id: Uuid,
    content: String,
    state: State<'_, AppState>,
) -> AppResult<GroupMessage> {
    state.groups.post_user_message(group_id, &content)
}

#[tauri::command]
pub fn stop_group(group_id: Uuid, state: State<'_, AppState>) -> AppResult<()> {
    state.groups.stop(group_id)
}
