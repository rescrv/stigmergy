-- This migration creates the configs table for versioned configuration storage.
-- Configurations are stored with an auto-incrementing version number,
-- and the system always loads the configuration with the maximum version on startup.

-- The `configs` table stores versioned configurations.
CREATE TABLE configs (
    -- Auto-incrementing version number, serves as the primary key.
    version BIGSERIAL PRIMARY KEY,
    -- The configuration data stored as JSON.
    config_json JSONB NOT NULL,
    -- The timestamp when this configuration version was created.
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Index to optimize queries that retrieve the latest configuration
CREATE INDEX idx_configs_version_desc ON configs(version DESC);
