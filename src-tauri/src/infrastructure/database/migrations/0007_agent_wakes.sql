-- Reasons to wake an agent without a user message. Rows queue while the agent is busy
-- and survive restarts; consumed_at marks the ones already handled.
CREATE TABLE IF NOT EXISTS agent_wakes (
  id TEXT PRIMARY KEY,
  agent_id TEXT NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
  origin_json TEXT NOT NULL,
  content TEXT NOT NULL,
  chain_depth INTEGER NOT NULL,
  -- Unix milliseconds plus insertion order keep same-instant wakes in arrival order.
  created_at_ms INTEGER NOT NULL,
  sequence INTEGER NOT NULL,
  consumed_at TEXT,
  outcome TEXT
);
CREATE INDEX IF NOT EXISTS idx_agent_wakes_pending
  ON agent_wakes(agent_id, consumed_at, created_at_ms, sequence);

PRAGMA user_version = 7;
