-- MCP servers from each provider's own configuration that the user made available to that
-- provider's agents. Only names are stored; server configuration and credentials stay with
-- the provider.
CREATE TABLE mcp_catalog (
  provider_id TEXT NOT NULL,
  name TEXT NOT NULL,
  added_at TEXT NOT NULL,
  PRIMARY KEY (provider_id, name)
);

PRAGMA user_version = 11;
