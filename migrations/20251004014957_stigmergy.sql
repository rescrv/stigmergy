-- This migration initializes the database schema for the stigmergy project.
-- It creates the core tables for managing entities, components, and systems.

-- The `entities` table stores the unique identifiers for all entities in the system.
-- An entity is a fundamental concept in stigmergy, representing a distinct object
-- with which components can be associated.
CREATE TABLE entities (
    -- A unique 32-byte identifier for the entity.
    entity_id BYTEA PRIMARY KEY,
    -- The timestamp when the entity was created.
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    -- The timestamp when the entity was last updated.
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    -- Ensures that the entity_id is exactly 32 bytes long.
    CONSTRAINT entity_id_length CHECK (octet_length(entity_id) = 32)
);

-- The `component_definitions` table stores the schema for each type of component.
-- A component definition acts as a template, defining the structure and validation
-- rules for component instances.
CREATE TABLE component_definitions (
    -- The unique name of the component, used as a primary key.
    component_name VARCHAR(255) PRIMARY KEY NOT NULL,
    -- The JSON schema that defines the structure of the component's data.
    schema JSONB NOT NULL,
    -- The timestamp when the component definition was created.
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    -- The timestamp when the component definition was last updated.
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- The `component_instances` table stores the actual data for each component
-- associated with an entity. Each row represents a specific component attached
-- to a specific entity.
CREATE TABLE component_instances (
    -- A foreign key referencing the `entity_id` in the `entities` table.
    -- If an entity is deleted, all its component instances are also deleted.
    entity_id BYTEA NOT NULL REFERENCES entities(entity_id) ON DELETE CASCADE,
    -- A foreign key referencing the `component_name` in the `component_definitions` table.
    -- If a component definition is deleted, all its instances are also deleted.
    component_name VARCHAR(255) NOT NULL REFERENCES component_definitions(component_name) ON DELETE CASCADE,
    -- The actual data of the component instance, stored as a JSON object.
    -- This data must conform to the schema defined in `component_definitions`.
    -- NULL indicates a tombstone.
    data JSONB,
    -- The timestamp when the component instance was created.
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    -- The timestamp when the component instance was last updated.
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    -- The primary key is a composite of `entity_id` and `component_name`,
    -- ensuring that each entity can only have one instance of each component type.
    PRIMARY KEY (entity_id, component_name)
);

-- This index improves the performance of queries that filter by `component_name`.
CREATE INDEX idx_component_instances_component_name ON component_instances(component_name);

-- The `systems` table stores information about different systems in the stigmergy project.
-- A system is a collection of entities and components that work together to achieve a goal.
CREATE TABLE systems (
    -- The unique name of the system, used as a primary key.
    system_name VARCHAR(255) PRIMARY KEY NOT NULL,
    -- A detailed description of the system's purpose and functionality.
    description TEXT,
    -- The model used by the system, which can influence its behavior.
    model VARCHAR(255) NOT NULL,
    -- A color associated with the system, for display purposes.
    color VARCHAR(50),
    -- The main content or definition of the system.
    content TEXT,
    -- A list of bids associated with the system.
    bids TEXT[] NOT NULL DEFAULT '{}',
    -- The timestamp when the system was created.
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    -- The timestamp when the system was last updated.
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- The `invariants` table stores conditions that must always be true.
-- Invariants are used to enforce data integrity and system correctness.
CREATE TABLE invariants (
    -- A unique 32-byte identifier for the invariant.
    invariant_id BYTEA PRIMARY KEY,
    -- The assertion or condition that must be met.
    asserts TEXT NOT NULL,
    -- The timestamp when the invariant was created.
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    -- The timestamp when the invariant was last updated.
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    -- Ensures that the invariant_id is exactly 32 bytes long.
    CONSTRAINT invariant_id_length CHECK (octet_length(invariant_id) = 32)
);

-- The `messages` table stores messages associated with a specific component of an entity.
-- These messages can be used for communication, logging, or debugging purposes.
CREATE TABLE messages (
    -- A foreign key referencing the `entity_id` in the `entities` table.
    entity_id BYTEA NOT NULL REFERENCES entities(entity_id) ON DELETE CASCADE,
    -- A foreign key referencing the `component_name` in the `component_definitions` table.
    component_name VARCHAR(255) NOT NULL REFERENCES component_definitions(component_name) ON DELETE CASCADE,
    -- A serial number for the message, ensuring chronological order for a given component.
    serial BIGINT NOT NULL,
    -- The role of the sender of the message (e.g., "user", "system").
    role VARCHAR(255) NOT NULL,
    -- The content of the message.
    message TEXT NOT NULL,
    -- The timestamp when the message was created.
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    -- The timestamp when the message was last updated.
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    -- The primary key is a composite of `entity_id`, `component_name`, and `serial`,
    -- uniquely identifying each message.
    PRIMARY KEY (entity_id, component_name, serial)
);
