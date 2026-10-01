-- Conversations between the user and several agents about one topic.
CREATE TABLE IF NOT EXISTS agent_groups (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  topic TEXT NOT NULL,
  created_by_json TEXT NOT NULL,
  -- Members still to answer, in order, and the wake queued for the current one.
  round_json TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS group_members (
  group_id TEXT NOT NULL REFERENCES agent_groups(id) ON DELETE CASCADE,
  agent_id TEXT NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
  position INTEGER NOT NULL,
  PRIMARY KEY (group_id, agent_id)
);
CREATE INDEX IF NOT EXISTS idx_group_members_agent ON group_members(agent_id);

CREATE TABLE IF NOT EXISTS group_messages (
  id TEXT PRIMARY KEY,
  group_id TEXT NOT NULL REFERENCES agent_groups(id) ON DELETE CASCADE,
  author_json TEXT NOT NULL,
  -- Set for agent-authored messages so they can be found without parsing author_json.
  author_agent_id TEXT,
  content TEXT NOT NULL,
  -- Unix milliseconds plus insertion order keep same-instant messages stable.
  created_at_ms INTEGER NOT NULL,
  sequence INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_group_messages_group
  ON group_messages(group_id, created_at_ms, sequence);

-- Provider sessions become per conversation: '' is the agent's direct conversation and a
-- group id is that group. Existing sessions belong to direct conversations.
CREATE TABLE provider_sessions_scoped (
  agent_id TEXT NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
  provider_id TEXT NOT NULL,
  conversation_id TEXT NOT NULL DEFAULT '',
  session_id TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  PRIMARY KEY (agent_id, provider_id, conversation_id)
);
INSERT INTO provider_sessions_scoped (agent_id, provider_id, conversation_id, session_id, updated_at)
  SELECT agent_id, provider_id, '', session_id, updated_at FROM provider_sessions;
DROP TABLE provider_sessions;
ALTER TABLE provider_sessions_scoped RENAME TO provider_sessions;

PRAGMA user_version = 12;
