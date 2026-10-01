//! Tools that let an agent act on the Open Bots runtime: tasks, memories, messages to other
//! agents, and approval requests.
//! They are served to providers over the local MCP server and act for the calling agent.

mod agents;
mod collaboration;
mod memory;
mod tasks;

use std::sync::Arc;

use uuid::Uuid;

use crate::{
    application::{ApprovalService, MemoryService, MessagingService, TaskService},
    domain::agents::Agent,
    error::{AppError, AppResult},
    infrastructure::database::AgentRepository,
};

pub use agents::AgentListTool;
pub use collaboration::{AgentMessageTool, ApprovalRequestTool};
pub use memory::MemorySaveTool;
pub use tasks::{TaskCreateTool, TaskListTool, TaskUpdateTool};

use super::{Tool, ToolRegistry};

/// Services the runtime tools delegate to.
#[derive(Clone)]
pub struct RuntimeToolServices {
    pub agents: Arc<dyn AgentRepository>,
    pub memories: Arc<MemoryService>,
    pub tasks: Arc<TaskService>,
    pub messaging: Arc<MessagingService>,
    pub approvals: Arc<ApprovalService>,
}

/// Registers every runtime tool.
pub fn register_runtime_tools(
    registry: &mut ToolRegistry,
    services: &RuntimeToolServices,
) -> AppResult<()> {
    let tools: Vec<Arc<dyn Tool>> = vec![
        Arc::new(MemorySaveTool::new(services.memories.clone())),
        Arc::new(TaskCreateTool::new(services.clone())),
        Arc::new(TaskUpdateTool::new(services.tasks.clone())),
        Arc::new(TaskListTool::new(services.clone())),
        Arc::new(AgentListTool::new(services.agents.clone())),
        Arc::new(AgentMessageTool::new(
            services.agents.clone(),
            services.messaging.clone(),
        )),
        Arc::new(ApprovalRequestTool::new(services.approvals.clone())),
    ];
    for tool in tools {
        registry.register(tool)?;
    }
    Ok(())
}

/// Finds an agent by id or by case-insensitive name.
pub(crate) fn resolve_agent(agents: &dyn AgentRepository, reference: &str) -> AppResult<Agent> {
    let reference = reference.trim();
    if let Ok(id) = reference.parse::<Uuid>() {
        return agents
            .find(id)?
            .ok_or_else(|| AppError::NotFound(format!("agent {id}")));
    }
    let mut matches = agents
        .list()?
        .into_iter()
        .filter(|agent| agent.name.eq_ignore_ascii_case(reference));
    match (matches.next(), matches.next()) {
        (Some(agent), None) => Ok(agent),
        (Some(_), Some(_)) => Err(AppError::Validation(format!(
            "several agents are named \"{reference}\"; use the agent id from agent_list"
        ))),
        (None, _) => Err(AppError::NotFound(format!(
            "no agent named \"{reference}\"; call agent_list to see the team"
        ))),
    }
}
