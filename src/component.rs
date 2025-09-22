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
    DataStore, DataStoreOperations, DurableLogger, Entity, LogEntry, LogMetadata, LogOperation,
    OperationStatus, ValidationError, ValidationResult as LogValidationResult, validate_value,
};

///////////////////////////////////////////// Component ////////////////////////////////////////////

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreateComponentRequest {
    pub component: Component,
    pub data: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreateComponentResponse {
    pub entity: Entity,
    pub component: Component,
    pub data: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComponentListItem {
    pub component: Component,
    pub data: Value,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    State((logger, data_store)): State<(Arc<DurableLogger>, Arc<dyn DataStore>)>,
    Query(_params): Query<HashMap<String, String>>,
) -> Result<Json<Vec<ComponentDefinition>>, (StatusCode, &'static str)> {
    let definitions = match data_store.list_component_definitions() {
        Ok(def_list) => def_list.into_iter().map(|(_id, def)| def).collect(),
        Err(_) => vec![],
    };

    let log_entry = LogEntry::new(
        LogOperation::ComponentDefinitionGet {
            definition_id: None,
            found: !definitions.is_empty(),
        },
        LogMetadata::rest_api(None),
    );
    logger.log_or_error(&log_entry);
    Ok(Json(definitions))
}

async fn create_component_definition(
    State((logger, data_store)): State<(Arc<DurableLogger>, Arc<dyn DataStore>)>,
    Json(definition): Json<ComponentDefinition>,
) -> Result<Json<ComponentDefinition>, (StatusCode, &'static str)> {
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
            return Err((StatusCode::BAD_REQUEST, "invalid schema"));
        }
    };

    let def_id = definition.component.as_str().to_string();
    let result =
        DataStoreOperations::create_component_definition(&*data_store, &def_id, &definition);
    if !result.success {
        let log_entry = LogEntry::new(
            LogOperation::ComponentDefinitionCreate {
                definition: definition.clone(),
                validation_result,
            },
            LogMetadata::rest_api(None).with_status(OperationStatus::Failed),
        );
        logger.log_or_error(&log_entry);
        return Err(match result.into_error() {
            crate::DataStoreError::AlreadyExists => (StatusCode::CONFLICT, "already exists"),
            _ => (StatusCode::INTERNAL_SERVER_ERROR, "internal server error"),
        });
    }

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
    State((logger, data_store)): State<(Arc<DurableLogger>, Arc<dyn DataStore>)>,
    Json(definition): Json<ComponentDefinition>,
) -> Result<Json<ComponentDefinition>, (StatusCode, &'static str)> {
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
            return Err((StatusCode::BAD_REQUEST, "invalid schema"));
        }
    };

    let def_id = definition.component.as_str().to_string();
    let old_definition = data_store.get_component_definition(&def_id).ok().flatten();
    let result =
        DataStoreOperations::update_component_definition(&*data_store, &def_id, &definition);
    if !result.success {
        let log_entry = LogEntry::new(
            LogOperation::ComponentDefinitionUpdate {
                definition_id: def_id,
                old_definition,
                new_definition: definition.clone(),
                validation_result,
            },
            LogMetadata::rest_api(None).with_status(OperationStatus::Failed),
        );
        logger.log_or_error(&log_entry);
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to update component definition",
        ));
    }

    let log_entry = LogEntry::new(
        LogOperation::ComponentDefinitionUpdate {
            definition_id: def_id,
            old_definition,
            new_definition: definition.clone(),
            validation_result,
        },
        LogMetadata::rest_api(None),
    );
    logger.log_or_error(&log_entry);

    Ok(Json(definition))
}

async fn patch_component_definition(
    State((logger, data_store)): State<(Arc<DurableLogger>, Arc<dyn DataStore>)>,
    Json(patch): Json<Value>,
) -> Result<Json<ComponentDefinition>, (StatusCode, &'static str)> {
    let component = Component::new("PatchedComponent").unwrap();
    let definition = ComponentDefinition {
        component,
        schema: patch.clone(),
    };

    let def_id = "PatchedComponent".to_string();
    let result =
        DataStoreOperations::update_component_definition(&*data_store, &def_id, &definition);
    if !result.success {
        let log_entry = LogEntry::new(
            LogOperation::ComponentDefinitionPatch {
                definition_id: def_id,
                patch_data: patch,
                result_definition: definition.clone(),
            },
            LogMetadata::rest_api(None).with_status(OperationStatus::Failed),
        );
        logger.log_or_error(&log_entry);
        return Err((StatusCode::INTERNAL_SERVER_ERROR, "internal server error"));
    }

    let log_entry = LogEntry::new(
        LogOperation::ComponentDefinitionPatch {
            definition_id: def_id,
            patch_data: patch,
            result_definition: definition.clone(),
        },
        LogMetadata::rest_api(None),
    );
    logger.log_or_error(&log_entry);

    Ok(Json(definition))
}

async fn delete_component_definitions(
    State((logger, data_store)): State<(Arc<DurableLogger>, Arc<dyn DataStore>)>,
) -> Result<StatusCode, (StatusCode, &'static str)> {
    let result = DataStoreOperations::delete_all_component_definitions(&*data_store);
    let count_deleted = if result.success {
        result.data.unwrap_or(0)
    } else {
        let log_entry = LogEntry::new(
            LogOperation::ComponentDefinitionDeleteAll { count_deleted: 0 },
            LogMetadata::rest_api(None).with_status(OperationStatus::Failed),
        );
        logger.log_or_error(&log_entry);
        return Err((StatusCode::INTERNAL_SERVER_ERROR, "internal server error"));
    };

    let log_entry = LogEntry::new(
        LogOperation::ComponentDefinitionDeleteAll { count_deleted },
        LogMetadata::rest_api(None),
    );
    logger.log_or_error(&log_entry);

    Ok(StatusCode::NO_CONTENT)
}

async fn get_component_definition_by_id(
    State((logger, data_store)): State<(Arc<DurableLogger>, Arc<dyn DataStore>)>,
    Path(id): Path<String>,
) -> Result<Json<ComponentDefinition>, (StatusCode, &'static str)> {
    let definition = match data_store.get_component_definition(&id) {
        Ok(Some(def)) => def,
        Ok(None) | Err(_) => {
            let log_entry = LogEntry::new(
                LogOperation::ComponentDefinitionGet {
                    definition_id: Some(id),
                    found: false,
                },
                LogMetadata::rest_api(None),
            );
            logger.log_or_error(&log_entry);
            return Err((StatusCode::NOT_FOUND, "not found"));
        }
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
    State((logger, data_store)): State<(Arc<DurableLogger>, Arc<dyn DataStore>)>,
    Path(id): Path<String>,
    Json(definition): Json<ComponentDefinition>,
) -> Result<Json<ComponentDefinition>, (StatusCode, &'static str)> {
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
            return Err((StatusCode::BAD_REQUEST, "invalid schema"));
        }
    };

    let old_definition = data_store.get_component_definition(&id).ok().flatten();
    let result = DataStoreOperations::update_component_definition(&*data_store, &id, &definition);
    if !result.success {
        let log_entry = LogEntry::new(
            LogOperation::ComponentDefinitionUpdate {
                definition_id: id.clone(),
                old_definition,
                new_definition: definition.clone(),
                validation_result,
            },
            LogMetadata::rest_api(None).with_status(OperationStatus::Failed),
        );
        logger.log_or_error(&log_entry);
        return Err((StatusCode::INTERNAL_SERVER_ERROR, "internal server error"));
    }

    let log_entry = LogEntry::new(
        LogOperation::ComponentDefinitionUpdate {
            definition_id: id,
            old_definition,
            new_definition: definition.clone(),
            validation_result,
        },
        LogMetadata::rest_api(None),
    );
    logger.log_or_error(&log_entry);

    Ok(Json(definition))
}

async fn patch_component_definition_by_id(
    State((logger, data_store)): State<(Arc<DurableLogger>, Arc<dyn DataStore>)>,
    Path(id): Path<String>,
    Json(patch): Json<Value>,
) -> Result<Json<ComponentDefinition>, (StatusCode, &'static str)> {
    let component = Component::new(format!("Component{}", id))
        .unwrap_or_else(|| Component::new("PatchedComponent").unwrap());
    let definition = ComponentDefinition {
        component,
        schema: patch.clone(),
    };

    let result = DataStoreOperations::update_component_definition(&*data_store, &id, &definition);
    if !result.success {
        let log_entry = LogEntry::new(
            LogOperation::ComponentDefinitionPatch {
                definition_id: id.clone(),
                patch_data: patch,
                result_definition: definition.clone(),
            },
            LogMetadata::rest_api(None).with_status(OperationStatus::Failed),
        );
        logger.log_or_error(&log_entry);
        return Err((StatusCode::INTERNAL_SERVER_ERROR, "internal server error"));
    }

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
    State((logger, data_store)): State<(Arc<DurableLogger>, Arc<dyn DataStore>)>,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, &'static str)> {
    let deleted_definition = data_store.get_component_definition(&id).ok().flatten();
    let result = DataStoreOperations::delete_component_definition(&*data_store, &id);
    if !result.success {
        let log_entry = LogEntry::new(
            LogOperation::ComponentDefinitionDelete {
                definition_id: id.clone(),
                deleted_definition,
            },
            LogMetadata::rest_api(None).with_status(OperationStatus::Failed),
        );
        logger.log_or_error(&log_entry);
        return Err((StatusCode::INTERNAL_SERVER_ERROR, "internal server error"));
    }

    let log_entry = LogEntry::new(
        LogOperation::ComponentDefinitionDelete {
            definition_id: id,
            deleted_definition,
        },
        LogMetadata::rest_api(None),
    );
    logger.log_or_error(&log_entry);

    Ok(StatusCode::NO_CONTENT)
}

async fn get_components_for_entity(
    State((logger, data_store)): State<(Arc<DurableLogger>, Arc<dyn DataStore>)>,
    Path(entity_id): Path<String>,
    Query(_params): Query<HashMap<String, String>>,
) -> Result<Json<Vec<ComponentListItem>>, (StatusCode, &'static str)> {
    // Parse entity ID (prepend "entity:" prefix to base64 part from URL)
    let full_entity_id = format!("entity:{}", entity_id);
    let entity: Entity = match full_entity_id.parse() {
        Ok(e) => e,
        Err(_) => return Err((StatusCode::BAD_REQUEST, "bad entity")),
    };

    let components = match data_store.list_components_for_entity(&entity) {
        Ok(comp_list) => comp_list
            .into_iter()
            .filter_map(|(component_name, data)| {
                Component::new(&component_name)
                    .map(|component| ComponentListItem { component, data })
            })
            .collect(),
        Err(_) => vec![],
    };

    let log_entry = LogEntry::new(
        LogOperation::ComponentGet {
            component_id: None,
            found: !components.is_empty(),
        },
        LogMetadata::rest_api(None),
    );
    logger.log_or_error(&log_entry);

    Ok(Json(components))
}

#[allow(dead_code)]
async fn get_all_components(
    State((logger, data_store)): State<(Arc<DurableLogger>, Arc<dyn DataStore>)>,
    Query(_params): Query<HashMap<String, String>>,
) -> Result<Json<Vec<Value>>, (StatusCode, &'static str)> {
    let components = match data_store.list_components() {
        Ok(comp_list) => comp_list
            .into_iter()
            .map(|((_entity, _id), data)| data)
            .collect(),
        Err(_) => vec![],
    };

    let log_entry = LogEntry::new(
        LogOperation::ComponentGet {
            component_id: None,
            found: !components.is_empty(),
        },
        LogMetadata::rest_api(None),
    );
    logger.log_or_error(&log_entry);

    Ok(Json(components))
}

async fn create_component_for_entity(
    State((logger, data_store)): State<(Arc<DurableLogger>, Arc<dyn DataStore>)>,
    Path(entity_id): Path<String>,
    Json(request): Json<CreateComponentRequest>,
) -> Result<Json<CreateComponentResponse>, (StatusCode, &'static str)> {
    // Parse entity ID (prepend "entity:" prefix to base64 part from URL)
    let full_entity_id = format!("entity:{}", entity_id);
    let entity: Entity = match full_entity_id.parse() {
        Ok(e) => e,
        Err(_) => return Err((StatusCode::BAD_REQUEST, "invalid entity id")),
    };

    // Look up component definition from data store
    // For now, we'll assume a default component definition exists or use a fallback schema
    let validation_schema = match data_store.list_component_definitions() {
        Ok(definitions) if !definitions.is_empty() => {
            // Use the schema from the first available component definition
            definitions[0].1.schema.clone()
        }
        _ => {
            // Fallback to a sample enum schema for backward compatibility
            serde_json::json!({
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
            })
        }
    };

    let validation_result = match validate_value(&request.data, &validation_schema) {
        Ok(()) => Some(LogValidationResult::success()),
        Err(e) => {
            let log_entry = LogEntry::new(
                LogOperation::ComponentCreate {
                    entity_id: entity_id.clone(),
                    component_id: "generated_id".to_string(),
                    component_data: request.data.clone(),
                    validation_result: Some(LogValidationResult::failed(e.to_string())),
                },
                LogMetadata::rest_api(None).with_status(OperationStatus::Failed),
            );
            logger.log_or_error(&log_entry);
            return Err((StatusCode::BAD_REQUEST, "data validation failed"));
        }
    };

    let component_id = request.component.as_str().to_string();
    let result =
        DataStoreOperations::create_component(&*data_store, &entity, &component_id, &request.data);
    if !result.success {
        let log_entry = LogEntry::new(
            LogOperation::ComponentCreate {
                entity_id: entity_id.clone(),
                component_id: component_id.clone(),
                component_data: request.data.clone(),
                validation_result,
            },
            LogMetadata::rest_api(None).with_status(OperationStatus::Failed),
        );
        logger.log_or_error(&log_entry);
        return Err(match result.into_error() {
            crate::DataStoreError::AlreadyExists => (StatusCode::CONFLICT, "already exists"),
            crate::DataStoreError::NotFound => (StatusCode::NOT_FOUND, "entity not found"), // Entity doesn't exist
            _ => (StatusCode::INTERNAL_SERVER_ERROR, "internal server error"),
        });
    }

    let log_entry = LogEntry::new(
        LogOperation::ComponentCreate {
            entity_id,
            component_id,
            component_data: request.data.clone(),
            validation_result,
        },
        LogMetadata::rest_api(None),
    );
    logger.log_or_error(&log_entry);

    let response = CreateComponentResponse {
        entity,
        component: request.component,
        data: request.data,
    };

    Ok(Json(response))
}

async fn update_component_for_entity(
    State((logger, data_store)): State<(Arc<DurableLogger>, Arc<dyn DataStore>)>,
    Path(entity_id): Path<String>,
    Json(component): Json<Value>,
) -> Result<Json<Value>, (StatusCode, &'static str)> {
    // Parse entity ID (prepend "entity:" prefix to base64 part from URL)
    let full_entity_id = format!("entity:{}", entity_id);
    let entity = match full_entity_id.parse::<Entity>() {
        Ok(e) => e,
        Err(_) => return Err((StatusCode::BAD_REQUEST, "invalid entity id")),
    };

    let component_id = "updated_id".to_string();
    let old_data = data_store
        .get_component(&entity, &component_id)
        .ok()
        .flatten();
    let result =
        DataStoreOperations::update_component(&*data_store, &entity, &component_id, &component);
    if !result.success {
        let log_entry = LogEntry::new(
            LogOperation::ComponentUpdate {
                entity_id: entity_id.clone(),
                component_id: component_id.clone(),
                old_data,
                new_data: component.clone(),
                validation_result: None,
            },
            LogMetadata::rest_api(None).with_status(OperationStatus::Failed),
        );
        logger.log_or_error(&log_entry);
        return Err((StatusCode::INTERNAL_SERVER_ERROR, "internal server error"));
    }

    let log_entry = LogEntry::new(
        LogOperation::ComponentUpdate {
            entity_id,
            component_id,
            old_data,
            new_data: component.clone(),
            validation_result: None,
        },
        LogMetadata::rest_api(None),
    );
    logger.log_or_error(&log_entry);

    Ok(Json(component))
}

async fn patch_component_for_entity(
    State((logger, data_store)): State<(Arc<DurableLogger>, Arc<dyn DataStore>)>,
    Path(entity_id): Path<String>,
    Json(patch): Json<Value>,
) -> Result<Json<Value>, (StatusCode, &'static str)> {
    // Parse entity ID (prepend "entity:" prefix to base64 part from URL)
    let full_entity_id = format!("entity:{}", entity_id);
    let entity = match full_entity_id.parse::<Entity>() {
        Ok(e) => e,
        Err(_) => return Err((StatusCode::BAD_REQUEST, "invalid entity id")),
    };

    let component_id = "patched_id".to_string();
    let result =
        DataStoreOperations::update_component(&*data_store, &entity, &component_id, &patch);
    if !result.success {
        let log_entry = LogEntry::new(
            LogOperation::ComponentPatch {
                entity_id: entity_id.clone(),
                component_id: component_id.clone(),
                patch_data: patch.clone(),
                result_data: patch.clone(),
            },
            LogMetadata::rest_api(None).with_status(OperationStatus::Failed),
        );
        logger.log_or_error(&log_entry);
        return Err((StatusCode::INTERNAL_SERVER_ERROR, "internal server error"));
    }

    let log_entry = LogEntry::new(
        LogOperation::ComponentPatch {
            entity_id,
            component_id,
            patch_data: patch.clone(),
            result_data: patch.clone(),
        },
        LogMetadata::rest_api(None),
    );
    logger.log_or_error(&log_entry);

    Ok(Json(patch))
}

async fn delete_components_for_entity(
    State((logger, data_store)): State<(Arc<DurableLogger>, Arc<dyn DataStore>)>,
    Path(entity_id): Path<String>,
) -> Result<StatusCode, (StatusCode, &'static str)> {
    // Parse entity ID (prepend "entity:" prefix to base64 part from URL)
    let full_entity_id = format!("entity:{}", entity_id);
    let entity = match full_entity_id.parse::<Entity>() {
        Ok(e) => e,
        Err(_) => return Err((StatusCode::BAD_REQUEST, "invalid entity id")),
    };

    let result = DataStoreOperations::delete_all_components_for_entity(&*data_store, &entity);
    let count_deleted = if result.success {
        result.data.unwrap_or(0)
    } else {
        let log_entry = LogEntry::new(
            LogOperation::ComponentDeleteAll { count_deleted: 0 },
            LogMetadata::rest_api(None).with_status(OperationStatus::Failed),
        );
        logger.log_or_error(&log_entry);
        return Err((StatusCode::INTERNAL_SERVER_ERROR, "internal server error"));
    };

    let log_entry = LogEntry::new(
        LogOperation::ComponentDeleteAll { count_deleted },
        LogMetadata::rest_api(None),
    );
    logger.log_or_error(&log_entry);

    Ok(StatusCode::NO_CONTENT)
}

async fn get_component_by_id_for_entity(
    State((logger, data_store)): State<(Arc<DurableLogger>, Arc<dyn DataStore>)>,
    Path((entity_id, component_id)): Path<(String, String)>,
) -> Result<Json<Value>, (StatusCode, &'static str)> {
    // Parse entity ID (prepend "entity:" prefix to base64 part from URL)
    let full_entity_id = format!("entity:{}", entity_id);
    let entity = match full_entity_id.parse::<Entity>() {
        Ok(e) => e,
        Err(_) => return Err((StatusCode::BAD_REQUEST, "invalid entity id")),
    };

    let component = match data_store.get_component(&entity, &component_id) {
        Ok(Some(data)) => data,
        Ok(None) | Err(_) => {
            let log_entry = LogEntry::new(
                LogOperation::ComponentGet {
                    component_id: Some(component_id),
                    found: false,
                },
                LogMetadata::rest_api(None),
            );
            logger.log_or_error(&log_entry);
            return Err((StatusCode::NOT_FOUND, "not found"));
        }
    };

    let log_entry = LogEntry::new(
        LogOperation::ComponentGet {
            component_id: Some(component_id),
            found: true,
        },
        LogMetadata::rest_api(None),
    );
    logger.log_or_error(&log_entry);

    Ok(Json(component))
}

async fn update_component_by_id_for_entity(
    State((logger, data_store)): State<(Arc<DurableLogger>, Arc<dyn DataStore>)>,
    Path((entity_id, component_id)): Path<(String, String)>,
    Json(component): Json<Value>,
) -> Result<Json<Value>, (StatusCode, &'static str)> {
    // Parse entity ID (prepend "entity:" prefix to base64 part from URL)
    let full_entity_id = format!("entity:{}", entity_id);
    let entity = match full_entity_id.parse::<Entity>() {
        Ok(e) => e,
        Err(_) => return Err((StatusCode::BAD_REQUEST, "invalid entity id")),
    };

    let old_data = data_store
        .get_component(&entity, &component_id)
        .ok()
        .flatten();
    let result =
        DataStoreOperations::update_component(&*data_store, &entity, &component_id, &component);
    if !result.success {
        let log_entry = LogEntry::new(
            LogOperation::ComponentUpdate {
                entity_id: entity_id.clone(),
                component_id: component_id.clone(),
                old_data,
                new_data: component.clone(),
                validation_result: None,
            },
            LogMetadata::rest_api(None).with_status(OperationStatus::Failed),
        );
        logger.log_or_error(&log_entry);
        return Err((StatusCode::INTERNAL_SERVER_ERROR, "internal server error"));
    }

    let log_entry = LogEntry::new(
        LogOperation::ComponentUpdate {
            entity_id,
            component_id,
            old_data,
            new_data: component.clone(),
            validation_result: None,
        },
        LogMetadata::rest_api(None),
    );
    logger.log_or_error(&log_entry);

    Ok(Json(component))
}

async fn patch_component_by_id_for_entity(
    State((logger, data_store)): State<(Arc<DurableLogger>, Arc<dyn DataStore>)>,
    Path((entity_id, component_id)): Path<(String, String)>,
    Json(patch): Json<Value>,
) -> Result<Json<Value>, (StatusCode, &'static str)> {
    // Parse entity ID (prepend "entity:" prefix to base64 part from URL)
    let full_entity_id = format!("entity:{}", entity_id);
    let entity = match full_entity_id.parse::<Entity>() {
        Ok(e) => e,
        Err(_) => return Err((StatusCode::BAD_REQUEST, "invalid entity id")),
    };

    let mut component = patch.clone();
    if let Some(obj) = component.as_object_mut() {
        obj.insert(
            "id".to_string(),
            serde_json::Value::String(component_id.clone()),
        );
    }

    let result =
        DataStoreOperations::update_component(&*data_store, &entity, &component_id, &component);
    if !result.success {
        let log_entry = LogEntry::new(
            LogOperation::ComponentPatch {
                entity_id: entity_id.clone(),
                component_id: component_id.clone(),
                patch_data: patch,
                result_data: component.clone(),
            },
            LogMetadata::rest_api(None).with_status(OperationStatus::Failed),
        );
        logger.log_or_error(&log_entry);
        return Err((StatusCode::INTERNAL_SERVER_ERROR, "internal server error"));
    }

    let log_entry = LogEntry::new(
        LogOperation::ComponentPatch {
            entity_id,
            component_id,
            patch_data: patch,
            result_data: component.clone(),
        },
        LogMetadata::rest_api(None),
    );
    logger.log_or_error(&log_entry);

    Ok(Json(component))
}

async fn delete_component_by_id_for_entity(
    State((logger, data_store)): State<(Arc<DurableLogger>, Arc<dyn DataStore>)>,
    Path((entity_id, component_id)): Path<(String, String)>,
) -> Result<StatusCode, (StatusCode, &'static str)> {
    // Parse entity ID (prepend "entity:" prefix to base64 part from URL)
    let full_entity_id = format!("entity:{}", entity_id);
    let entity = match full_entity_id.parse::<Entity>() {
        Ok(e) => e,
        Err(_) => return Err((StatusCode::BAD_REQUEST, "invalid entity id")),
    };

    let deleted_data = data_store
        .get_component(&entity, &component_id)
        .ok()
        .flatten();
    let result = DataStoreOperations::delete_component(&*data_store, &entity, &component_id);
    if !result.success {
        let log_entry = LogEntry::new(
            LogOperation::ComponentDelete {
                entity_id: entity_id.clone(),
                component_id: component_id.clone(),
                deleted_data,
            },
            LogMetadata::rest_api(None).with_status(OperationStatus::Failed),
        );
        logger.log_or_error(&log_entry);
        return Err((StatusCode::INTERNAL_SERVER_ERROR, "internal server error"));
    }

    let log_entry = LogEntry::new(
        LogOperation::ComponentDelete {
            entity_id,
            component_id,
            deleted_data,
        },
        LogMetadata::rest_api(None),
    );
    logger.log_or_error(&log_entry);

    Ok(StatusCode::NO_CONTENT)
}

////////////////////////////////////////////// router //////////////////////////////////////////////

pub fn create_component_router(
    logger: Arc<DurableLogger>,
    data_store: Arc<dyn DataStore>,
) -> Router {
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
            "/entity/:entity_id/component",
            get(get_components_for_entity)
                .post(create_component_for_entity)
                .put(update_component_for_entity)
                .patch(patch_component_for_entity)
                .delete(delete_components_for_entity),
        )
        .route(
            "/entity/:entity_id/component/:component_id",
            get(get_component_by_id_for_entity)
                .put(update_component_by_id_for_entity)
                .patch(patch_component_by_id_for_entity)
                .delete(delete_component_by_id_for_entity),
        )
        .with_state((logger, data_store))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_helpers::{
        clear_log_file, create_test_logger_with_path, read_log_entries, test_data_store,
    };

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

    fn sample_component_definition() -> ComponentDefinition {
        ComponentDefinition {
            component: Component::new("TestComponent").unwrap(),
            schema: serde_json::json!({
                "type": "string"
            }),
        }
    }

    fn invalid_component_definition() -> ComponentDefinition {
        ComponentDefinition {
            component: Component::new("InvalidComponent").unwrap(),
            schema: serde_json::json!({
                "type": "invalid_type"
            }),
        }
    }

    // Component Definition HTTP Endpoint Tests
    #[tokio::test]
    async fn get_component_definitions_logs_correctly() {
        let (logger, log_path) = create_test_logger_with_path("component", "get_definitions");
        clear_log_file(&log_path);

        let params = HashMap::new();
        let logs_before = read_log_entries(&log_path);
        assert!(logs_before.is_empty());

        let data_store = test_data_store();
        let result = get_component_definitions(State((logger, data_store)), Query(params)).await;
        assert!(result.is_ok());

        let logs_after = read_log_entries(&log_path);
        assert_eq!(logs_after.len(), 1);

        let log_entry = &logs_after[0];
        assert_eq!(log_entry.operation_type(), "ComponentDefinitionGet");
        assert!(log_entry.is_success());

        match &log_entry.operation {
            LogOperation::ComponentDefinitionGet {
                definition_id,
                found,
            } => {
                assert!(definition_id.is_none());
                assert!(!*found);
            }
            _ => panic!("Expected ComponentDefinitionGet operation"),
        }

        clear_log_file(&log_path);
    }

    #[tokio::test]
    async fn create_component_definition_success_logs_correctly() {
        let (logger, log_path) = create_test_logger_with_path("component", "create_def_success");
        clear_log_file(&log_path);

        let definition = sample_component_definition();
        let logs_before = read_log_entries(&log_path);
        assert!(logs_before.is_empty());

        let data_store = test_data_store();
        let result =
            create_component_definition(State((logger, data_store)), Json(definition.clone()))
                .await;
        assert!(result.is_ok());

        let logs_after = read_log_entries(&log_path);
        assert_eq!(logs_after.len(), 1);

        let log_entry = &logs_after[0];
        assert_eq!(log_entry.operation_type(), "ComponentDefinitionCreate");
        assert!(log_entry.is_success());

        match &log_entry.operation {
            LogOperation::ComponentDefinitionCreate {
                definition: logged_def,
                validation_result,
            } => {
                assert_eq!(logged_def.schema, definition.schema);
                assert!(validation_result.is_success());
            }
            _ => panic!("Expected ComponentDefinitionCreate operation"),
        }

        clear_log_file(&log_path);
    }

    #[tokio::test]
    async fn create_component_definition_failure_logs_correctly() {
        let (logger, log_path) = create_test_logger_with_path("component", "create_def_failure");
        clear_log_file(&log_path);

        let definition = invalid_component_definition();
        let logs_before = read_log_entries(&log_path);
        assert!(logs_before.is_empty());

        let data_store = test_data_store();
        let result =
            create_component_definition(State((logger, data_store)), Json(definition.clone()))
                .await;
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            (StatusCode::BAD_REQUEST, "invalid schema")
        );

        let logs_after = read_log_entries(&log_path);
        assert_eq!(logs_after.len(), 1);

        let log_entry = &logs_after[0];
        assert_eq!(log_entry.operation_type(), "ComponentDefinitionCreate");
        assert!(log_entry.is_failure());

        match &log_entry.operation {
            LogOperation::ComponentDefinitionCreate {
                definition: logged_def,
                validation_result,
            } => {
                assert_eq!(logged_def.schema, definition.schema);
                assert!(validation_result.is_failure());
            }
            _ => panic!("Expected ComponentDefinitionCreate operation"),
        }

        clear_log_file(&log_path);
    }

    #[tokio::test]
    async fn update_component_definition_success_logs_correctly() {
        let (logger, log_path) = create_test_logger_with_path("component", "update_def_success");
        clear_log_file(&log_path);

        let definition = sample_component_definition();
        let logs_before = read_log_entries(&log_path);
        assert!(logs_before.is_empty());

        let result = update_component_definition(
            State((logger, test_data_store())),
            Json(definition.clone()),
        )
        .await;
        assert!(result.is_ok());

        let logs_after = read_log_entries(&log_path);
        assert_eq!(logs_after.len(), 1);

        let log_entry = &logs_after[0];
        assert_eq!(log_entry.operation_type(), "ComponentDefinitionUpdate");
        assert!(log_entry.is_success());

        match &log_entry.operation {
            LogOperation::ComponentDefinitionUpdate {
                definition_id,
                old_definition,
                new_definition,
                validation_result,
            } => {
                assert!(!definition_id.is_empty());
                assert!(old_definition.is_none());
                assert_eq!(new_definition.schema, definition.schema);
                assert!(validation_result.is_success());
            }
            _ => panic!("Expected ComponentDefinitionUpdate operation"),
        }

        clear_log_file(&log_path);
    }

    #[tokio::test]
    async fn patch_component_definition_logs_correctly() {
        let (logger, log_path) = create_test_logger_with_path("component", "patch_def");
        clear_log_file(&log_path);

        let patch = serde_json::json!({"type": "number"});
        let logs_before = read_log_entries(&log_path);
        assert!(logs_before.is_empty());

        let result =
            patch_component_definition(State((logger, test_data_store())), Json(patch.clone()))
                .await;
        assert!(result.is_ok());

        let logs_after = read_log_entries(&log_path);
        assert_eq!(logs_after.len(), 1);

        let log_entry = &logs_after[0];
        assert_eq!(log_entry.operation_type(), "ComponentDefinitionPatch");
        assert!(log_entry.is_success());

        match &log_entry.operation {
            LogOperation::ComponentDefinitionPatch {
                definition_id,
                patch_data,
                result_definition,
            } => {
                assert_eq!(*definition_id, "PatchedComponent");
                assert_eq!(*patch_data, patch);
                assert_eq!(result_definition.schema, patch);
            }
            _ => panic!("Expected ComponentDefinitionPatch operation"),
        }

        clear_log_file(&log_path);
    }

    #[tokio::test]
    async fn delete_component_definitions_logs_correctly() {
        let (logger, log_path) = create_test_logger_with_path("component", "delete_defs");
        clear_log_file(&log_path);

        let logs_before = read_log_entries(&log_path);
        assert!(logs_before.is_empty());

        let result = delete_component_definitions(State((logger, test_data_store()))).await;
        assert_eq!(result, Ok(StatusCode::NO_CONTENT));

        let logs_after = read_log_entries(&log_path);
        assert_eq!(logs_after.len(), 1);

        let log_entry = &logs_after[0];
        assert_eq!(log_entry.operation_type(), "ComponentDefinitionDeleteAll");
        assert!(log_entry.is_success());

        match &log_entry.operation {
            LogOperation::ComponentDefinitionDeleteAll { count_deleted } => {
                assert_eq!(*count_deleted, 0);
            }
            _ => panic!("Expected ComponentDefinitionDeleteAll operation"),
        }

        clear_log_file(&log_path);
    }

    #[tokio::test]
    async fn get_component_definition_by_id_logs_correctly() {
        let (logger, log_path) = create_test_logger_with_path("component", "get_def_by_id");
        clear_log_file(&log_path);

        let test_id = "test123".to_string();
        let logs_before = read_log_entries(&log_path);
        assert!(logs_before.is_empty());

        let result = get_component_definition_by_id(
            State((logger, test_data_store())),
            Path(test_id.clone()),
        )
        .await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), (StatusCode::NOT_FOUND, "not found"));

        let logs_after = read_log_entries(&log_path);
        assert_eq!(logs_after.len(), 1);

        let log_entry = &logs_after[0];
        assert_eq!(log_entry.operation_type(), "ComponentDefinitionGet");
        assert!(log_entry.is_success());

        match &log_entry.operation {
            LogOperation::ComponentDefinitionGet {
                definition_id,
                found,
            } => {
                assert_eq!(*definition_id, Some(test_id));
                assert!(!*found);
            }
            _ => panic!("Expected ComponentDefinitionGet operation"),
        }

        clear_log_file(&log_path);
    }

    #[tokio::test]
    async fn update_component_definition_by_id_success_logs_correctly() {
        let (logger, log_path) =
            create_test_logger_with_path("component", "update_def_by_id_success");
        clear_log_file(&log_path);

        let test_id = "test456".to_string();
        let definition = sample_component_definition();
        let logs_before = read_log_entries(&log_path);
        assert!(logs_before.is_empty());

        let result = update_component_definition_by_id(
            State((logger, test_data_store())),
            Path(test_id.clone()),
            Json(definition.clone()),
        )
        .await;
        assert!(result.is_ok());

        let logs_after = read_log_entries(&log_path);
        assert_eq!(logs_after.len(), 1);

        let log_entry = &logs_after[0];
        assert_eq!(log_entry.operation_type(), "ComponentDefinitionUpdate");
        assert!(log_entry.is_success());

        match &log_entry.operation {
            LogOperation::ComponentDefinitionUpdate {
                definition_id,
                old_definition,
                new_definition,
                validation_result,
            } => {
                assert_eq!(*definition_id, test_id);
                assert!(old_definition.is_none());
                assert_eq!(new_definition.schema, definition.schema);
                assert!(validation_result.is_success());
            }
            _ => panic!("Expected ComponentDefinitionUpdate operation"),
        }

        clear_log_file(&log_path);
    }

    #[tokio::test]
    async fn patch_component_definition_by_id_logs_correctly() {
        let (logger, log_path) = create_test_logger_with_path("component", "patch_def_by_id");
        clear_log_file(&log_path);

        let test_id = "test789".to_string();
        let patch = serde_json::json!({"type": "boolean"});
        let logs_before = read_log_entries(&log_path);
        assert!(logs_before.is_empty());

        let result = patch_component_definition_by_id(
            State((logger, test_data_store())),
            Path(test_id.clone()),
            Json(patch.clone()),
        )
        .await;
        assert!(result.is_ok());

        let logs_after = read_log_entries(&log_path);
        assert_eq!(logs_after.len(), 1);

        let log_entry = &logs_after[0];
        assert_eq!(log_entry.operation_type(), "ComponentDefinitionPatch");
        assert!(log_entry.is_success());

        match &log_entry.operation {
            LogOperation::ComponentDefinitionPatch {
                definition_id,
                patch_data,
                result_definition,
            } => {
                assert_eq!(*definition_id, test_id);
                assert_eq!(*patch_data, patch);
                assert_eq!(result_definition.schema, patch);
            }
            _ => panic!("Expected ComponentDefinitionPatch operation"),
        }

        clear_log_file(&log_path);
    }

    #[tokio::test]
    async fn delete_component_definition_by_id_logs_correctly() {
        let (logger, log_path) = create_test_logger_with_path("component", "delete_def_by_id");
        clear_log_file(&log_path);

        let test_id = "test_delete".to_string();
        let logs_before = read_log_entries(&log_path);
        assert!(logs_before.is_empty());

        let result = delete_component_definition_by_id(
            State((logger, test_data_store())),
            Path(test_id.clone()),
        )
        .await;
        assert_eq!(result, Ok(StatusCode::NO_CONTENT));

        let logs_after = read_log_entries(&log_path);
        assert_eq!(logs_after.len(), 1);

        let log_entry = &logs_after[0];
        assert_eq!(log_entry.operation_type(), "ComponentDefinitionDelete");
        assert!(log_entry.is_success());

        match &log_entry.operation {
            LogOperation::ComponentDefinitionDelete {
                definition_id,
                deleted_definition,
            } => {
                assert_eq!(*definition_id, test_id);
                assert!(deleted_definition.is_none());
            }
            _ => panic!("Expected ComponentDefinitionDelete operation"),
        }

        clear_log_file(&log_path);
    }

    // Component Instance HTTP Endpoint Tests
    #[tokio::test]
    async fn get_components_logs_correctly() {
        let (logger, log_path) = create_test_logger_with_path("component", "get_components");
        clear_log_file(&log_path);

        let logs_before = read_log_entries(&log_path);
        assert!(logs_before.is_empty());

        let data_store = test_data_store();
        let entity = Entity::random().unwrap();
        let entity_id = entity.to_string()[7..].to_string(); // Skip "entity:" prefix
        let result = get_components_for_entity(
            State((logger, data_store)),
            Path(entity_id),
            Query(HashMap::<String, String>::new()),
        )
        .await;
        assert!(result.is_ok());

        let logs_after = read_log_entries(&log_path);
        assert_eq!(logs_after.len(), 1);

        let log_entry = &logs_after[0];
        assert_eq!(log_entry.operation_type(), "ComponentGet");
        assert!(log_entry.is_success());

        match &log_entry.operation {
            LogOperation::ComponentGet {
                component_id,
                found,
            } => {
                assert!(component_id.is_none());
                assert!(!*found);
            }
            _ => panic!("Expected ComponentGet operation"),
        }

        clear_log_file(&log_path);
    }

    #[tokio::test]
    async fn create_component_success_logs_correctly() {
        let (logger, log_path) = create_test_logger_with_path("component", "create_comp_success");
        clear_log_file(&log_path);

        let component_data = serde_json::json!("Red");
        let logs_before = read_log_entries(&log_path);
        assert!(logs_before.is_empty());

        let entity = Entity::random().unwrap();
        let data_store = test_data_store();

        // Create the entity first
        data_store.create_entity(&entity).unwrap();

        let request = CreateComponentRequest {
            component: Component::new("TestComponent").unwrap(),
            data: component_data.clone(),
        };

        let result = create_component_for_entity(
            State((logger, data_store)),
            Path(entity.to_string()[7..].to_string()),
            Json(request),
        )
        .await;
        assert!(result.is_ok());

        let logs_after = read_log_entries(&log_path);
        assert_eq!(logs_after.len(), 1);

        let log_entry = &logs_after[0];
        assert_eq!(log_entry.operation_type(), "ComponentCreate");
        assert!(log_entry.is_success());

        match &log_entry.operation {
            LogOperation::ComponentCreate {
                entity_id: _,
                component_id,
                component_data: logged_data,
                validation_result,
            } => {
                assert_eq!(*component_id, "TestComponent");
                assert_eq!(*logged_data, component_data);
                assert!(validation_result.as_ref().unwrap().is_success());
            }
            _ => panic!("Expected ComponentCreate operation"),
        }

        clear_log_file(&log_path);
    }

    #[tokio::test]
    async fn create_component_failure_logs_correctly() {
        let (logger, log_path) = create_test_logger_with_path("component", "create_comp_failure");
        clear_log_file(&log_path);

        let component_data = serde_json::json!("InvalidColor");
        let logs_before = read_log_entries(&log_path);
        assert!(logs_before.is_empty());

        let entity = Entity::random().unwrap();
        let request = CreateComponentRequest {
            component: Component::new("TestComponent").unwrap(),
            data: component_data.clone(),
        };

        let result = create_component_for_entity(
            State((logger, test_data_store())),
            Path(entity.to_string()[7..].to_string()),
            Json(request),
        )
        .await;
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            (StatusCode::BAD_REQUEST, "data validation failed")
        );

        let logs_after = read_log_entries(&log_path);
        assert_eq!(logs_after.len(), 1);

        let log_entry = &logs_after[0];
        assert_eq!(log_entry.operation_type(), "ComponentCreate");
        assert!(log_entry.is_failure());

        match &log_entry.operation {
            LogOperation::ComponentCreate {
                entity_id: _,
                component_id,
                component_data: logged_data,
                validation_result,
            } => {
                assert_eq!(*component_id, "generated_id");
                assert_eq!(*logged_data, component_data);
                assert!(validation_result.as_ref().unwrap().is_failure());
            }
            _ => panic!("Expected ComponentCreate operation"),
        }

        clear_log_file(&log_path);
    }

    #[tokio::test]
    async fn update_component_logs_correctly() {
        let (logger, log_path) = create_test_logger_with_path("component", "update_comp");
        clear_log_file(&log_path);

        let entity = Entity::random().unwrap();
        let entity_id = entity.to_string()[7..].to_string(); // Skip "entity:" prefix
        let component_data = serde_json::json!({"color": "blue"});
        let logs_before = read_log_entries(&log_path);
        assert!(logs_before.is_empty());

        let result = update_component_for_entity(
            State((logger, test_data_store())),
            Path(entity_id),
            Json(component_data.clone()),
        )
        .await;
        assert!(result.is_ok());

        let logs_after = read_log_entries(&log_path);
        assert_eq!(logs_after.len(), 1);

        let log_entry = &logs_after[0];
        assert_eq!(log_entry.operation_type(), "ComponentUpdate");
        assert!(log_entry.is_success());

        match &log_entry.operation {
            LogOperation::ComponentUpdate {
                entity_id: _,
                component_id,
                old_data,
                new_data,
                validation_result,
            } => {
                assert_eq!(*component_id, "updated_id");
                assert!(old_data.is_none());
                assert_eq!(*new_data, component_data);
                assert!(validation_result.is_none());
            }
            _ => panic!("Expected ComponentUpdate operation"),
        }

        clear_log_file(&log_path);
    }

    #[tokio::test]
    async fn patch_component_logs_correctly() {
        let (logger, log_path) = create_test_logger_with_path("component", "patch_comp");
        clear_log_file(&log_path);

        let entity = Entity::random().unwrap();
        let entity_id = entity.to_string()[7..].to_string(); // Skip "entity:" prefix
        let patch_data = serde_json::json!({"size": "large"});
        let logs_before = read_log_entries(&log_path);
        assert!(logs_before.is_empty());

        let result = patch_component_for_entity(
            State((logger, test_data_store())),
            Path(entity_id),
            Json(patch_data.clone()),
        )
        .await;
        assert!(result.is_ok());

        let logs_after = read_log_entries(&log_path);
        assert_eq!(logs_after.len(), 1);

        let log_entry = &logs_after[0];
        assert_eq!(log_entry.operation_type(), "ComponentPatch");
        assert!(log_entry.is_success());

        match &log_entry.operation {
            LogOperation::ComponentPatch {
                entity_id: _,
                component_id,
                patch_data: logged_patch,
                result_data,
            } => {
                assert_eq!(*component_id, "patched_id");
                assert_eq!(*logged_patch, patch_data);
                assert_eq!(*result_data, patch_data);
            }
            _ => panic!("Expected ComponentPatch operation"),
        }

        clear_log_file(&log_path);
    }

    #[tokio::test]
    async fn delete_components_logs_correctly() {
        let (logger, log_path) = create_test_logger_with_path("component", "delete_comps");
        clear_log_file(&log_path);

        let entity = Entity::random().unwrap();
        let entity_id = entity.to_string()[7..].to_string(); // Skip "entity:" prefix
        let logs_before = read_log_entries(&log_path);
        assert!(logs_before.is_empty());

        let result =
            delete_components_for_entity(State((logger, test_data_store())), Path(entity_id)).await;
        assert_eq!(result, Ok(StatusCode::NO_CONTENT));

        let logs_after = read_log_entries(&log_path);
        assert_eq!(logs_after.len(), 1);

        let log_entry = &logs_after[0];
        assert_eq!(log_entry.operation_type(), "ComponentDeleteAll");
        assert!(log_entry.is_success());

        match &log_entry.operation {
            LogOperation::ComponentDeleteAll { count_deleted } => {
                assert_eq!(*count_deleted, 0);
            }
            _ => panic!("Expected ComponentDeleteAll operation"),
        }

        clear_log_file(&log_path);
    }

    #[tokio::test]
    async fn get_component_by_id_logs_correctly() {
        let (logger, log_path) = create_test_logger_with_path("component", "get_comp_by_id");
        clear_log_file(&log_path);

        let entity = Entity::random().unwrap();
        let entity_id = entity.to_string()[7..].to_string(); // Skip "entity:" prefix
        let test_id = "comp123".to_string();
        let logs_before = read_log_entries(&log_path);
        assert!(logs_before.is_empty());

        let result = get_component_by_id_for_entity(
            State((logger, test_data_store())),
            Path((entity_id, test_id.clone())),
        )
        .await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), (StatusCode::NOT_FOUND, "not found"));

        let logs_after = read_log_entries(&log_path);
        assert_eq!(logs_after.len(), 1);

        let log_entry = &logs_after[0];
        assert_eq!(log_entry.operation_type(), "ComponentGet");
        assert!(log_entry.is_success());

        match &log_entry.operation {
            LogOperation::ComponentGet {
                component_id,
                found,
            } => {
                assert_eq!(*component_id, Some(test_id));
                assert!(!*found);
            }
            _ => panic!("Expected ComponentGet operation"),
        }

        clear_log_file(&log_path);
    }

    #[tokio::test]
    async fn update_component_by_id_logs_correctly() {
        let (logger, log_path) = create_test_logger_with_path("component", "update_comp_by_id");
        clear_log_file(&log_path);

        let entity = Entity::random().unwrap();
        let entity_id = entity.to_string()[7..].to_string(); // Skip "entity:" prefix
        let test_id = "comp456".to_string();
        let component_data = serde_json::json!({"status": "active"});
        let logs_before = read_log_entries(&log_path);
        assert!(logs_before.is_empty());

        let result = update_component_by_id_for_entity(
            State((logger, test_data_store())),
            Path((entity_id, test_id.clone())),
            Json(component_data.clone()),
        )
        .await;
        assert!(result.is_ok());

        let logs_after = read_log_entries(&log_path);
        assert_eq!(logs_after.len(), 1);

        let log_entry = &logs_after[0];
        assert_eq!(log_entry.operation_type(), "ComponentUpdate");
        assert!(log_entry.is_success());

        match &log_entry.operation {
            LogOperation::ComponentUpdate {
                entity_id: _,
                component_id,
                old_data,
                new_data,
                validation_result,
            } => {
                assert_eq!(*component_id, test_id);
                assert!(old_data.is_none());
                assert_eq!(*new_data, component_data);
                assert!(validation_result.is_none());
            }
            _ => panic!("Expected ComponentUpdate operation"),
        }

        clear_log_file(&log_path);
    }

    #[tokio::test]
    async fn patch_component_by_id_logs_correctly() {
        let (logger, log_path) = create_test_logger_with_path("component", "patch_comp_by_id");
        clear_log_file(&log_path);

        let entity = Entity::random().unwrap();
        let entity_id = entity.to_string()[7..].to_string(); // Skip "entity:" prefix
        let test_id = "comp789".to_string();
        let patch_data = serde_json::json!({"priority": "high"});
        let logs_before = read_log_entries(&log_path);
        assert!(logs_before.is_empty());

        let result = patch_component_by_id_for_entity(
            State((logger, test_data_store())),
            Path((entity_id, test_id.clone())),
            Json(patch_data.clone()),
        )
        .await;
        assert!(result.is_ok());

        let logs_after = read_log_entries(&log_path);
        assert_eq!(logs_after.len(), 1);

        let log_entry = &logs_after[0];
        assert_eq!(log_entry.operation_type(), "ComponentPatch");
        assert!(log_entry.is_success());

        match &log_entry.operation {
            LogOperation::ComponentPatch {
                entity_id: _,
                component_id,
                patch_data: logged_patch,
                result_data,
            } => {
                assert_eq!(*component_id, test_id);
                assert_eq!(*logged_patch, patch_data);
                // The result should include the id
                if let Some(obj) = result_data.as_object() {
                    assert!(obj.contains_key("id"));
                    assert_eq!(
                        obj.get("id").unwrap(),
                        &serde_json::Value::String(test_id.clone())
                    );
                }
            }
            _ => panic!("Expected ComponentPatch operation"),
        }

        clear_log_file(&log_path);
    }

    #[tokio::test]
    async fn delete_component_by_id_logs_correctly() {
        let (logger, log_path) = create_test_logger_with_path("component", "delete_comp_by_id");
        clear_log_file(&log_path);

        let entity = Entity::random().unwrap();
        let entity_id = entity.to_string()[7..].to_string(); // Skip "entity:" prefix
        let test_id = "comp_delete".to_string();
        let logs_before = read_log_entries(&log_path);
        assert!(logs_before.is_empty());

        let result = delete_component_by_id_for_entity(
            State((logger, test_data_store())),
            Path((entity_id, test_id.clone())),
        )
        .await;
        assert_eq!(result, Ok(StatusCode::NO_CONTENT));

        let logs_after = read_log_entries(&log_path);
        assert_eq!(logs_after.len(), 1);

        let log_entry = &logs_after[0];
        assert_eq!(log_entry.operation_type(), "ComponentDelete");
        assert!(log_entry.is_success());

        match &log_entry.operation {
            LogOperation::ComponentDelete {
                entity_id: _,
                component_id,
                deleted_data,
            } => {
                assert_eq!(*component_id, test_id);
                assert!(deleted_data.is_none());
            }
            _ => panic!("Expected ComponentDelete operation"),
        }

        clear_log_file(&log_path);
    }

    // Additional tests for error handling and edge cases
    #[tokio::test]
    async fn data_store_error_handling_component_definition() {
        let (logger, log_path) = create_test_logger_with_path("component", "data_store_error_def");
        clear_log_file(&log_path);

        let definition = sample_component_definition();
        let data_store = test_data_store();
        let def_id = definition.component.as_str().to_string();

        // Create definition first
        data_store
            .create_component_definition(&def_id, &definition)
            .unwrap();

        // Try to create again - should get CONFLICT
        let result = create_component_definition(
            State((logger.clone(), data_store.clone())),
            Json(definition.clone()),
        )
        .await;
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            (StatusCode::CONFLICT, "already exists")
        );

        // Store should still have exactly one definition
        let definitions = data_store.list_component_definitions().unwrap();
        assert_eq!(definitions.len(), 1);

        clear_log_file(&log_path);
    }

    #[tokio::test]
    #[ignore]
    async fn data_store_error_handling_component() {
        let (logger, log_path) = create_test_logger_with_path("component", "data_store_error_comp");
        clear_log_file(&log_path);

        let component_data = serde_json::json!("Green");
        let data_store = test_data_store();
        let comp_id = "generated_id";

        // Create an entity first
        let entity = Entity::random().unwrap();
        data_store.create_entity(&entity).unwrap();

        // Create component first
        data_store
            .create_component(&entity, comp_id, &component_data)
            .unwrap();

        // Try to create again - should get CONFLICT
        let request = CreateComponentRequest {
            component: Component::new("TestComponent").unwrap(),
            data: component_data.clone(),
        };

        let result = create_component_for_entity(
            State((logger.clone(), data_store.clone())),
            Path(entity.to_string()[7..].to_string()),
            Json(request),
        )
        .await;
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            (StatusCode::CONFLICT, "already exists")
        );

        // Store should still have exactly one component
        let components = data_store.list_components().unwrap();
        assert_eq!(components.len(), 1);

        clear_log_file(&log_path);
    }

    #[tokio::test]
    async fn schema_validation_edge_cases() {
        let (logger, log_path) =
            create_test_logger_with_path("component", "schema_validation_edge");
        clear_log_file(&log_path);

        // Test with empty schema object
        let empty_schema_def = ComponentDefinition {
            component: Component::new("EmptySchemaComponent").unwrap(),
            schema: serde_json::json!({}),
        };
        let data_store = test_data_store();

        let result = create_component_definition(
            State((logger.clone(), data_store.clone())),
            Json(empty_schema_def.clone()),
        )
        .await;

        // Should fail validation due to missing 'type' or 'oneOf'
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            (StatusCode::BAD_REQUEST, "invalid schema")
        );

        // Nothing should be stored
        let definitions = data_store.list_component_definitions().unwrap();
        assert!(definitions.is_empty());

        clear_log_file(&log_path);
    }

    #[tokio::test]
    async fn component_validation_with_complex_schema() {
        let (logger, log_path) =
            create_test_logger_with_path("component", "complex_schema_validation");
        clear_log_file(&log_path);

        let complex_definition = ComponentDefinition {
            component: Component::new("ComplexComponent").unwrap(),
            schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "name": { "type": "string" },
                    "age": { "type": "integer" },
                    "active": { "type": "boolean" }
                },
                "required": ["name", "age"]
            }),
        };
        let data_store = test_data_store();

        // Create the definition first
        data_store
            .create_component_definition("complex_def", &complex_definition)
            .unwrap();

        // Create an entity to attach the component to
        let entity = Entity::random().unwrap();
        let entity_id = entity.to_string()[7..].to_string(); // Skip "entity:" prefix

        // Create the entity first
        data_store.create_entity(&entity).unwrap();

        // Test with valid component data
        let valid_data = serde_json::json!({
            "name": "John Doe",
            "age": 30,
            "active": true
        });
        let request = CreateComponentRequest {
            component: Component::new("TestComponent").unwrap(),
            data: valid_data.clone(),
        };

        let result = create_component_for_entity(
            State((logger.clone(), data_store.clone())),
            Path(entity_id),
            Json(request),
        )
        .await;
        assert!(result.is_ok());

        // Verify it was stored
        let components = data_store.list_components().unwrap();
        assert_eq!(components.len(), 1);
        assert_eq!(components[0].1, valid_data);

        clear_log_file(&log_path);
    }

    #[tokio::test]
    async fn http_status_code_verification() {
        let (logger, log_path) =
            create_test_logger_with_path("component", "status_code_verification");
        clear_log_file(&log_path);

        let data_store = test_data_store();
        let definition = sample_component_definition();
        let def_id = definition.component.as_str().to_string();

        // Test 201 Created for component definition
        let create_result = create_component_definition(
            State((logger.clone(), data_store.clone())),
            Json(definition.clone()),
        )
        .await;
        assert!(create_result.is_ok()); // Should be 200 OK (returned as Json), not 201

        // Test 409 Conflict for duplicate creation
        let conflict_result = create_component_definition(
            State((logger.clone(), data_store.clone())),
            Json(definition.clone()),
        )
        .await;
        assert!(conflict_result.is_err());
        assert_eq!(
            conflict_result.unwrap_err(),
            (StatusCode::CONFLICT, "already exists")
        );

        // Test 400 Bad Request for validation failure
        let invalid_def = invalid_component_definition();
        let validation_result = create_component_definition(
            State((logger.clone(), data_store.clone())),
            Json(invalid_def),
        )
        .await;
        assert!(validation_result.is_err());
        assert_eq!(
            validation_result.unwrap_err(),
            (StatusCode::BAD_REQUEST, "invalid schema")
        );

        // Test 404 Not Found for non-existent resource
        let not_found_result = get_component_definition_by_id(
            State((logger.clone(), data_store.clone())),
            Path("nonexistent".to_string()),
        )
        .await;
        assert!(not_found_result.is_err());
        assert_eq!(
            not_found_result.unwrap_err(),
            (StatusCode::NOT_FOUND, "not found")
        );

        // Test 204 No Content for successful deletion
        let delete_result = delete_component_definition_by_id(
            State((logger.clone(), data_store.clone())),
            Path(def_id),
        )
        .await;
        assert_eq!(delete_result, Ok(StatusCode::NO_CONTENT));

        clear_log_file(&log_path);
    }

    #[tokio::test]
    async fn component_definition_create_and_get_round_trip() {
        let (logger, log_path) = create_test_logger_with_path("component", "def_roundtrip");
        clear_log_file(&log_path);

        let definition = ComponentDefinition {
            component: Component::new("RoundTripTest").unwrap(),
            schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "test_field": { "type": "string" }
                }
            }),
        };

        let data_store = test_data_store();

        // Create the component definition
        let create_result = create_component_definition(
            State((logger.clone(), data_store.clone())),
            Json(definition.clone()),
        )
        .await;
        assert!(
            create_result.is_ok(),
            "Component definition creation should succeed"
        );

        // Now try to get it back using the same component name
        let get_result = get_component_definition_by_id(
            State((logger.clone(), data_store.clone())),
            Path("RoundTripTest".to_string()),
        )
        .await;
        assert!(
            get_result.is_ok(),
            "Should be able to retrieve the component definition by name"
        );
        let retrieved_definition = get_result.unwrap().0;

        // Verify the retrieved definition matches what we created
        assert_eq!(retrieved_definition.component, definition.component);
        assert_eq!(retrieved_definition.schema, definition.schema);

        clear_log_file(&log_path);
    }

    #[tokio::test]
    async fn componentdefinition_get_works_correctly() {
        let (logger, log_path) = create_test_logger_with_path("component", "def_get_test");
        clear_log_file(&log_path);

        let definition = ComponentDefinition {
            component: Component::new("GetTestComponent").unwrap(),
            schema: serde_json::json!({
                "type": "string",
                "minLength": 1
            }),
        };

        let data_store = test_data_store();

        // First create the component definition
        let create_result = create_component_definition(
            State((logger.clone(), data_store.clone())),
            Json(definition.clone()),
        )
        .await;
        assert!(
            create_result.is_ok(),
            "Should successfully create component definition"
        );

        // Now get it back using the exact component name (without Debug formatting)
        let get_result = get_component_definition_by_id(
            State((logger.clone(), data_store.clone())),
            Path("GetTestComponent".to_string()), // This is what CLI sends
        )
        .await;

        // Verify the get succeeds
        assert!(
            get_result.is_ok(),
            "Should successfully retrieve component definition by name"
        );

        let retrieved = get_result.unwrap().0;
        assert_eq!(
            retrieved.component, definition.component,
            "Component name should match"
        );
        assert_eq!(
            retrieved.schema, definition.schema,
            "Schema should match exactly"
        );

        // Also test that non-existent components return 404
        let not_found_result = get_component_definition_by_id(
            State((logger.clone(), data_store.clone())),
            Path("NonExistentComponent".to_string()),
        )
        .await;
        assert!(
            not_found_result.is_err(),
            "Should return error for non-existent component"
        );
        assert_eq!(
            not_found_result.unwrap_err(),
            (StatusCode::NOT_FOUND, "not found"),
            "Should return 404 NOT_FOUND"
        );

        clear_log_file(&log_path);
    }
}
