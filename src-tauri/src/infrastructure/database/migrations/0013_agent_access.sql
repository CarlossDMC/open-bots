-- Agents may now edit their workspace, run commands, and use the internet by default.
-- Existing agents are switched on to match; each can be turned off in the agent details.
UPDATE agents SET permissions_json = json_set(
  permissions_json,
  '$.filesystem', 'workspace-only',
  '$.shell', 'allowed',
  '$.network', 'allowed'
);

PRAGMA user_version = 13;
