-- This migration adds the active_entity table to track which entities are currently
-- active in the system and optionally associated with specific systems.

-- The `active_entity` table tracks entities that are currently active in the system.
-- An active entity may be associated with a specific system or may be active independently.
CREATE TABLE active_entity (
    -- A foreign key referencing the `entity_id` in the `entities` table.
    -- If an entity is deleted, its active_entity record is also deleted.
    entity_id BYTEA PRIMARY KEY REFERENCES entities(entity_id) ON DELETE CASCADE,
    -- A nullable foreign key referencing the `system_name` in the `systems` table.
    -- If a system is deleted, the system_name is set to NULL.
    -- NULL indicates the entity is active but not associated with a specific system.
    system_name VARCHAR(255) REFERENCES systems(system_name) ON DELETE SET NULL,
    -- The timestamp when the entity became active.
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    -- The timestamp when the active_entity record was last updated.
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    -- Ensures that the entity_id is exactly 32 bytes long.
    CONSTRAINT entity_id_length CHECK (octet_length(entity_id) = 32)
);

-- This index improves the performance of queries that filter by `system_name`.
CREATE INDEX idx_active_entity_system_name ON active_entity(system_name);
