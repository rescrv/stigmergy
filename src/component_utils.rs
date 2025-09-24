//! # Component Utilities
//!
//! This module provides utility functions for working with components and component
//! definitions, including validation, creation, and schema handling.
//!
//! ## Key Features
//!
//! - **Component Creation**: Safe creation of component types with validation
//! - **Schema Validation**: Comprehensive JSON schema validation utilities
//! - **Error Handling**: User-friendly error messages for common component operations
//! - **JSON Parsing**: Robust parsing of JSON schemas and component data
//!
//! ## Usage Examples
//!
//! ```rust
//! use stigmergy::component_utils;
//! use serde_json::json;
//!
//! // Create and validate a component definition
//! let schema = json!({"type": "object", "properties": {"x": {"type": "number"}}});
//! let definition = component_utils::create_and_validate_definition("Position", schema).unwrap();
//!
//! // Parse JSON schema from string
//! let schema_str = r#"{"type": "string"}"#;
//! let schema = component_utils::parse_schema(schema_str).unwrap();
//! ```

use serde_json::Value;

use crate::{Component, ComponentDefinition};

/// Creates a Component from a name string with validation.
///
/// This function validates that the component name follows Rust identifier
/// conventions before creating the Component instance.
///
/// # Arguments
/// * `name` - The component name to validate and use
///
/// # Returns
/// * `Ok(Component)` - Successfully created component
/// * `Err(String)` - Validation error with descriptive message
///
/// # Examples
/// ```
/// use stigmergy::component_utils::create_component;
///
/// let component = create_component("Position").unwrap();
/// assert!(create_component("invalid-name").is_err());
/// ```
pub fn create_component(name: &str) -> Result<Component, String> {
    Component::new(name).ok_or_else(|| {
        format!(
            "Invalid component name '{}': must be a valid Rust type path",
            name
        )
    })
}

/// Creates and validates a ComponentDefinition with comprehensive schema checking.
///
/// This function combines component creation and schema validation into a single
/// operation, ensuring both the component name and schema are valid before
/// returning a ComponentDefinition.
///
/// # Arguments
/// * `name` - The component name to validate
/// * `schema` - The JSON schema to validate and use
///
/// # Returns
/// * `Ok(ComponentDefinition)` - Successfully created and validated definition
/// * `Err(String)` - Validation error describing the specific problem
///
/// # Examples
/// ```
/// use stigmergy::component_utils::create_and_validate_definition;
/// use serde_json::json;
///
/// let schema = json!({"type": "object", "required": ["x", "y"]});
/// let definition = create_and_validate_definition("Position", schema).unwrap();
/// ```
pub fn create_and_validate_definition(
    name: &str,
    schema: Value,
) -> Result<ComponentDefinition, String> {
    let component = create_component(name)?;
    let definition = ComponentDefinition::new(component, schema);

    definition
        .validate_schema()
        .map_err(|e| format!("Schema validation failed: {}", e))?;

    Ok(definition)
}

/// Validates a JSON schema by creating a temporary component definition.
///
/// This function performs schema validation without permanently creating a
/// component definition, useful for checking schema validity before storage.
///
/// # Arguments
/// * `component_id` - The component identifier to use for validation
/// * `schema` - The JSON schema to validate
///
/// # Returns
/// * `Ok(())` - Schema is valid
/// * `Err(String)` - Validation error with specific details
///
/// # Examples
/// ```
/// use stigmergy::component_utils::validate_schema_for_component;
/// use serde_json::json;
///
/// let schema = json!({"type": "number"});
/// assert!(validate_schema_for_component("Health", &schema).is_ok());
/// ```
pub fn validate_schema_for_component(component_id: &str, schema: &Value) -> Result<(), String> {
    let component = create_component(component_id)?;
    let temp_definition = ComponentDefinition::new(component, schema.clone());
    temp_definition
        .validate_schema()
        .map_err(|e| format!("Schema validation failed: {}", e))
}

/// Parses a JSON schema from a string with comprehensive error handling.
///
/// This function provides user-friendly error messages for JSON parsing failures,
/// making it easier to debug schema definition problems.
///
/// # Arguments
/// * `schema_str` - The JSON schema string to parse
///
/// # Returns
/// * `Ok(Value)` - Successfully parsed JSON schema
/// * `Err(String)` - Parse error with descriptive message
///
/// # Examples
/// ```
/// use stigmergy::component_utils::parse_schema;
///
/// let valid_schema = parse_schema(r#"{"type": "string"}"#).unwrap();
/// assert!(parse_schema("invalid json").is_err());
/// ```
pub fn parse_schema(schema_str: &str) -> Result<Value, String> {
    serde_json::from_str(schema_str).map_err(|e| format!("Invalid JSON schema: {}", e))
}

/// Parses JSON data from a string with comprehensive error handling.
///
/// This function provides user-friendly error messages for JSON parsing failures,
/// specifically designed for component data validation.
///
/// # Arguments
/// * `data_str` - The JSON data string to parse
///
/// # Returns
/// * `Ok(Value)` - Successfully parsed JSON data
/// * `Err(String)` - Parse error with descriptive message
///
/// # Examples
/// ```
/// use stigmergy::component_utils::parse_json_data;
///
/// let valid_data = parse_json_data(r#"{"x": 10, "y": 20}"#).unwrap();
/// assert!(parse_json_data("invalid json").is_err());
/// ```
pub fn parse_json_data(data_str: &str) -> Result<Value, String> {
    serde_json::from_str(data_str).map_err(|e| format!("Invalid JSON data: {}", e))
}
