//! # Stigmergy: Emergent Coordination through Shared Data Structures
//!
//! Stigmergy is a biological concept describing how organisms coordinate their activities
//! through modifications to their shared environment. Ants building trails, termites
//! constructing mounds, and birds flocking all demonstrate stigmergic behavior - simple
//! local actions that produce complex global coordination without central control.
//!
//! This crate implements a stigmergic system for software, providing:
//!
//! - **Entity-Component Architecture**: Entities are unique identifiers that can have
//!   multiple components attached, following ECS (Entity Component System) patterns
//! - **Schema Validation**: Components are validated against JSON schemas to ensure
//!   data integrity and consistency
//! - **Persistent Logging**: All operations are logged to JSONL files for auditability,
//!   debugging, and system replay
//! - **HTTP API**: RESTful endpoints for managing entities and components
//! - **Configuration Management**: System configuration through frontmatter-delimited files
//!
//! ## Core Concepts
//!
//! ### Entities
//! Entities are 32-byte identifiers encoded as URL-safe base64 strings with an "entity:"
//! prefix. They serve as unique handles in the system and can be randomly generated or
//! explicitly specified.
//!
//! ### Components
//! Components are typed data structures that can be attached to entities. Each component
//! has a Rust-style type path identifier (e.g., "Position", "std::collections::HashMap")
//! and JSON data that must conform to a predefined schema.
//!
//! ### Validation
//! All component data is validated against JSON schemas that support:
//! - Primitive types (null, boolean, integer, number, string)
//! - Complex types (arrays, objects)
//! - Enumerations and union types via `oneOf`
//! - Nested structures with full recursion
//!
//! ### Persistence
//! Every operation is logged to JSONL (JSON Lines) files, creating an immutable audit
//! trail. These logs can be replayed to reconstruct system state or analyze behavior
//! over time.
//!
//! ## Architecture
//!
//! The system follows a layered architecture:
//!
//! ```text
//! ┌─────────────────────────────────────────┐
//! │ HTTP API Layer (Axum routes)            │
//! ├─────────────────────────────────────────┤
//! │ Business Logic (Component operations)   │
//! ├─────────────────────────────────────────┤
//! │ Data Operations (Standardized wrapper)  │
//! ├─────────────────────────────────────────┤
//! │ Data Store (Trait-based abstraction)    │
//! ├─────────────────────────────────────────┤
//! │ Persistence (JSONL logging & replay)    │
//! └─────────────────────────────────────────┘
//! ```
//!
//! ## Usage Examples
//!
//! ### Creating and Managing Entities
//!
//! ```rust
//! # use stigmergy::{Entity, InMemoryDataStore, DataStore};
//! # use std::sync::Arc;
//! // Create a new entity with a specific ID
//! let entity = Entity::new([1u8; 32]);
//! let entity_str = entity.to_string(); // "entity:AQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQE"
//!
//! // Parse an entity from its string representation
//! let parsed: Entity = entity_str.parse().unwrap();
//! assert_eq!(entity, parsed);
//!
//! // Generate a random entity
//! # #[cfg(feature = "random")]
//! let random_entity = Entity::random().unwrap();
//! ```
//!
//! ### Component Schema Definition
//!
//! Component schemas can be created manually or automatically generated using the derive macro:
//!
//! ```rust
//! # use stigmergy::{Component, ComponentDefinition, JsonSchema};
//! # use serde_json::json;
//! // Manual schema definition
//! let component = Component::new("Position").unwrap();
//!
//! let schema = json!({
//!     "type": "object",
//!     "properties": {
//!         "x": { "type": "number" },
//!         "y": { "type": "number" },
//!         "z": { "type": "number" }
//!     },
//!     "required": ["x", "y"]
//! });
//!
//! let definition = ComponentDefinition::new(component, schema);
//!
//! // Validate component data
//! let valid_data = json!({"x": 1.0, "y": 2.0, "z": 3.0});
//! assert!(definition.validate_component_data(&valid_data).is_ok());
//! ```
//!
//! ### Automatic Schema Generation
//!
//! Use the `JsonSchemaDerive` macro to automatically generate schemas for structs and enums:
//!
//! ```rust
//! # use stigmergy::JsonSchema;
//! # use serde_json::json;
//! // Define types with automatic schema generation
//! #[derive(stigmergy_derive::JsonSchema)]
//! struct Position {
//!     x: f64,
//!     y: f64,
//!     z: Option<f64>,
//! }
//!
//! #[derive(stigmergy_derive::JsonSchema)]
//! enum Status {
//!     Active,
//!     Inactive,
//!     Pending,
//! }
//!
//! #[derive(stigmergy_derive::JsonSchema)]
//! enum Shape {
//!     Circle { radius: f64 },
//!     Rectangle { width: f64, height: f64 },
//! }
//!
//! // Generate schemas automatically
//! let position_schema = Position::json_schema();
//! let status_schema = Status::json_schema();
//! let shape_schema = Shape::json_schema();
//!
//! // Unit enums become string enums
//! assert_eq!(status_schema["type"], "string");
//! assert!(status_schema["enum"].as_array().unwrap().len() == 3);
//!
//! // Complex enums use oneOf patterns
//! assert!(shape_schema["oneOf"].is_array());
//! ```
//!
//! ### Data Store Operations
//!
//! ```rust
//! # use stigmergy::{Entity, InMemoryDataStore, DataStore, Component, ComponentDefinition};
//! # use serde_json::json;
//! # use std::sync::Arc;
//! let data_store = Arc::new(InMemoryDataStore::new());
//!
//! // Create an entity
//! let entity = Entity::new([1u8; 32]);
//! data_store.create_entity(&entity).unwrap();
//!
//! // Create a component definition
//! let component = Component::new("Health").unwrap();
//! let definition = ComponentDefinition::new(
//!     component,
//!     json!({"type": "object", "properties": {"hp": {"type": "integer"}}})
//! );
//! data_store.create_component_definition("Health", &definition).unwrap();
//!
//! // Attach component data to the entity
//! let health_data = json!({"hp": 100});
//! data_store.create_component(&entity, "Health", &health_data).unwrap();
//!
//! // Retrieve the component
//! let retrieved = data_store.get_component(&entity, "Health").unwrap();
//! assert_eq!(retrieved, Some(health_data));
//! ```

#![deny(missing_docs)]
mod bid;
mod component;
mod data_operations;
mod data_store;
mod entity;
mod json_schema;
mod savefile;
mod system;
mod system_parser;
mod test_utils;
mod validate;

// CLI utility modules

/// Command-line interface utilities for program termination and output formatting.
///
/// This module provides common CLI utilities for stigmergy binaries, including
/// error handling, formatted output, and program termination functions.
pub mod cli_utils;

/// Component-specific utilities for validation and creation.
///
/// This module provides helper functions for working with components and component
/// definitions, including schema validation and error handling.
pub mod component_utils;

/// Command-line interface command handlers.
///
/// This module contains organized command handlers for the stigctl CLI application,
/// with each command type implemented in a dedicated submodule.
pub mod commands;

/// HTTP client utilities for interacting with stigmergy services.
///
/// This module provides a standardized HTTP client for communicating with
/// stigmergy HTTP APIs, handling requests, responses, and error conditions.
pub mod http_utils;

pub use bid::{Bid, BidParseError, BidParser, BinaryOperator, Expression, Position, UnaryOperator};
pub use component::{
    Component, ComponentDefinition, ComponentListItem, CreateComponentRequest,
    CreateComponentResponse, create_component_router,
};
pub use data_operations::{DataStoreOperations, OperationResult};
pub use data_store::{ComponentList, DataStore, DataStoreError, InMemoryDataStore};
pub use entity::{
    CreateEntityRequest, CreateEntityResponse, Entity, EntityParseError, create_entity_router,
};
pub use json_schema::{JsonSchema, JsonSchemaBuilder};
pub use savefile::{
    OperationStatus, RestoreResult, SaveEntry, SaveMetadata, SaveOperation, SavefileManager,
    ValidationResult, ValidationType,
};
pub use system::{
    CreateSystemFromMarkdownRequest, CreateSystemRequest, CreateSystemResponse, System, SystemId,
    SystemIdParseError, SystemListItem, create_system_router,
};
pub use system_parser::{ParseError, SystemConfig, SystemParser};
pub use validate::{ValidationError, validate_value};
