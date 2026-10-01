-- MCP servers from the provider's own configuration that an agent may use, as a JSON array
-- of server names. An empty array keeps the agent to Open Bots runtime tools only.
ALTER TABLE agents ADD COLUMN mcp_servers_json TEXT NOT NULL DEFAULT '[]';

PRAGMA user_version = 9;
