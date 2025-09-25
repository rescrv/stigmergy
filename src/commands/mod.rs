//! # Command Handlers
//!
//! This module contains organized command handlers for the stigctl CLI application.
//! Each command type is implemented in a dedicated submodule for better organization
//! and maintainability.
//!
//! ## Structure
//!
//! - `entity` - Entity management commands (create, list, delete)
//! - `system` - System management commands (create, list, get, update, delete)
//! - `component_definition` - Component definition commands (create, list, get, update, delete)
//! - `component` - Component instance commands (create, list, get, update, delete)
//! - `apply` - JSONL operation application commands
//! - `shared` - Shared utilities and validation functions

pub mod apply;
pub mod component;
pub mod component_definition;
pub mod entity;
pub mod error_extensions;
pub mod errors;
pub mod shared;
pub mod system;

pub use apply::handle_apply_command;
pub use component::handle_component_command;
pub use component_definition::handle_componentdefinition_command;
pub use entity::handle_entity_command;
pub use system::handle_system_command;
