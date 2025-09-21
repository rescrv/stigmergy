use std::collections::HashMap;

use axum::Router;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::Json;
use axum::routing::get;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;

use crate::{
    DurableLogger, LogEntry, LogMetadata, LogOperation, OperationStatus, ValidationError,
    ValidationResult as LogValidationResult, validate_value,
};

///////////////////////////////////////////// Component ////////////////////////////////////////////

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Component(String);

impl Component {
    pub fn new(c: impl Into<String>) -> Option<Component> {
        let s = c.into();
        if is_valid_rust_type_path(&s) {
            Some(Component(s))
        } else {
            None
        }
    }
}

fn is_valid_rust_identifier(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }

    let mut chars = s.chars();
    let first = chars.next().unwrap();

    // First character must be a letter or underscore
    if !first.is_ascii_alphabetic() && first != '_' {
        return false;
    }

    // Remaining characters must be alphanumeric or underscore
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn is_valid_rust_type_path(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }

    // Split by :: to handle type paths like ghai::Issue
    let segments: Vec<&str> = s.split("::").collect();

    // Each segment must be a valid identifier
    segments
        .iter()
        .all(|segment| is_valid_rust_identifier(segment))
}

//////////////////////////////////////// ComponentDefinition ///////////////////////////////////////

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentDefinition {
    pub component: Component,
    pub schema: serde_json::Value,
}

impl ComponentDefinition {
    pub fn new(component: Component, schema: Value) -> Self {
        Self { component, schema }
    }

    pub fn validate_schema(&self) -> Result<(), ValidationError> {
        validate_schema_structure(&self.schema)
    }

    pub fn validate_component_data(&self, data: &Value) -> Result<(), ValidationError> {
        validate_value(data, &self.schema)
    }
}

fn validate_schema_structure(schema: &Value) -> Result<(), ValidationError> {
    if !schema.is_object() {
        return Err(ValidationError::InvalidSchema(
            "Schema must be an object".to_string(),
        ));
    }

    let schema_obj = schema.as_object().unwrap();

    if let Some(one_of) = schema_obj.get("oneOf") {
        if !one_of.is_array() {
            return Err(ValidationError::InvalidSchema(
                "oneOf must be an array".to_string(),
            ));
        }

        for (i, sub_schema) in one_of.as_array().unwrap().iter().enumerate() {
            validate_schema_structure(sub_schema).map_err(|e| {
                ValidationError::InvalidSchema(format!(
                    "Invalid oneOf schema at index {}: {}",
                    i, e
                ))
            })?;
        }
        return Ok(());
    }

    if let Some(schema_type) = schema_obj.get("type") {
        if !schema_type.is_string() {
            return Err(ValidationError::InvalidSchema(
                "Schema type must be a string".to_string(),
            ));
        }

        let type_str = schema_type.as_str().unwrap();
        match type_str {
            "null" | "boolean" | "integer" | "number" | "string" => Ok(()),
            "array" => {
                if let Some(items) = schema_obj.get("items") {
                    validate_schema_structure(items)
                } else {
                    Ok(())
                }
            }
            "object" => {
                if let Some(properties) = schema_obj.get("properties") {
                    if !properties.is_object() {
                        return Err(ValidationError::InvalidSchema(
                            "Properties must be an object".to_string(),
                        ));
                    }

                    for (prop_name, prop_schema) in properties.as_object().unwrap() {
                        validate_schema_structure(prop_schema).map_err(|e| {
                            ValidationError::InvalidSchema(format!(
                                "Invalid property schema '{}': {}",
                                prop_name, e
                            ))
                        })?;
                    }
                }
                Ok(())
            }
            _ => Err(ValidationError::InvalidSchema(format!(
                "Unknown schema type: {}",
                type_str
            ))),
        }
    } else {
        Err(ValidationError::InvalidSchema(
            "Schema must have either 'type' or 'oneOf'".to_string(),
        ))
    }
}

////////////////////////////////////////////// routes //////////////////////////////////////////////

async fn get_component_definitions(
    State(logger): State<Arc<DurableLogger>>,
    Query(_params): Query<HashMap<String, String>>,
) -> Result<Json<Vec<ComponentDefinition>>, StatusCode> {
    let log_entry = LogEntry::new(
        LogOperation::ComponentDefinitionGet {
            definition_id: None,
            found: true,
        },
        LogMetadata::rest_api(None),
    );
    logger.log_or_error(&log_entry);
    Ok(Json(vec![]))
}

async fn create_component_definition(
    State(logger): State<Arc<DurableLogger>>,
    Json(definition): Json<ComponentDefinition>,
) -> Result<Json<ComponentDefinition>, StatusCode> {
    let validation_result = match definition.validate_schema() {
        Ok(()) => LogValidationResult::success(),
        Err(e) => {
            let log_entry = LogEntry::new(
                LogOperation::ComponentDefinitionCreate {
                    definition: definition.clone(),
                    validation_result: LogValidationResult::failed(e.to_string()),
                },
                LogMetadata::rest_api(None).with_status(OperationStatus::Failed),
            );
            logger.log_or_error(&log_entry);
            eprintln!("Invalid component definition schema: {}", e); // TODO(claude): cleanup this output
            return Err(StatusCode::BAD_REQUEST);
        }
    };

    let log_entry = LogEntry::new(
        LogOperation::ComponentDefinitionCreate {
            definition: definition.clone(),
            validation_result,
        },
        LogMetadata::rest_api(None),
    );
    logger.log_or_error(&log_entry);

    Ok(Json(definition))
}

async fn update_component_definition(
    State(logger): State<Arc<DurableLogger>>,
    Json(definition): Json<ComponentDefinition>,
) -> Result<Json<ComponentDefinition>, StatusCode> {
    let validation_result = match definition.validate_schema() {
        Ok(()) => LogValidationResult::success(),
        Err(e) => {
            let log_entry = LogEntry::new(
                LogOperation::ComponentDefinitionUpdate {
                    definition_id: format!("{:?}", definition.component),
                    old_definition: None,
                    new_definition: definition.clone(),
                    validation_result: LogValidationResult::failed(e.to_string()),
                },
                LogMetadata::rest_api(None).with_status(OperationStatus::Failed),
            );
            logger.log_or_error(&log_entry);
            eprintln!("Invalid component definition schema: {}", e); // TODO(claude): cleanup this output
            return Err(StatusCode::BAD_REQUEST);
        }
    };

    let log_entry = LogEntry::new(
        LogOperation::ComponentDefinitionUpdate {
            definition_id: format!("{:?}", definition.component),
            old_definition: None,
            new_definition: definition.clone(),
            validation_result,
        },
        LogMetadata::rest_api(None),
    );
    logger.log_or_error(&log_entry);

    Ok(Json(definition))
}

async fn patch_component_definition(
    State(logger): State<Arc<DurableLogger>>,
    Json(patch): Json<Value>,
) -> Result<Json<ComponentDefinition>, StatusCode> {
    let component = Component::new("PatchedComponent").unwrap();
    let definition = ComponentDefinition {
        component,
        schema: patch.clone(),
    };

    let log_entry = LogEntry::new(
        LogOperation::ComponentDefinitionPatch {
            definition_id: "PatchedComponent".to_string(),
            patch_data: patch,
            result_definition: definition.clone(),
        },
        LogMetadata::rest_api(None),
    );
    logger.log_or_error(&log_entry);

    Ok(Json(definition))
}

async fn delete_component_definitions(
    State(logger): State<Arc<DurableLogger>>,
) -> Result<StatusCode, StatusCode> {
    let log_entry = LogEntry::new(
        LogOperation::ComponentDefinitionDeleteAll { count_deleted: 0 },
        LogMetadata::rest_api(None),
    );
    logger.log_or_error(&log_entry);

    Ok(StatusCode::NO_CONTENT)
}

async fn get_component_definition_by_id(
    State(logger): State<Arc<DurableLogger>>,
    Path(id): Path<String>,
) -> Result<Json<ComponentDefinition>, StatusCode> {
    let component = Component::new(format!("Component{}", id))
        .unwrap_or_else(|| Component::new("DefaultComponent").unwrap());
    let definition = ComponentDefinition {
        component,
        schema: serde_json::json!({}),
    };

    let log_entry = LogEntry::new(
        LogOperation::ComponentDefinitionGet {
            definition_id: Some(id),
            found: true,
        },
        LogMetadata::rest_api(None),
    );
    logger.log_or_error(&log_entry);

    Ok(Json(definition))
}

async fn update_component_definition_by_id(
    State(logger): State<Arc<DurableLogger>>,
    Path(id): Path<String>,
    Json(definition): Json<ComponentDefinition>,
) -> Result<Json<ComponentDefinition>, StatusCode> {
    let validation_result = match definition.validate_schema() {
        Ok(()) => LogValidationResult::success(),
        Err(e) => {
            let log_entry = LogEntry::new(
                LogOperation::ComponentDefinitionUpdate {
                    definition_id: id.clone(),
                    old_definition: None,
                    new_definition: definition.clone(),
                    validation_result: LogValidationResult::failed(e.to_string()),
                },
                LogMetadata::rest_api(None).with_status(OperationStatus::Failed),
            );
            logger.log_or_error(&log_entry);
            eprintln!("Invalid component definition schema: {}", e); // TODO(claude): cleanup this output
            return Err(StatusCode::BAD_REQUEST);
        }
    };

    let log_entry = LogEntry::new(
        LogOperation::ComponentDefinitionUpdate {
            definition_id: id,
            old_definition: None,
            new_definition: definition.clone(),
            validation_result,
        },
        LogMetadata::rest_api(None),
    );
    logger.log_or_error(&log_entry);

    Ok(Json(definition))
}

async fn patch_component_definition_by_id(
    State(logger): State<Arc<DurableLogger>>,
    Path(id): Path<String>,
    Json(patch): Json<Value>,
) -> Result<Json<ComponentDefinition>, StatusCode> {
    let component = Component::new(format!("Component{}", id))
        .unwrap_or_else(|| Component::new("PatchedComponent").unwrap());
    let definition = ComponentDefinition {
        component,
        schema: patch.clone(),
    };

    let log_entry = LogEntry::new(
        LogOperation::ComponentDefinitionPatch {
            definition_id: id,
            patch_data: patch,
            result_definition: definition.clone(),
        },
        LogMetadata::rest_api(None),
    );
    logger.log_or_error(&log_entry);

    Ok(Json(definition))
}

async fn delete_component_definition_by_id(
    State(logger): State<Arc<DurableLogger>>,
    Path(id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    let log_entry = LogEntry::new(
        LogOperation::ComponentDefinitionDelete {
            definition_id: id,
            deleted_definition: None,
        },
        LogMetadata::rest_api(None),
    );
    logger.log_or_error(&log_entry);

    Ok(StatusCode::NO_CONTENT)
}

async fn get_components(
    State(logger): State<Arc<DurableLogger>>,
    Query(_params): Query<HashMap<String, String>>,
) -> Result<Json<Vec<Value>>, StatusCode> {
    let log_entry = LogEntry::new(
        LogOperation::ComponentGet {
            component_id: None,
            found: true,
        },
        LogMetadata::rest_api(None),
    );
    logger.log_or_error(&log_entry);

    Ok(Json(vec![]))
}

async fn create_component(
    State(logger): State<Arc<DurableLogger>>,
    Json(component): Json<Value>,
) -> Result<Json<Value>, StatusCode> {
    // TODO(user): Implement actual component definition lookup from data store integration
    // For demonstration, validate against a sample enum schema
    let sample_enum_schema = serde_json::json!({
        "oneOf": [
            {
                "type": "string",
                "enum": ["Red", "Green", "Blue"]
            },
            {
                "type": "object",
                "properties": {
                    "Custom": { "type": "string" }
                },
                "required": ["Custom"]
            }
        ]
    });

    let validation_result = match validate_value(&component, &sample_enum_schema) {
        Ok(()) => Some(LogValidationResult::success()),
        Err(e) => {
            let log_entry = LogEntry::new(
                LogOperation::ComponentCreate {
                    component_id: "generated_id".to_string(),
                    component_data: component.clone(),
                    validation_result: Some(LogValidationResult::failed(e.to_string())),
                },
                LogMetadata::rest_api(None).with_status(OperationStatus::Failed),
            );
            logger.log_or_error(&log_entry);
            eprintln!("Component validation failed: {}", e); // TODO(claude): cleanup this output
            return Err(StatusCode::BAD_REQUEST);
        }
    };

    let log_entry = LogEntry::new(
        LogOperation::ComponentCreate {
            component_id: "generated_id".to_string(),
            component_data: component.clone(),
            validation_result,
        },
        LogMetadata::rest_api(None),
    );
    logger.log_or_error(&log_entry);

    Ok(Json(component))
}

async fn update_component(
    State(logger): State<Arc<DurableLogger>>,
    Json(component): Json<Value>,
) -> Result<Json<Value>, StatusCode> {
    let log_entry = LogEntry::new(
        LogOperation::ComponentUpdate {
            component_id: "updated_id".to_string(),
            old_data: None,
            new_data: component.clone(),
            validation_result: None,
        },
        LogMetadata::rest_api(None),
    );
    logger.log_or_error(&log_entry);

    Ok(Json(component))
}

async fn patch_component(
    State(logger): State<Arc<DurableLogger>>,
    Json(patch): Json<Value>,
) -> Result<Json<Value>, StatusCode> {
    let log_entry = LogEntry::new(
        LogOperation::ComponentPatch {
            component_id: "patched_id".to_string(),
            patch_data: patch.clone(),
            result_data: patch.clone(),
        },
        LogMetadata::rest_api(None),
    );
    logger.log_or_error(&log_entry);

    Ok(Json(patch))
}

async fn delete_components(
    State(logger): State<Arc<DurableLogger>>,
) -> Result<StatusCode, StatusCode> {
    let log_entry = LogEntry::new(
        LogOperation::ComponentDeleteAll { count_deleted: 0 },
        LogMetadata::rest_api(None),
    );
    logger.log_or_error(&log_entry);

    Ok(StatusCode::NO_CONTENT)
}

async fn get_component_by_id(
    State(logger): State<Arc<DurableLogger>>,
    Path(id): Path<String>,
) -> Result<Json<Value>, StatusCode> {
    let component = serde_json::json!({
        "id": id.clone(),
        "data": {}
    });

    let log_entry = LogEntry::new(
        LogOperation::ComponentGet {
            component_id: Some(id),
            found: true,
        },
        LogMetadata::rest_api(None),
    );
    logger.log_or_error(&log_entry);

    Ok(Json(component))
}

async fn update_component_by_id(
    State(logger): State<Arc<DurableLogger>>,
    Path(id): Path<String>,
    Json(component): Json<Value>,
) -> Result<Json<Value>, StatusCode> {
    let log_entry = LogEntry::new(
        LogOperation::ComponentUpdate {
            component_id: id,
            old_data: None,
            new_data: component.clone(),
            validation_result: None,
        },
        LogMetadata::rest_api(None),
    );
    logger.log_or_error(&log_entry);

    Ok(Json(component))
}

async fn patch_component_by_id(
    State(logger): State<Arc<DurableLogger>>,
    Path(id): Path<String>,
    Json(patch): Json<Value>,
) -> Result<Json<Value>, StatusCode> {
    let mut component = patch.clone();
    if let Some(obj) = component.as_object_mut() {
        obj.insert("id".to_string(), serde_json::Value::String(id.clone()));
    }

    let log_entry = LogEntry::new(
        LogOperation::ComponentPatch {
            component_id: id,
            patch_data: patch,
            result_data: component.clone(),
        },
        LogMetadata::rest_api(None),
    );
    logger.log_or_error(&log_entry);

    Ok(Json(component))
}

async fn delete_component_by_id(
    State(logger): State<Arc<DurableLogger>>,
    Path(id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    let log_entry = LogEntry::new(
        LogOperation::ComponentDelete {
            component_id: id,
            deleted_data: None,
        },
        LogMetadata::rest_api(None),
    );
    logger.log_or_error(&log_entry);

    Ok(StatusCode::NO_CONTENT)
}

////////////////////////////////////////////// router //////////////////////////////////////////////

pub fn create_component_router(logger: Arc<DurableLogger>) -> Router {
    Router::new()
        .route(
            "/componentdefinition",
            get(get_component_definitions)
                .post(create_component_definition)
                .put(update_component_definition)
                .patch(patch_component_definition)
                .delete(delete_component_definitions),
        )
        .route(
            "/componentdefinition/:id",
            get(get_component_definition_by_id)
                .put(update_component_definition_by_id)
                .patch(patch_component_definition_by_id)
                .delete(delete_component_definition_by_id),
        )
        .route(
            "/component",
            get(get_components)
                .post(create_component)
                .put(update_component)
                .patch(patch_component)
                .delete(delete_components),
        )
        .route(
            "/component/:id",
            get(get_component_by_id)
                .put(update_component_by_id)
                .patch(patch_component_by_id)
                .delete(delete_component_by_id),
        )
        .with_state(logger)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_rust_identifier_simple() {
        assert!(is_valid_rust_identifier("foo"));
        assert!(is_valid_rust_identifier("_bar"));
        assert!(is_valid_rust_identifier("baz123"));
        assert!(is_valid_rust_identifier("_"));
    }

    #[test]
    fn invalid_rust_identifier() {
        assert!(!is_valid_rust_identifier(""));
        assert!(!is_valid_rust_identifier("123foo"));
        assert!(!is_valid_rust_identifier("foo-bar"));
        assert!(!is_valid_rust_identifier("foo::bar"));
    }

    #[test]
    fn valid_rust_type_path_simple() {
        assert!(is_valid_rust_type_path("String"));
        assert!(is_valid_rust_type_path("_Foo"));
        assert!(is_valid_rust_type_path("MyType123"));
    }

    #[test]
    fn valid_rust_type_path_with_modules() {
        assert!(is_valid_rust_type_path("std::String"));
        assert!(is_valid_rust_type_path("ghai::Issue"));
        assert!(is_valid_rust_type_path("my_crate::module::Type"));
        assert!(is_valid_rust_type_path("a::b::c::d::Type"));
    }

    #[test]
    fn invalid_rust_type_path() {
        assert!(!is_valid_rust_type_path(""));
        assert!(!is_valid_rust_type_path("::"));
        assert!(!is_valid_rust_type_path("foo::"));
        assert!(!is_valid_rust_type_path("::foo"));
        assert!(!is_valid_rust_type_path("foo::::bar"));
        assert!(!is_valid_rust_type_path("123::foo"));
        assert!(!is_valid_rust_type_path("foo::123"));
        assert!(!is_valid_rust_type_path("foo-bar::baz"));
    }

    #[test]
    fn component_new_with_valid_type_paths() {
        assert!(Component::new("String").is_some());
        assert!(Component::new("ghai::Issue").is_some());
        assert!(Component::new("std::collections::HashMap").is_some());
    }

    #[test]
    fn component_new_with_invalid_type_paths() {
        assert!(Component::new("").is_none());
        assert!(Component::new("::").is_none());
        assert!(Component::new("foo::").is_none());
        assert!(Component::new("123::foo").is_none());
    }

    #[test]
    fn component_definition_validate_simple_schema() {
        let definition = ComponentDefinition {
            component: Component::new("TestComponent").unwrap(),
            schema: serde_json::json!({
                "type": "string"
            }),
        };

        assert!(definition.validate_schema().is_ok());
    }

    #[test]
    fn component_definition_validate_enum_schema() {
        let definition = ComponentDefinition {
            component: Component::new("ColorComponent").unwrap(),
            schema: serde_json::json!({
                "oneOf": [
                    {
                        "type": "string",
                        "enum": ["Red", "Green", "Blue"]
                    },
                    {
                        "type": "object",
                        "properties": {
                            "Custom": { "type": "string" }
                        },
                        "required": ["Custom"]
                    }
                ]
            }),
        };

        assert!(definition.validate_schema().is_ok());
    }

    #[test]
    fn component_definition_validate_invalid_schema() {
        let definition = ComponentDefinition {
            component: Component::new("InvalidComponent").unwrap(),
            schema: serde_json::json!({
                "type": "invalid_type"
            }),
        };

        assert!(definition.validate_schema().is_err());
    }

    #[test]
    fn component_definition_validate_component_data() {
        let definition = ComponentDefinition {
            component: Component::new("ColorComponent").unwrap(),
            schema: serde_json::json!({
                "oneOf": [
                    {
                        "type": "string",
                        "enum": ["Red", "Green", "Blue"]
                    },
                    {
                        "type": "object",
                        "properties": {
                            "Custom": { "type": "string" }
                        },
                        "required": ["Custom"]
                    }
                ]
            }),
        };

        assert!(
            definition
                .validate_component_data(&serde_json::json!("Red"))
                .is_ok()
        );
        assert!(
            definition
                .validate_component_data(&serde_json::json!({"Custom": "purple"}))
                .is_ok()
        );
        assert!(
            definition
                .validate_component_data(&serde_json::json!("Yellow"))
                .is_err()
        );
        assert!(
            definition
                .validate_component_data(&serde_json::json!(42))
                .is_err()
        );
    }

    #[test]
    fn component_definition_validate_tagged_union_data() {
        let definition = ComponentDefinition {
            component: Component::new("ShapeComponent").unwrap(),
            schema: serde_json::json!({
                "oneOf": [
                    {
                        "type": "object",
                        "properties": {
                            "Circle": {
                                "type": "object",
                                "properties": {
                                    "radius": { "type": "number" }
                                },
                                "required": ["radius"]
                            }
                        },
                        "required": ["Circle"]
                    },
                    {
                        "type": "object",
                        "properties": {
                            "Rectangle": {
                                "type": "object",
                                "properties": {
                                    "width": { "type": "number" },
                                    "height": { "type": "number" }
                                },
                                "required": ["width", "height"]
                            }
                        },
                        "required": ["Rectangle"]
                    }
                ]
            }),
        };

        assert!(
            definition
                .validate_component_data(&serde_json::json!({"Circle": {"radius": 5.0}}))
                .is_ok()
        );
        assert!(
            definition
                .validate_component_data(
                    &serde_json::json!({"Rectangle": {"width": 10.0, "height": 20.0}})
                )
                .is_ok()
        );
        assert!(
            definition
                .validate_component_data(&serde_json::json!({"Circle": {"radius": "invalid"}}))
                .is_err()
        );
        assert!(
            definition
                .validate_component_data(&serde_json::json!({"Triangle": {"side": 5.0}}))
                .is_err()
        );
    }
}
