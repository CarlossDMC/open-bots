use std::sync::Arc;

use serde_json::json;
use uuid::Uuid;

use crate::{
    domain::{
        events::{DomainEvent, EventType},
        tasks::{NewTask, Task, TaskStatus},
    },
    error::{AppError, AppResult},
    infrastructure::database::{AgentRepository, EventRepository, TaskRepository},
    runtime::event_bus::EventBus,
};

use super::routine_service::now;

/// Who is changing a task. Agents may only change tasks they are assigned to or created.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskActor {
    User,
    Agent(Uuid),
}

pub struct TaskService {
    tasks: Arc<dyn TaskRepository>,
    agents: Arc<dyn AgentRepository>,
    events: Arc<dyn EventRepository>,
    event_bus: EventBus,
}

impl TaskService {
    pub fn new(
        tasks: Arc<dyn TaskRepository>,
        agents: Arc<dyn AgentRepository>,
        events: Arc<dyn EventRepository>,
        event_bus: EventBus,
    ) -> Self {
        Self {
            tasks,
            agents,
            events,
            event_bus,
        }
    }

    pub fn list(&self) -> AppResult<Vec<Task>> {
        self.tasks.list()
    }

    pub fn list_for_agent(&self, agent_id: Uuid) -> AppResult<Vec<Task>> {
        self.tasks.list_for_agent(agent_id)
    }

    pub fn find(&self, id: Uuid) -> AppResult<Task> {
        self.tasks
            .find(id)?
            .ok_or_else(|| AppError::NotFound(format!("task {id}")))
    }

    /// Creates a task. The workspace defaults to the assignee's, then the creator's.
    pub fn create(&self, mut input: NewTask) -> AppResult<Task> {
        let assignee = input
            .assigned_agent_id
            .map(|id| self.require_agent(id))
            .transpose()?;
        let creator = input
            .created_by_agent_id
            .map(|id| self.require_agent(id))
            .transpose()?;
        if let Some(parent_id) = input.parent_task_id {
            self.find(parent_id)?;
        }
        if input.workspace_id.trim().is_empty() {
            input.workspace_id = assignee
                .as_ref()
                .or(creator.as_ref())
                .map(|agent| agent.workspace.clone())
                .unwrap_or_default();
        }
        let task = Task::create(input, now())?;
        self.tasks.save(&task)?;
        self.publish(EventType::TaskCreated, &task)?;
        if task.assigned_agent_id.is_some() {
            self.publish(EventType::TaskAssigned, &task)?;
        }
        tracing::info!(task_id = %task.id, "task created");
        Ok(task)
    }

    pub fn assign(&self, id: Uuid, agent_id: Uuid, actor: TaskActor) -> AppResult<Task> {
        let mut task = self.find(id)?;
        self.authorize(&task, actor)?;
        self.require_agent(agent_id)?;
        task.assign(agent_id, now())?;
        self.tasks.save(&task)?;
        self.publish(EventType::TaskAssigned, &task)?;
        Ok(task)
    }

    pub fn update_status(
        &self,
        id: Uuid,
        status: TaskStatus,
        result: Option<String>,
        actor: TaskActor,
    ) -> AppResult<Task> {
        let mut task = self.find(id)?;
        self.authorize(&task, actor)?;
        task.update_status(status, result, now())?;
        self.tasks.save(&task)?;
        let event_type = match status {
            TaskStatus::Running => EventType::TaskStarted,
            TaskStatus::Completed => EventType::TaskCompleted,
            TaskStatus::Failed => EventType::TaskFailed,
            TaskStatus::Cancelled => EventType::TaskCancelled,
            _ => EventType::TaskUpdated,
        };
        self.publish(event_type, &task)?;
        Ok(task)
    }

    fn authorize(&self, task: &Task, actor: TaskActor) -> AppResult<()> {
        match actor {
            TaskActor::User => Ok(()),
            TaskActor::Agent(agent_id)
                if task.assigned_agent_id == Some(agent_id)
                    || task.created_by_agent_id == Some(agent_id) =>
            {
                Ok(())
            }
            TaskActor::Agent(_) => Err(AppError::Validation(
                "only the assignee or the creator can change this task".into(),
            )),
        }
    }

    fn require_agent(&self, id: Uuid) -> AppResult<crate::domain::agents::Agent> {
        self.agents
            .find(id)?
            .ok_or_else(|| AppError::NotFound(format!("agent {id}")))
    }

    fn publish(&self, event_type: EventType, task: &Task) -> AppResult<()> {
        // Descriptions and results stay out of the payload; the timeline needs identity only.
        let event = DomainEvent::new(
            event_type,
            Some(task.id),
            json!({
                "title": task.title,
                "status": task.status,
                "agentId": task.assigned_agent_id,
                "createdByAgentId": task.created_by_agent_id,
            }),
        );
        self.events.append(&event)?;
        self.event_bus.publish(event);
        Ok(())
    }
}
