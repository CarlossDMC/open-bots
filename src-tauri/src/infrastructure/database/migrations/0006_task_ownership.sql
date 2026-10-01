-- Who created a task (NULL means the user), what the assignee reported, and when it last changed.
ALTER TABLE tasks ADD COLUMN created_by_agent_id TEXT REFERENCES agents(id) ON DELETE SET NULL;
ALTER TABLE tasks ADD COLUMN result TEXT;
ALTER TABLE tasks ADD COLUMN updated_at TEXT NOT NULL DEFAULT '';
UPDATE tasks SET updated_at = created_at WHERE updated_at = '';
CREATE INDEX IF NOT EXISTS idx_tasks_assigned_agent_id ON tasks(assigned_agent_id, created_at);

PRAGMA user_version = 6;
