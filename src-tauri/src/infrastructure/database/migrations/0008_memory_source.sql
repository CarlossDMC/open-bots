-- Who wrote each memory: 'user' from the agent's settings or 'agent' through memory_save.
ALTER TABLE agent_memories ADD COLUMN source TEXT NOT NULL DEFAULT 'user';

PRAGMA user_version = 8;
