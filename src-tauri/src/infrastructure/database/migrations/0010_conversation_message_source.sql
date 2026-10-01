-- Identifies the peer agent behind an incoming agent message so the conversation can
-- present it separately from runtime notices. Older messages remain valid without a source.
ALTER TABLE conversation_messages ADD COLUMN source_agent_id TEXT
  REFERENCES agents(id) ON DELETE SET NULL;

PRAGMA user_version = 10;
