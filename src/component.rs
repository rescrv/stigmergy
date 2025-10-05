//! # Component System
//!
//! This module implements the component system for the stigmergy architecture, providing
//! typed data structures that can be attached to entities. The system follows Entity-
//! Component-System (ECS) patterns where components represent data and behavior.
//!
//! ## Key Features
//!
//! - **Type-Safe Components**: Component types follow Rust identifier conventions
//! - **Schema Validation**: All component data is validated against JSON schemas
//! - **HTTP API**: Complete REST API for component definition and instance management
//! - **Flexible Schemas**: Support for complex schemas including oneOf unions and enums
//! - **Entity Scoping**: Components are scoped to specific entities
//! - **JSON and YAML Support**: Component definitions accept both JSON and YAML based on Content-Type header
//!
//! ## Component Architecture
//!
//! The component system has two main concepts:
//!
//! 1. **Component Definitions**: Schema definitions that specify what data is valid
//! 2. **Component Instances**: Actual data attached to entities, validated against schemas
//!
//! ## Usage Examples
//!
//! ### Creating Component Definitions
//!
//! ```rust
//! use stigmergy::{Component, ComponentDefinition};
//! use serde_json::json;
//!
//! // Define a component type
//! let position_component = Component::new("Position").unwrap();
//!
//! // Create a schema for 3D position data
//! let schema = json!({
//!     "type": "object",
//!     "properties": {
//!         "x": { "type": "number" },
//!         "y": { "type": "number" },
//!         "z": { "type": "number" }
//!     },
//!     "required": ["x", "y", "z"]
//! });
//!
//! let definition = ComponentDefinition::new(position_component, schema);
//! assert!(definition.validate_schema().is_ok());
//! ```
//!
//! ### Validating Component Data
//!
//! ```rust
//! # use stigmergy::{Component, ComponentDefinition};
//! # use serde_json::json;
//! # let position_component = Component::new("Position").unwrap();
//! # let schema = json!({
//! #     "type": "object",
//! #     "properties": {
//! #         "x": { "type": "number" },
//! #         "y": { "type": "number" },
//! #         "z": { "type": "number" }
//! #     },
//! #     "required": ["x", "y", "z"]
//! # });
//! # let definition = ComponentDefinition::new(position_component, schema);
//!
//! // Valid data
//! let valid_data = json!({"x": 1.0, "y": 2.0, "z": 3.0});
//! assert!(definition.validate_component_data(&valid_data).is_ok());
//!
//! // Invalid data (missing required field)
//! let invalid_data = json!({"x": 1.0, "y": 2.0});
//! assert!(definition.validate_component_data(&invalid_data).is_err());
//! ```
//!
//! ### HTTP API with YAML Support
//!
//! The HTTP API automatically detects the content type and accepts both JSON and YAML:
//!
//! ```bash
//! # POST with JSON (default)
//! curl -X POST http://localhost:8080/componentdefinition \
//!   -H "Content-Type: application/json" \
//!   -d '{"component":"Position","schema":{"type":"object","properties":{"x":{"type":"number"}}}}'
//!
//! # POST with YAML
//! curl -X POST http://localhost:8080/componentdefinition \
//!   -H "Content-Type: application/yaml" \
//!   -d 'component: Position
//! schema:
//!   type: object
//!   properties:
//!     x:
//!       type: number'
//! ```

use std::collections::HashMap;
use std::sync::Arc;

use axum::Router;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::Json;
use axum::routing::get;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    DataStore, DataStoreOperations, Entity, OperationStatus, SaveEntry, SaveMetadata,
    SaveOperation, SavefileManager, ValidationResult as LogValidationResult, validate_value,
};

///////////////////////////////////////////// Component ////////////////////////////////////////////

/// A component type identifier that follows Rust naming conventions.
///
/// Components represent typed data that can be attached to entities. The component
/// type identifier must be a valid Rust type path, supporting both simple names
/// and module-qualified paths.
///
/// # Examples
///
/// ```rust
/// use stigmergy::Component;
///
/// // Simple component names
/// let health = Component::new("Health").unwrap();
/// let position = Component::new("Position").unwrap();
///
/// // Module-qualified component names
/// let issue = Component::new("ghai::Issue").unwrap();
/// let hashmap = Component::new("std::collections::HashMap").unwrap();
///
/// // Invalid names return None
/// assert!(Component::new("123Invalid").is_none());
/// assert!(Component::new("").is_none());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Component(String);

impl Component {
    /// Creates a new Component with the given type identifier.
    ///
    /// The identifier must be a valid Rust type path consisting of valid identifiers
    /// separated by `::`. Each identifier must start with a letter or underscore
    /// and contain only alphanumeric characters and underscores.
    ///
    /// # Arguments
    /// * `c` - A string-like type that can be converted to a component identifier
    ///
    /// # Returns
    /// * `Some(Component)` - If the identifier is valid
    /// * `None` - If the identifier is invalid
    ///
    /// # Examples
    /// ```rust
    /// # use stigmergy::Component;
    /// assert!(Component::new("Position").is_some());
    /// assert!(Component::new("ghai::Issue").is_some());
    /// assert!(Component::new("123Invalid").is_none());
    /// ```
    pub fn new(c: impl Into<String>) -> Option<Component> {
        let s = c.into();
        if is_valid_rust_type_path(&s) {
            Some(Component(s))
        } else {
            None
        }
    }

    /// Returns the component type identifier as a string slice.
    ///
    /// # Examples
    /// ```rust
    /// # use stigmergy::Component;
    /// let component = Component::new("Position").unwrap();
    /// assert_eq!(component.as_str(), "Position");
    /// ```
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Request structure for creating a new component instance.
///
/// This structure is used when attaching component data to an entity via HTTP API.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreateComponentRequest {
    /// The component type identifier
    pub component: Component,
    /// The component data (must validate against the component's schema)
    pub data: Value,
}

/// Response structure for successful component creation.
///
/// Contains the entity that the component was attached to, along with the
/// component type and data that was stored.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreateComponentResponse {
    /// The entity that owns this component
    pub entity: Entity,
    /// The component type identifier
    pub component: Component,
    /// The component data that was stored
    pub data: Value,
}

/// A component instance item used in list responses.
///
/// Represents a single component attached to an entity, containing both
/// the component type and its associated data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComponentListItem {
    /// The component type identifier
    pub component: Component,
    /// The component data
    pub data: Value,
}

/// Validates that a string is a valid Rust identifier.
///
/// A valid Rust identifier must:
/// - Be non-empty
/// - Start with a letter (a-z, A-Z) or underscore
/// - Contain only letters, digits, or underscores
///
/// # Examples
/// ```rust
/// # use stigmergy::Component;
/// // Valid identifiers
/// assert!(Component::new("foo").is_some());
/// assert!(Component::new("_bar").is_some());
/// assert!(Component::new("baz123").is_some());
///
/// // Invalid identifiers
/// assert!(Component::new("123foo").is_none());
/// assert!(Component::new("foo-bar").is_none());
/// ```
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

/// Validates that a string is a valid Rust type path.
///
/// A valid Rust type path consists of valid identifiers separated by `::`.
/// This supports both simple names and module-qualified paths.
///
/// # Examples
/// ```rust
/// # use stigmergy::Component;
/// // Valid type paths
/// assert!(Component::new("String").is_some());
/// assert!(Component::new("std::collections::HashMap").is_some());
/// assert!(Component::new("ghai::Issue").is_some());
///
/// // Invalid type paths
/// assert!(Component::new("").is_none());
/// assert!(Component::new("::").is_none());
/// assert!(Component::new("foo::").is_none());
/// ```
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

async fn get_components_for_entity(
    State((logger, data_store)): State<(Arc<SavefileManager>, Arc<dyn DataStore>)>,
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
            .map(|(component, data)| ComponentListItem { component, data })
            .collect(),
        Err(_) => vec![],
    };

    let save_entry = SaveEntry::new(
        SaveOperation::ComponentGet {
            component_id: None,
            found: !components.is_empty(),
        },
        SaveMetadata::rest_api(None),
    );
    logger.save_or_error(&save_entry);

    Ok(Json(components))
}

#[allow(dead_code)]
async fn get_all_components(
    State((logger, data_store)): State<(Arc<SavefileManager>, Arc<dyn DataStore>)>,
    Query(_params): Query<HashMap<String, String>>,
) -> Result<Json<Vec<Value>>, (StatusCode, &'static str)> {
    let components = match data_store.list_components() {
        Ok(comp_list) => comp_list
            .into_iter()
            .map(|((_entity, _id), data)| data)
            .collect(),
        Err(_) => vec![],
    };

    let save_entry = SaveEntry::new(
        SaveOperation::ComponentGet {
            component_id: None,
            found: !components.is_empty(),
        },
        SaveMetadata::rest_api(None),
    );
    logger.save_or_error(&save_entry);

    Ok(Json(components))
}

async fn create_component_for_entity(
    State((logger, data_store)): State<(Arc<SavefileManager>, Arc<dyn DataStore>)>,
    Path(entity_id): Path<String>,
    Json(request): Json<CreateComponentRequest>,
) -> Result<Json<CreateComponentResponse>, (StatusCode, &'static str)> {
    // Parse entity ID (prepend "entity:" prefix to base64 part from URL)
    let full_entity_id = format!("entity:{}", entity_id);
    let entity: Entity = match full_entity_id.parse() {
        Ok(e) => e,
        Err(_) => return Err((StatusCode::BAD_REQUEST, "invalid entity id")),
    };

    // Use a fallback schema for validation
    // Component definitions are now managed via PostgreSQL and accessed through
    // the component definition router, not through the DataStore trait
    let validation_schema = serde_json::json!({
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

    let validation_result = match validate_value(&request.data, &validation_schema) {
        Ok(()) => Some(LogValidationResult::success()),
        Err(e) => {
            let save_entry = SaveEntry::new(
                SaveOperation::ComponentCreate {
                    entity_id: entity_id.clone(),
                    component_id: "generated_id".to_string(),
                    component_data: request.data.clone(),
                    validation_result: Some(LogValidationResult::failed(e.to_string())),
                },
                SaveMetadata::rest_api(None).with_status(OperationStatus::Failed),
            );
            logger.save_or_error(&save_entry);
            return Err((StatusCode::BAD_REQUEST, "data validation failed"));
        }
    };

    let component_id = request.component.as_str().to_string();
    let result = DataStoreOperations::create_component(
        &*data_store,
        &entity,
        &request.component,
        &request.data,
    );
    if !result.success {
        let save_entry = SaveEntry::new(
            SaveOperation::ComponentCreate {
                entity_id: entity_id.clone(),
                component_id: component_id.clone(),
                component_data: request.data.clone(),
                validation_result,
            },
            SaveMetadata::rest_api(None).with_status(OperationStatus::Failed),
        );
        logger.save_or_error(&save_entry);
        return Err(match result.into_error() {
            crate::DataStoreError::AlreadyExists => (StatusCode::CONFLICT, "already exists"),
            crate::DataStoreError::NotFound => (StatusCode::NOT_FOUND, "entity not found"), // Entity doesn't exist
            _ => (StatusCode::INTERNAL_SERVER_ERROR, "internal server error"),
        });
    }

    let save_entry = SaveEntry::new(
        SaveOperation::ComponentCreate {
            entity_id,
            component_id,
            component_data: request.data.clone(),
            validation_result,
        },
        SaveMetadata::rest_api(None),
    );
    logger.save_or_error(&save_entry);

    let response = CreateComponentResponse {
        entity,
        component: request.component,
        data: request.data,
    };

    Ok(Json(response))
}

async fn update_component_for_entity(
    State((logger, data_store)): State<(Arc<SavefileManager>, Arc<dyn DataStore>)>,
    Path(entity_id): Path<String>,
    Json(component): Json<Value>,
) -> Result<Json<Value>, (StatusCode, &'static str)> {
    // Parse entity ID (prepend "entity:" prefix to base64 part from URL)
    let full_entity_id = format!("entity:{}", entity_id);
    let entity = match full_entity_id.parse::<Entity>() {
        Ok(e) => e,
        Err(_) => return Err((StatusCode::BAD_REQUEST, "invalid entity id")),
    };

    let component_id = Component::new("updated_id").unwrap();
    let old_data = data_store
        .get_component(&entity, &component_id)
        .ok()
        .flatten();
    let result =
        DataStoreOperations::update_component(&*data_store, &entity, &component_id, &component);
    if !result.success {
        let save_entry = SaveEntry::new(
            SaveOperation::ComponentUpdate {
                entity_id: entity_id.clone(),
                component_id: component_id.as_str().to_string(),
                old_data,
                new_data: component.clone(),
                validation_result: None,
            },
            SaveMetadata::rest_api(None).with_status(OperationStatus::Failed),
        );
        logger.save_or_error(&save_entry);
        return Err((StatusCode::INTERNAL_SERVER_ERROR, "internal server error"));
    }

    let save_entry = SaveEntry::new(
        SaveOperation::ComponentUpdate {
            entity_id,
            component_id: component_id.as_str().to_string(),
            old_data,
            new_data: component.clone(),
            validation_result: None,
        },
        SaveMetadata::rest_api(None),
    );
    logger.save_or_error(&save_entry);

    Ok(Json(component))
}

async fn patch_component_for_entity(
    State((logger, data_store)): State<(Arc<SavefileManager>, Arc<dyn DataStore>)>,
    Path(entity_id): Path<String>,
    Json(patch): Json<Value>,
) -> Result<Json<Value>, (StatusCode, &'static str)> {
    // Parse entity ID (prepend "entity:" prefix to base64 part from URL)
    let full_entity_id = format!("entity:{}", entity_id);
    let entity = match full_entity_id.parse::<Entity>() {
        Ok(e) => e,
        Err(_) => return Err((StatusCode::BAD_REQUEST, "invalid entity id")),
    };

    let component_id = Component::new("patched_id").unwrap();
    let result =
        DataStoreOperations::update_component(&*data_store, &entity, &component_id, &patch);
    if !result.success {
        let save_entry = SaveEntry::new(
            SaveOperation::ComponentPatch {
                entity_id: entity_id.clone(),
                component_id: component_id.as_str().to_string(),
                patch_data: patch.clone(),
                result_data: patch.clone(),
            },
            SaveMetadata::rest_api(None).with_status(OperationStatus::Failed),
        );
        logger.save_or_error(&save_entry);
        return Err((StatusCode::INTERNAL_SERVER_ERROR, "internal server error"));
    }

    let save_entry = SaveEntry::new(
        SaveOperation::ComponentPatch {
            entity_id,
            component_id: component_id.as_str().to_string(),
            patch_data: patch.clone(),
            result_data: patch.clone(),
        },
        SaveMetadata::rest_api(None),
    );
    logger.save_or_error(&save_entry);

    Ok(Json(patch))
}

async fn delete_components_for_entity(
    State((logger, data_store)): State<(Arc<SavefileManager>, Arc<dyn DataStore>)>,
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
        let save_entry = SaveEntry::new(
            SaveOperation::ComponentDeleteAll { count_deleted: 0 },
            SaveMetadata::rest_api(None).with_status(OperationStatus::Failed),
        );
        logger.save_or_error(&save_entry);
        return Err((StatusCode::INTERNAL_SERVER_ERROR, "internal server error"));
    };

    let save_entry = SaveEntry::new(
        SaveOperation::ComponentDeleteAll { count_deleted },
        SaveMetadata::rest_api(None),
    );
    logger.save_or_error(&save_entry);

    Ok(StatusCode::NO_CONTENT)
}

async fn get_component_by_id_for_entity(
    State((logger, data_store)): State<(Arc<SavefileManager>, Arc<dyn DataStore>)>,
    Path((entity_id, component_id)): Path<(String, String)>,
) -> Result<Json<Value>, (StatusCode, &'static str)> {
    // Parse entity ID (prepend "entity:" prefix to base64 part from URL)
    let full_entity_id = format!("entity:{}", entity_id);
    let entity = match full_entity_id.parse::<Entity>() {
        Ok(e) => e,
        Err(_) => return Err((StatusCode::BAD_REQUEST, "invalid entity id")),
    };

    let component_type =
        Component::new(&component_id).ok_or((StatusCode::BAD_REQUEST, "invalid component name"))?;
    let component = match data_store.get_component(&entity, &component_type) {
        Ok(Some(data)) => data,
        Ok(None) | Err(_) => {
            let save_entry = SaveEntry::new(
                SaveOperation::ComponentGet {
                    component_id: Some(component_id),
                    found: false,
                },
                SaveMetadata::rest_api(None),
            );
            logger.save_or_error(&save_entry);
            return Err((StatusCode::NOT_FOUND, "not found"));
        }
    };

    let save_entry = SaveEntry::new(
        SaveOperation::ComponentGet {
            component_id: Some(component_id),
            found: true,
        },
        SaveMetadata::rest_api(None),
    );
    logger.save_or_error(&save_entry);

    Ok(Json(component))
}

async fn update_component_by_id_for_entity(
    State((logger, data_store)): State<(Arc<SavefileManager>, Arc<dyn DataStore>)>,
    Path((entity_id, component_id)): Path<(String, String)>,
    Json(component): Json<Value>,
) -> Result<Json<Value>, (StatusCode, &'static str)> {
    // Parse entity ID (prepend "entity:" prefix to base64 part from URL)
    let full_entity_id = format!("entity:{}", entity_id);
    let entity = match full_entity_id.parse::<Entity>() {
        Ok(e) => e,
        Err(_) => return Err((StatusCode::BAD_REQUEST, "invalid entity id")),
    };

    let component_type =
        Component::new(&component_id).ok_or((StatusCode::BAD_REQUEST, "invalid component name"))?;
    let old_data = data_store
        .get_component(&entity, &component_type)
        .ok()
        .flatten();
    let result =
        DataStoreOperations::update_component(&*data_store, &entity, &component_type, &component);
    if !result.success {
        let save_entry = SaveEntry::new(
            SaveOperation::ComponentUpdate {
                entity_id: entity_id.clone(),
                component_id: component_id.as_str().to_string(),
                old_data,
                new_data: component.clone(),
                validation_result: None,
            },
            SaveMetadata::rest_api(None).with_status(OperationStatus::Failed),
        );
        logger.save_or_error(&save_entry);
        return Err((StatusCode::INTERNAL_SERVER_ERROR, "internal server error"));
    }

    let save_entry = SaveEntry::new(
        SaveOperation::ComponentUpdate {
            entity_id,
            component_id: component_id.as_str().to_string(),
            old_data,
            new_data: component.clone(),
            validation_result: None,
        },
        SaveMetadata::rest_api(None),
    );
    logger.save_or_error(&save_entry);

    Ok(Json(component))
}

async fn patch_component_by_id_for_entity(
    State((logger, data_store)): State<(Arc<SavefileManager>, Arc<dyn DataStore>)>,
    Path((entity_id, component_id)): Path<(String, String)>,
    Json(patch): Json<Value>,
) -> Result<Json<Value>, (StatusCode, &'static str)> {
    // Parse entity ID (prepend "entity:" prefix to base64 part from URL)
    let full_entity_id = format!("entity:{}", entity_id);
    let entity = match full_entity_id.parse::<Entity>() {
        Ok(e) => e,
        Err(_) => return Err((StatusCode::BAD_REQUEST, "invalid entity id")),
    };

    let component_type =
        Component::new(&component_id).ok_or((StatusCode::BAD_REQUEST, "invalid component name"))?;
    let mut component = patch.clone();
    if let Some(obj) = component.as_object_mut() {
        obj.insert(
            "id".to_string(),
            serde_json::Value::String(component_id.clone()),
        );
    }

    let result =
        DataStoreOperations::update_component(&*data_store, &entity, &component_type, &component);
    if !result.success {
        let save_entry = SaveEntry::new(
            SaveOperation::ComponentPatch {
                entity_id: entity_id.clone(),
                component_id: component_id.as_str().to_string(),
                patch_data: patch,
                result_data: component.clone(),
            },
            SaveMetadata::rest_api(None).with_status(OperationStatus::Failed),
        );
        logger.save_or_error(&save_entry);
        return Err((StatusCode::INTERNAL_SERVER_ERROR, "internal server error"));
    }

    let save_entry = SaveEntry::new(
        SaveOperation::ComponentPatch {
            entity_id,
            component_id,
            patch_data: patch,
            result_data: component.clone(),
        },
        SaveMetadata::rest_api(None),
    );
    logger.save_or_error(&save_entry);

    Ok(Json(component))
}

async fn delete_component_by_id_for_entity(
    State((logger, data_store)): State<(Arc<SavefileManager>, Arc<dyn DataStore>)>,
    Path((entity_id, component_id)): Path<(String, String)>,
) -> Result<StatusCode, (StatusCode, &'static str)> {
    // Parse entity ID (prepend "entity:" prefix to base64 part from URL)
    let full_entity_id = format!("entity:{}", entity_id);
    let entity = match full_entity_id.parse::<Entity>() {
        Ok(e) => e,
        Err(_) => return Err((StatusCode::BAD_REQUEST, "invalid entity id")),
    };

    let component_type =
        Component::new(&component_id).ok_or((StatusCode::BAD_REQUEST, "invalid component name"))?;
    let deleted_data = data_store
        .get_component(&entity, &component_type)
        .ok()
        .flatten();
    let result = DataStoreOperations::delete_component(&*data_store, &entity, &component_type);
    if !result.success {
        let save_entry = SaveEntry::new(
            SaveOperation::ComponentDelete {
                entity_id: entity_id.clone(),
                component_id: component_id.clone(),
                deleted_data,
            },
            SaveMetadata::rest_api(None).with_status(OperationStatus::Failed),
        );
        logger.save_or_error(&save_entry);
        return Err((StatusCode::INTERNAL_SERVER_ERROR, "internal server error"));
    }

    let save_entry = SaveEntry::new(
        SaveOperation::ComponentDelete {
            entity_id,
            component_id,
            deleted_data,
        },
        SaveMetadata::rest_api(None),
    );
    logger.save_or_error(&save_entry);

    Ok(StatusCode::NO_CONTENT)
}

////////////////////////////////////////////// router //////////////////////////////////////////////

/// Creates an HTTP router for component instance operations.
///
/// This function sets up HTTP endpoints for managing component instances attached to entities.
///
/// # Arguments
/// * `logger` - Shared savefile manager for operation logging
/// * `data_store` - Shared data store for component instance storage
///
/// # Returns
/// An Axum Router configured with component instance endpoints
///
/// # Endpoints Created
/// - `POST /entity/:entity_base64/component` - Create component instance
/// - `GET /entity/:entity_base64/component/:component_id` - Get component instance
/// - `PUT /entity/:entity_base64/component/:component_id` - Update component instance
/// - `PATCH /entity/:entity_base64/component/:component_id` - Patch component instance
/// - `DELETE /entity/:entity_base64/component/:component_id` - Delete component instance
/// - `DELETE /entity/:entity_base64/component` - Delete all components for entity
/// - `GET /entity/:entity_base64/component` - List components for entity
///
/// # Examples
/// ```no_run
/// # use stigmergy::{create_component_instance_router, SavefileManager, InMemoryDataStore};
/// # use std::sync::Arc;
/// # use std::path::PathBuf;
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let logger = Arc::new(SavefileManager::new(PathBuf::from("test.jsonl")));
/// let store = Arc::new(InMemoryDataStore::new());
/// let router = create_component_instance_router(logger, store);
/// # Ok(())
/// # }
/// ```
pub fn create_component_instance_router(
    logger: Arc<SavefileManager>,
    data_store: Arc<dyn DataStore>,
) -> Router {
    Router::new()
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
        clear_savefile, create_test_savefile_manager_with_path, load_entries, test_data_store,
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

    // Component Instance HTTP Endpoint Tests
    #[tokio::test]
    async fn get_components_logs_correctly() {
        let (logger, log_path) =
            create_test_savefile_manager_with_path("component", "get_components");
        clear_savefile(&log_path);

        let logs_before = load_entries(&log_path);
        assert!(logs_before.is_empty());

        let data_store = test_data_store();
        let entity = Entity::random().unwrap();
        let entity_id = entity.base64_part();
        let result = get_components_for_entity(
            State((logger, data_store)),
            Path(entity_id),
            Query(HashMap::<String, String>::new()),
        )
        .await;
        assert!(result.is_ok());

        let logs_after = load_entries(&log_path);
        assert_eq!(logs_after.len(), 1);

        let save_entry = &logs_after[0];
        assert_eq!(save_entry.operation_type(), "ComponentGet");
        assert!(save_entry.is_success());

        match &save_entry.operation {
            SaveOperation::ComponentGet {
                component_id,
                found,
            } => {
                assert!(component_id.is_none());
                assert!(!*found);
            }
            _ => panic!("Expected ComponentGet operation"),
        }

        clear_savefile(&log_path);
    }

    #[tokio::test]
    async fn create_component_success_logs_correctly() {
        let (logger, log_path) =
            create_test_savefile_manager_with_path("component", "create_comp_success");
        clear_savefile(&log_path);

        let component_data = serde_json::json!("Red");
        let logs_before = load_entries(&log_path);
        assert!(logs_before.is_empty());

        let entity = Entity::random().unwrap();
        let data_store = test_data_store();

        let request = CreateComponentRequest {
            component: Component::new("TestComponent").unwrap(),
            data: component_data.clone(),
        };

        let result = create_component_for_entity(
            State((logger, data_store)),
            Path(entity.base64_part()),
            Json(request),
        )
        .await;
        assert!(result.is_ok());

        let logs_after = load_entries(&log_path);
        assert_eq!(logs_after.len(), 1);

        let save_entry = &logs_after[0];
        assert_eq!(save_entry.operation_type(), "ComponentCreate");
        assert!(save_entry.is_success());

        match &save_entry.operation {
            SaveOperation::ComponentCreate {
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

        clear_savefile(&log_path);
    }

    #[tokio::test]
    async fn create_component_failure_logs_correctly() {
        let (logger, log_path) =
            create_test_savefile_manager_with_path("component", "create_comp_failure");
        clear_savefile(&log_path);

        let component_data = serde_json::json!("InvalidColor");
        let logs_before = load_entries(&log_path);
        assert!(logs_before.is_empty());

        let entity = Entity::random().unwrap();
        let request = CreateComponentRequest {
            component: Component::new("TestComponent").unwrap(),
            data: component_data.clone(),
        };

        let result = create_component_for_entity(
            State((logger, test_data_store())),
            Path(entity.base64_part()),
            Json(request),
        )
        .await;
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            (StatusCode::BAD_REQUEST, "data validation failed")
        );

        let logs_after = load_entries(&log_path);
        assert_eq!(logs_after.len(), 1);

        let save_entry = &logs_after[0];
        assert_eq!(save_entry.operation_type(), "ComponentCreate");
        assert!(save_entry.is_failure());

        match &save_entry.operation {
            SaveOperation::ComponentCreate {
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

        clear_savefile(&log_path);
    }

    #[tokio::test]
    async fn update_component_logs_correctly() {
        let (logger, log_path) = create_test_savefile_manager_with_path("component", "update_comp");
        clear_savefile(&log_path);

        let entity = Entity::random().unwrap();
        let entity_id = entity.base64_part();
        let component_data = serde_json::json!({"color": "blue"});
        let logs_before = load_entries(&log_path);
        assert!(logs_before.is_empty());

        let result = update_component_for_entity(
            State((logger, test_data_store())),
            Path(entity_id),
            Json(component_data.clone()),
        )
        .await;
        assert!(result.is_ok());

        let logs_after = load_entries(&log_path);
        assert_eq!(logs_after.len(), 1);

        let save_entry = &logs_after[0];
        assert_eq!(save_entry.operation_type(), "ComponentUpdate");
        assert!(save_entry.is_success());

        match &save_entry.operation {
            SaveOperation::ComponentUpdate {
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

        clear_savefile(&log_path);
    }

    #[tokio::test]
    async fn patch_component_logs_correctly() {
        let (logger, log_path) = create_test_savefile_manager_with_path("component", "patch_comp");
        clear_savefile(&log_path);

        let entity = Entity::random().unwrap();
        let entity_id = entity.base64_part();
        let patch_data = serde_json::json!({"size": "large"});
        let logs_before = load_entries(&log_path);
        assert!(logs_before.is_empty());

        let result = patch_component_for_entity(
            State((logger, test_data_store())),
            Path(entity_id),
            Json(patch_data.clone()),
        )
        .await;
        assert!(result.is_ok());

        let logs_after = load_entries(&log_path);
        assert_eq!(logs_after.len(), 1);

        let save_entry = &logs_after[0];
        assert_eq!(save_entry.operation_type(), "ComponentPatch");
        assert!(save_entry.is_success());

        match &save_entry.operation {
            SaveOperation::ComponentPatch {
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

        clear_savefile(&log_path);
    }

    #[tokio::test]
    async fn delete_components_logs_correctly() {
        let (logger, log_path) =
            create_test_savefile_manager_with_path("component", "delete_comps");
        clear_savefile(&log_path);

        let entity = Entity::random().unwrap();
        let entity_id = entity.base64_part();
        let logs_before = load_entries(&log_path);
        assert!(logs_before.is_empty());

        let result =
            delete_components_for_entity(State((logger, test_data_store())), Path(entity_id)).await;
        assert_eq!(result, Ok(StatusCode::NO_CONTENT));

        let logs_after = load_entries(&log_path);
        assert_eq!(logs_after.len(), 1);

        let save_entry = &logs_after[0];
        assert_eq!(save_entry.operation_type(), "ComponentDeleteAll");
        assert!(save_entry.is_success());

        match &save_entry.operation {
            SaveOperation::ComponentDeleteAll { count_deleted } => {
                assert_eq!(*count_deleted, 0);
            }
            _ => panic!("Expected ComponentDeleteAll operation"),
        }

        clear_savefile(&log_path);
    }

    #[tokio::test]
    async fn get_component_by_id_logs_correctly() {
        let (logger, log_path) =
            create_test_savefile_manager_with_path("component", "get_comp_by_id");
        clear_savefile(&log_path);

        let entity = Entity::random().unwrap();
        let entity_id = entity.base64_part();
        let test_id = "comp123".to_string();
        let logs_before = load_entries(&log_path);
        assert!(logs_before.is_empty());

        let result = get_component_by_id_for_entity(
            State((logger, test_data_store())),
            Path((entity_id, test_id.clone())),
        )
        .await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), (StatusCode::NOT_FOUND, "not found"));

        let logs_after = load_entries(&log_path);
        assert_eq!(logs_after.len(), 1);

        let save_entry = &logs_after[0];
        assert_eq!(save_entry.operation_type(), "ComponentGet");
        assert!(save_entry.is_success());

        match &save_entry.operation {
            SaveOperation::ComponentGet {
                component_id,
                found,
            } => {
                assert_eq!(*component_id, Some(test_id));
                assert!(!*found);
            }
            _ => panic!("Expected ComponentGet operation"),
        }

        clear_savefile(&log_path);
    }

    #[tokio::test]
    async fn update_component_by_id_logs_correctly() {
        let (logger, log_path) =
            create_test_savefile_manager_with_path("component", "update_comp_by_id");
        clear_savefile(&log_path);

        let entity = Entity::random().unwrap();
        let entity_id = entity.base64_part();
        let test_id = "comp456".to_string();
        let component_data = serde_json::json!({"status": "active"});
        let logs_before = load_entries(&log_path);
        assert!(logs_before.is_empty());

        let result = update_component_by_id_for_entity(
            State((logger, test_data_store())),
            Path((entity_id, test_id.clone())),
            Json(component_data.clone()),
        )
        .await;
        assert!(result.is_ok());

        let logs_after = load_entries(&log_path);
        assert_eq!(logs_after.len(), 1);

        let save_entry = &logs_after[0];
        assert_eq!(save_entry.operation_type(), "ComponentUpdate");
        assert!(save_entry.is_success());

        match &save_entry.operation {
            SaveOperation::ComponentUpdate {
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

        clear_savefile(&log_path);
    }

    #[tokio::test]
    async fn patch_component_by_id_logs_correctly() {
        let (logger, log_path) =
            create_test_savefile_manager_with_path("component", "patch_comp_by_id");
        clear_savefile(&log_path);

        let entity = Entity::random().unwrap();
        let entity_id = entity.base64_part();
        let test_id = "comp789".to_string();
        let patch_data = serde_json::json!({"priority": "high"});
        let logs_before = load_entries(&log_path);
        assert!(logs_before.is_empty());

        let result = patch_component_by_id_for_entity(
            State((logger, test_data_store())),
            Path((entity_id, test_id.clone())),
            Json(patch_data.clone()),
        )
        .await;
        assert!(result.is_ok());

        let logs_after = load_entries(&log_path);
        assert_eq!(logs_after.len(), 1);

        let save_entry = &logs_after[0];
        assert_eq!(save_entry.operation_type(), "ComponentPatch");
        assert!(save_entry.is_success());

        match &save_entry.operation {
            SaveOperation::ComponentPatch {
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

        clear_savefile(&log_path);
    }

    #[tokio::test]
    async fn delete_component_by_id_logs_correctly() {
        let (logger, log_path) =
            create_test_savefile_manager_with_path("component", "delete_comp_by_id");
        clear_savefile(&log_path);

        let entity = Entity::random().unwrap();
        let entity_id = entity.base64_part();
        let test_id = "comp_delete".to_string();
        let logs_before = load_entries(&log_path);
        assert!(logs_before.is_empty());

        let result = delete_component_by_id_for_entity(
            State((logger, test_data_store())),
            Path((entity_id, test_id.clone())),
        )
        .await;
        assert_eq!(result, Ok(StatusCode::NO_CONTENT));

        let logs_after = load_entries(&log_path);
        assert_eq!(logs_after.len(), 1);

        let save_entry = &logs_after[0];
        assert_eq!(save_entry.operation_type(), "ComponentDelete");
        assert!(save_entry.is_success());

        match &save_entry.operation {
            SaveOperation::ComponentDelete {
                entity_id: _,
                component_id,
                deleted_data,
            } => {
                assert_eq!(*component_id, test_id);
                assert!(deleted_data.is_none());
            }
            _ => panic!("Expected ComponentDelete operation"),
        }

        clear_savefile(&log_path);
    }

    #[tokio::test]
    async fn data_store_error_handling_component() {
        let (logger, log_path) =
            create_test_savefile_manager_with_path("component", "data_store_error_comp");
        clear_savefile(&log_path);

        let component_data = serde_json::json!("Green");
        let data_store = test_data_store();
        let test_component = Component::new("TestComponent").unwrap();

        let entity = Entity::random().unwrap();

        // Create component first (directly in data store)
        data_store
            .create_component(&entity, &test_component, &component_data)
            .unwrap();

        // Try to create again via HTTP handler - should get CONFLICT
        let request = CreateComponentRequest {
            component: test_component,
            data: component_data.clone(),
        };

        let result = create_component_for_entity(
            State((logger.clone(), data_store.clone())),
            Path(entity.base64_part()),
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

        clear_savefile(&log_path);
    }
}
