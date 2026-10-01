use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{DomainError, DomainResult};

pub const MAX_TASK_TITLE_LENGTH: usize = 200;
pub const MAX_TASK_DESCRIPTION_LENGTH: usize = 4_000;
pub const MAX_TASK_RESULT_LENGTH: usize = 4_000;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub id: Uuid,
    pub title: String,
    pub description: String,
    pub status: TaskStatus,
    pub assigned_agent_id: Option<Uuid>,
    /// The agent that created the task; `None` means the user created it.
    pub created_by_agent_id: Option<Uuid>,
    pub parent_task_id: Option<Uuid>,
    pub workspace_id: String,
    /// What the assignee reported when the task completed or failed.
    pub result: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    Pending,
    Queued,
    Running,
    Waiting,
    Blocked,
    Completed,
    Failed,
    Cancelled,
}

impl TaskStatus {
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewTask {
    pub title: String,
    #[serde(default)]
    pub description: String,
    pub assigned_agent_id: Option<Uuid>,
    #[serde(default)]
    pub created_by_agent_id: Option<Uuid>,
    pub parent_task_id: Option<Uuid>,
    #[serde(default)]
    pub workspace_id: String,
}

impl Task {
    pub fn create(input: NewTask, now: DateTime<Utc>) -> DomainResult<Self> {
        let title = input.title.trim();
        if title.is_empty() || title.chars().count() > MAX_TASK_TITLE_LENGTH {
            return Err(DomainError::Validation(format!(
                "task title must contain 1 to {MAX_TASK_TITLE_LENGTH} characters"
            )));
        }
        let description = input.description.trim();
        if description.chars().count() > MAX_TASK_DESCRIPTION_LENGTH {
            return Err(DomainError::Validation(format!(
                "task description must not exceed {MAX_TASK_DESCRIPTION_LENGTH} characters"
            )));
        }
        let status = if input.assigned_agent_id.is_some() {
            TaskStatus::Queued
        } else {
            TaskStatus::Pending
        };
        Ok(Self {
            id: Uuid::new_v4(),
            title: title.into(),
            description: description.into(),
            status,
            assigned_agent_id: input.assigned_agent_id,
            created_by_agent_id: input.created_by_agent_id,
            parent_task_id: input.parent_task_id,
            workspace_id: input.workspace_id,
            result: None,
            created_at: now,
            updated_at: now,
            started_at: None,
            completed_at: None,
        })
    }

    /// Hands the task to an agent. A reassigned task waits in the new agent's queue.
    pub fn assign(&mut self, agent_id: Uuid, now: DateTime<Utc>) -> DomainResult<()> {
        self.ensure_open()?;
        self.assigned_agent_id = Some(agent_id);
        self.status = TaskStatus::Queued;
        self.updated_at = now;
        Ok(())
    }

    /// Moves an open task to `target`. Finished tasks keep their outcome, and only an
    /// assigned task can be queued or worked on.
    pub fn update_status(
        &mut self,
        target: TaskStatus,
        result: Option<String>,
        now: DateTime<Utc>,
    ) -> DomainResult<()> {
        self.ensure_open()?;
        if target == TaskStatus::Pending {
            return Err(DomainError::Validation(
                "a task cannot return to pending".into(),
            ));
        }
        if !target.is_terminal() && self.assigned_agent_id.is_none() {
            return Err(DomainError::Validation(
                "assign the task to an agent before working on it".into(),
            ));
        }
        let result = result
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty());
        if let Some(value) = &result {
            if value.chars().count() > MAX_TASK_RESULT_LENGTH {
                return Err(DomainError::Validation(format!(
                    "task result must not exceed {MAX_TASK_RESULT_LENGTH} characters"
                )));
            }
        }
        if target == TaskStatus::Running && self.started_at.is_none() {
            self.started_at = Some(now);
        }
        if target.is_terminal() {
            self.completed_at = Some(now);
            self.result = result;
        } else if result.is_some() {
            return Err(DomainError::Validation(
                "a result is only recorded when the task finishes".into(),
            ));
        }
        self.status = target;
        self.updated_at = now;
        Ok(())
    }

    fn ensure_open(&self) -> DomainResult<()> {
        if self.status.is_terminal() {
            return Err(DomainError::Validation(
                "the task has already finished".into(),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn new_task(assignee: Option<Uuid>) -> NewTask {
        NewTask {
            title: "  Write the release notes ".into(),
            description: String::new(),
            assigned_agent_id: assignee,
            created_by_agent_id: None,
            parent_task_id: None,
            workspace_id: "/workspace".into(),
        }
    }

    #[test]
    fn creation_trims_and_queues_assigned_tasks() {
        let now = Utc::now();
        let task = Task::create(new_task(Some(Uuid::new_v4())), now).expect("task");
        assert_eq!(task.title, "Write the release notes");
        assert_eq!(task.status, TaskStatus::Queued);
        let unassigned = Task::create(new_task(None), now).expect("task");
        assert_eq!(unassigned.status, TaskStatus::Pending);
    }

    #[test]
    fn creation_rejects_empty_and_oversized_text() {
        let now = Utc::now();
        let mut input = new_task(None);
        input.title = "   ".into();
        assert!(Task::create(input, now).is_err());
        let mut input = new_task(None);
        input.description = "x".repeat(MAX_TASK_DESCRIPTION_LENGTH + 1);
        assert!(Task::create(input, now).is_err());
    }

    #[test]
    fn work_requires_an_assignee() {
        let now = Utc::now();
        let mut task = Task::create(new_task(None), now).expect("task");
        assert!(task.update_status(TaskStatus::Running, None, now).is_err());
        task.assign(Uuid::new_v4(), now).expect("assign");
        task.update_status(TaskStatus::Running, None, now)
            .expect("start");
        assert_eq!(task.started_at, Some(now));
    }

    #[test]
    fn finishing_records_the_result_and_locks_the_task() {
        let now = Utc::now();
        let mut task = Task::create(new_task(Some(Uuid::new_v4())), now).expect("task");
        task.update_status(TaskStatus::Completed, Some(" Shipped ".into()), now)
            .expect("complete");
        assert_eq!(task.result.as_deref(), Some("Shipped"));
        assert_eq!(task.completed_at, Some(now));
        assert!(task.update_status(TaskStatus::Running, None, now).is_err());
        assert!(task.assign(Uuid::new_v4(), now).is_err());
    }

    #[test]
    fn unassigned_tasks_can_still_be_cancelled() {
        let now = Utc::now();
        let mut task = Task::create(new_task(None), now).expect("task");
        task.update_status(TaskStatus::Cancelled, None, now)
            .expect("cancel");
        assert_eq!(task.status, TaskStatus::Cancelled);
    }

    #[test]
    fn rejects_pending_targets_and_results_on_open_states() {
        let now = Utc::now();
        let mut task = Task::create(new_task(Some(Uuid::new_v4())), now).expect("task");
        assert!(task.update_status(TaskStatus::Pending, None, now).is_err());
        assert!(task
            .update_status(TaskStatus::Blocked, Some("why".into()), now)
            .is_err());
    }
}
