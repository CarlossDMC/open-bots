CREATE TABLE IF NOT EXISTS conversation_messages (
  id TEXT PRIMARY KEY,
  agent_id TEXT NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
  role TEXT NOT NULL,
  content TEXT NOT NULL,
  -- Unix milliseconds plus insertion order keep same-instant messages stable.
  created_at_ms INTEGER NOT NULL,
  sequence INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_conversation_messages_agent
  ON conversation_messages(agent_id, created_at_ms, sequence);

-- The provider-side session an agent resumes, one per agent and provider.
CREATE TABLE IF NOT EXISTS provider_sessions (
  agent_id TEXT NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
  provider_id TEXT NOT NULL,
  session_id TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  PRIMARY KEY (agent_id, provider_id)
);

PRAGMA user_version = 4;
