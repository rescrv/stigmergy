use serde_json::Value;

use crate::{Component, ComponentDefinition};

/// Validates component name and creates a Component
pub fn create_component(name: &str) -> Result<Component, String> {
    Component::new(name).ok_or_else(|| {
        format!(
            "Invalid component name '{}': must be a valid Rust type path",
            name
        )
    })
}

/// Validates and creates a ComponentDefinition with schema validation
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

/// Validates a schema against a temporary component definition
pub fn validate_schema_for_component(component_id: &str, schema: &Value) -> Result<(), String> {
    let component = create_component(component_id)?;
    let temp_definition = ComponentDefinition::new(component, schema.clone());
    temp_definition
        .validate_schema()
        .map_err(|e| format!("Schema validation failed: {}", e))
}

/// Parses JSON schema from string with error handling
pub fn parse_schema(schema_str: &str) -> Result<Value, String> {
    serde_json::from_str(schema_str).map_err(|e| format!("Invalid JSON schema: {}", e))
}

/// Parses JSON data from string with error handling
pub fn parse_json_data(data_str: &str) -> Result<Value, String> {
    serde_json::from_str(data_str).map_err(|e| format!("Invalid JSON data: {}", e))
}
