use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

use super::{resolve_agent, RuntimeToolServices};
use crate::{
    application::{TaskActor, TaskService},
    domain::{
        agents::PermissionLevel,
        tasks::{
            NewTask, Task, TaskStatus, MAX_TASK_DESCRIPTION_LENGTH, MAX_TASK_RESULT_LENGTH,
            MAX_TASK_TITLE_LENGTH,
        },
    },
    error::AppResult,
    tools::{parse_input, Tool, ToolRequest, ToolResult},
};

fn task_summary(task: &Task) -> Value {
    json!({
        "id": task.id,
        "title": task.title,
        "description": task.description,
        "status": task.status,
        "assignedAgentId": task.assigned_agent_id,
        "createdByAgentId": task.created_by_agent_id,
        "parentTaskId": task.parent_task_id,
        "result": task.result,
    })
}

pub struct TaskCreateTool {
    services: RuntimeToolServices,
}

impl TaskCreateTool {
    pub fn new(services: RuntimeToolServices) -> Self {
        Self { services }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct CreateInput {
    title: String,
    #[serde(default)]
    description: String,
    assignee: Option<String>,
    parent_task_id: Option<Uuid>,
}

#[async_trait]
impl Tool for TaskCreateTool {
    fn id(&self) -> &'static str {
        "task_create"
    }
    fn description(&self) -> &'static str {
        "Create a task. Set assignee to another agent's name or id to delegate it: that agent \
         is woken to work on it, and you are woken with the result when it finishes. Omit \
         assignee to keep the task yourself."
    }
    fn input_contract(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "title": { "type": "string", "maxLength": MAX_TASK_TITLE_LENGTH },
                "description": { "type": "string", "maxLength": MAX_TASK_DESCRIPTION_LENGTH, "description": "What done looks like, with any context the assignee needs." },
                "assignee": { "type": "string", "description": "Agent name or id. Defaults to you." },
                "parentTaskId": { "type": "string", "description": "Task this one breaks down." }
            },
            "required": ["title"],
            "additionalProperties": false
        })
    }
    fn required_permission(&self) -> PermissionLevel {
        PermissionLevel::Allowed
    }
    fn action_id(&self) -> &'static str {
        "task.write"
    }
    async fn execute(&self, request: ToolRequest) -> AppResult<ToolResult> {
        let input: CreateInput = parse_input(request.input)?;
        let caller = request.context.agent_id;
        let assignee = match input.assignee.as_deref() {
            Some(reference) if !reference.trim().is_empty() => {
                resolve_agent(self.services.agents.as_ref(), reference)?.id
            }
            _ => caller,
        };
        let task = self.services.tasks.create(NewTask {
            title: input.title,
            description: input.description,
            assigned_agent_id: Some(assignee),
            created_by_agent_id: Some(caller),
            parent_task_id: input.parent_task_id,
            workspace_id: String::new(),
        })?;
        let summary = if assignee == caller {
            format!("Created task {}.", task.id)
        } else {
            format!(
                "Created and delegated task {}; you will be woken when it finishes.",
                task.id
            )
        };
        Ok(ToolResult {
            output: task_summary(&task),
            summary,
        })
    }
}

pub struct TaskUpdateTool {
    tasks: Arc<TaskService>,
}

impl TaskUpdateTool {
    pub fn new(tasks: Arc<TaskService>) -> Self {
        Self { tasks }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct UpdateInput {
    task_id: Uuid,
    status: TaskStatus,
    result: Option<String>,
}

#[async_trait]
impl Tool for TaskUpdateTool {
    fn id(&self) -> &'static str {
        "task_update"
    }
    fn description(&self) -> &'static str {
        "Change the status of a task you are assigned to or created. Mark it running when you \
         start, blocked or waiting when you cannot continue, and completed or failed with a \
         short result when you finish. Finishing a delegated task wakes the agent that \
         created it."
    }
    fn input_contract(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "taskId": { "type": "string" },
                "status": { "type": "string", "enum": ["running", "waiting", "blocked", "completed", "failed", "cancelled"] },
                "result": { "type": "string", "maxLength": MAX_TASK_RESULT_LENGTH, "description": "Outcome, only when the task finishes." }
            },
            "required": ["taskId", "status"],
            "additionalProperties": false
        })
    }
    fn required_permission(&self) -> PermissionLevel {
        PermissionLevel::Allowed
    }
    fn action_id(&self) -> &'static str {
        "task.write"
    }
    async fn execute(&self, request: ToolRequest) -> AppResult<ToolResult> {
        let input: UpdateInput = parse_input(request.input)?;
        let task = self.tasks.update_status(
            input.task_id,
            input.status,
            input.result,
            TaskActor::Agent(request.context.agent_id),
        )?;
        Ok(ToolResult {
            output: task_summary(&task),
            summary: format!("Task {} is now {}.", task.id, status_word(task.status)),
        })
    }
}

pub struct TaskListTool {
    services: RuntimeToolServices,
}

impl TaskListTool {
    pub fn new(services: RuntimeToolServices) -> Self {
        Self { services }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct ListInput {
    #[serde(default)]
    include_finished: bool,
}

#[async_trait]
impl Tool for TaskListTool {
    fn id(&self) -> &'static str {
        "task_list"
    }
    fn description(&self) -> &'static str {
        "List tasks assigned to you or created by you, newest first. Finished tasks are \
         left out unless includeFinished is true."
    }
    fn input_contract(&self) -> Value {
        json!({
            "type": "object",
            "properties": { "includeFinished": { "type": "boolean" } },
            "additionalProperties": false
        })
    }
    fn required_permission(&self) -> PermissionLevel {
        PermissionLevel::Allowed
    }
    fn action_id(&self) -> &'static str {
        "task.read"
    }
    async fn execute(&self, request: ToolRequest) -> AppResult<ToolResult> {
        let input: ListInput = parse_input(request.input)?;
        let tasks: Vec<Value> = self
            .services
            .tasks
            .list_for_agent(request.context.agent_id)?
            .iter()
            .filter(|task| input.include_finished || !task.status.is_terminal())
            .map(task_summary)
            .collect();
        let summary = format!("{} task(s).", tasks.len());
        Ok(ToolResult {
            output: json!({ "tasks": tasks }),
            summary,
        })
    }
}

fn status_word(status: TaskStatus) -> String {
    serde_json::to_value(status)
        .ok()
        .and_then(|value| value.as_str().map(str::to_owned))
        .unwrap_or_default()
}
