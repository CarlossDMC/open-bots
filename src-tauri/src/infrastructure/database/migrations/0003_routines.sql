CREATE TABLE IF NOT EXISTS routines (
  id TEXT PRIMARY KEY,
  agent_id TEXT NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
  name TEXT NOT NULL,
  instructions TEXT NOT NULL,
  schedule_json TEXT NOT NULL,
  enabled INTEGER NOT NULL,
  -- Unix milliseconds so due-time comparisons are numeric.
  next_run_at_ms INTEGER NOT NULL,
  last_run_at TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_routines_agent_id ON routines(agent_id, created_at);
CREATE INDEX IF NOT EXISTS idx_routines_due ON routines(enabled, next_run_at_ms);

PRAGMA user_version = 3;
