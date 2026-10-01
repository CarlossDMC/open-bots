-- The provider model and reasoning effort an agent runs with. NULL keeps the provider default.
ALTER TABLE agents ADD COLUMN model TEXT;
ALTER TABLE agents ADD COLUMN reasoning_effort TEXT;

PRAGMA user_version = 5;
