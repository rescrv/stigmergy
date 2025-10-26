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
//! - **PostgreSQL Storage**: All entities and components are persisted in PostgreSQL
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
//! All entities, components, and their relationships are stored in PostgreSQL with
//! automatic timestamp tracking for creation and modification events.
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
//! │ PostgreSQL Database (Persistent storage)│
//! └─────────────────────────────────────────┘
//! ```
//!
//! ## Usage Examples
//!
//! ### Creating and Managing Entities
//!
//! ```rust
//! # use stigmergy::Entity;
//! // Create a new entity with a specific ID
//! let entity = Entity::new([1u8; 32]);
//! let entity_str = entity.to_string(); // "entity:AQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQE"
//!
//! // Parse an entity from its string representation
//! let parsed: Entity = entity_str.parse().unwrap();
//! assert_eq!(entity, parsed);
//!
//! // Generate a random entity
//! let random_entity = Entity::random_url_safe().unwrap();
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

#![deny(missing_docs)]
mod apply;
mod bid;
mod component;
mod component_definition;
mod config;
mod edge;
mod entity;
mod errors;
mod invariant;
mod json_schema;
mod system;
mod system_parser;
mod test_utils;
mod validate;

// Public modules

/// PostgreSQL database operations for stigmergy.
///
/// This module provides functions for interacting with the PostgreSQL database,
/// including entity management with automatic timestamp tracking.
pub mod sql;

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

pub use apply::{create_apply_router, ApplyRequest, ApplyResponse, Operation, OperationResult};
pub use bid::{
    Bid, BidParseError, BidParser, BinaryOperator, EntityResolver, EvaluationError, Expression,
    Position, UnaryOperator,
};
pub use component::{
    create_component_instance_router, Component, ComponentListItem, CreateComponentRequest,
    CreateComponentResponse,
};
pub use component_definition::{create_component_definition_router, ComponentDefinition};
pub use config::{
    create_config_router, load_latest_config, save_config, Config, GetConfigResponse, IoSystem,
    PostConfigRequest, PostConfigResponse,
};
pub use edge::{create_edge_router, CreateEdgeRequest, CreateEdgeResponse, Edge};
pub use entity::{
    create_entity_router, CreateEntityRequest, CreateEntityResponse, Entity, EntityParseError,
};
pub use errors::DataStoreError;
pub use invariant::{
    create_invariant_router, CreateInvariantRequest, CreateInvariantResponse, GetInvariantResponse,
    InvariantID, InvariantIDParseError, UpdateInvariantRequest,
};
pub use json_schema::{JsonSchema, JsonSchemaBuilder};
pub use system::{
    create_system_router, CreateSystemFromMarkdownRequest, CreateSystemResponse, System,
    SystemListItem, SystemName, SystemNameParseError,
};
pub use system_parser::{AccessMode, ComponentAccess, ParseError, SystemConfig, SystemParser};
pub use validate::{validate_value, ValidationError};
