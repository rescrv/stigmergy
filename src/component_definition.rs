//! # Component Definition System
//!
//! This module implements the component definition system for stigmergy, providing
//! schema-based validation for component data structures. Component definitions
//! establish the structure and validation rules for component types.
//!
//! ## Key Features
//!
//! - **Schema Validation**: JSON Schema-based validation for component data
//! - **Type Safety**: Component types must follow Rust naming conventions
//! - **Flexible Schemas**: Support for complex schemas including oneOf unions and enums
//! - **HTTP API**: Complete REST API for component definition management
//! - **JSON and YAML Support**: Accept both formats based on Content-Type header

use std::collections::HashMap;

use axum::Router;
use axum::async_trait;
use axum::body::Bytes;
use axum::extract::{FromRequest, Path, Query, Request, State};
use axum::http::StatusCode;
use axum::response::Json;
use axum::routing::get;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{Component, ValidationError, validate_value};

/// A component definition that associates a component type with its JSON schema.
///
/// Component definitions establish the structure and validation rules for component data.
/// Each definition consists of a component type identifier and a JSON schema that
/// describes what data is valid for that component type.
///
/// # Examples
///
/// ```rust
/// use stigmergy::{Component, ComponentDefinition};
/// use serde_json::json;
///
/// // Create a simple component definition
/// let health_component = Component::new("Health").unwrap();
/// let health_schema = json!({
///     "type": "object",
///     "properties": {
///         "hp": { "type": "integer", "minimum": 0 },
///         "max_hp": { "type": "integer", "minimum": 1 }
///     },
///     "required": ["hp", "max_hp"]
/// });
///
/// let definition = ComponentDefinition::new(health_component, health_schema);
///
/// // Validate the schema structure
/// assert!(definition.validate_schema().is_ok());
///
/// // Validate component data against the schema
/// let valid_data = json!({"hp": 100, "max_hp": 100});
/// assert!(definition.validate_component_data(&valid_data).is_ok());
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComponentDefinition {
    /// The component type this definition applies to
    pub component: Component,
    /// The JSON schema that validates component data
    pub schema: serde_json::Value,
}

impl ComponentDefinition {
    /// Creates a new component definition.
    ///
    /// # Arguments
    /// * `component` - The component type identifier
    /// * `schema` - The JSON schema for validating component data
    ///
    /// # Examples
    /// ```rust
    /// # use stigmergy::{Component, ComponentDefinition};
    /// # use serde_json::json;
    /// let component = Component::new("Position").unwrap();
    /// let schema = json!({"type": "object", "properties": {"x": {"type": "number"}}});
    /// let definition = ComponentDefinition::new(component, schema);
    /// ```
    pub fn new(component: Component, schema: Value) -> Self {
        Self { component, schema }
    }

    /// Validates that the schema structure is well-formed.
    ///
    /// This method checks that the JSON schema follows the expected format and
    /// contains valid schema constructs. It validates the schema structure
    /// recursively to ensure all nested schemas are also valid.
    ///
    /// # Returns
    /// * `Ok(())` - If the schema is valid
    /// * `Err(ValidationError)` - If the schema structure is invalid
    ///
    /// # Examples
    /// ```rust
    /// # use stigmergy::{Component, ComponentDefinition};
    /// # use serde_json::json;
    /// let component = Component::new("Test").unwrap();
    ///
    /// // Valid schema
    /// let valid_schema = json!({"type": "string"});
    /// let definition = ComponentDefinition::new(component.clone(), valid_schema);
    /// assert!(definition.validate_schema().is_ok());
    ///
    /// // Invalid schema (unknown type)
    /// let invalid_schema = json!({"type": "invalid_type"});
    /// let definition = ComponentDefinition::new(component, invalid_schema);
    /// assert!(definition.validate_schema().is_err());
    /// ```
    pub fn validate_schema(&self) -> Result<(), ValidationError> {
        validate_schema_structure(&self.schema)
    }

    /// Validates component data against this definition's schema.
    ///
    /// This method checks that the provided data conforms to the JSON schema
    /// defined for this component type. It performs comprehensive validation
    /// including type checking, required fields, and nested structure validation.
    ///
    /// # Arguments
    /// * `data` - The component data to validate
    ///
    /// # Returns
    /// * `Ok(())` - If the data is valid according to the schema
    /// * `Err(ValidationError)` - If the data doesn't match the schema
    ///
    /// # Examples
    /// ```rust
    /// # use stigmergy::{Component, ComponentDefinition};
    /// # use serde_json::json;
    /// let component = Component::new("Health").unwrap();
    /// let schema = json!({
    ///     "type": "object",
    ///     "properties": {"hp": {"type": "integer"}},
    ///     "required": ["hp"]
    /// });
    /// let definition = ComponentDefinition::new(component, schema);
    ///
    /// // Valid data
    /// assert!(definition.validate_component_data(&json!({"hp": 100})).is_ok());
    ///
    /// // Invalid data (wrong type)
    /// assert!(definition.validate_component_data(&json!({"hp": "high"})).is_err());
    ///
    /// // Invalid data (missing required field)
    /// assert!(definition.validate_component_data(&json!({})).is_err());
    /// ```
    pub fn validate_component_data(&self, data: &Value) -> Result<(), ValidationError> {
        validate_value(data, &self.schema)
    }
}

/// A wrapper that extracts ComponentDefinition from either JSON or YAML based on Content-Type.
pub struct ComponentDefinitionExtractor(pub ComponentDefinition);

#[async_trait]
impl<S> FromRequest<S> for ComponentDefinitionExtractor
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let (parts, body) = req.into_parts();
        let content_type = parts
            .headers
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("application/json")
            .to_string();

        let bytes = Bytes::from_request(Request::from_parts(parts, body), state)
            .await
            .map_err(|_| (StatusCode::BAD_REQUEST, "failed to read request body"))?;

        let definition = if content_type.contains("yaml") || content_type.contains("yml") {
            serde_yml::from_slice::<ComponentDefinition>(&bytes)
                .map_err(|_| (StatusCode::BAD_REQUEST, "invalid yaml"))?
        } else {
            serde_json::from_slice::<ComponentDefinition>(&bytes)
                .map_err(|_| (StatusCode::BAD_REQUEST, "invalid json"))?
        };

        Ok(ComponentDefinitionExtractor(definition))
    }
}

/// Validates the structure of a JSON schema to ensure it's well-formed.
///
/// This function recursively validates JSON schema objects to ensure they follow
/// the expected format and contain valid schema constructs. It supports:
/// - Basic types (null, boolean, integer, number, string)
/// - Complex types (array, object)
/// - Union types via oneOf
/// - Nested schemas and recursive validation
///
/// # Arguments
/// * `schema` - The JSON schema value to validate
///
/// # Returns
/// * `Ok(())` - If the schema structure is valid
/// * `Err(ValidationError::InvalidSchema)` - If the schema structure is malformed
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

async fn get_component_definitions(
    State(pool): State<sqlx::PgPool>,
    Query(_params): Query<HashMap<String, String>>,
) -> Result<Json<Vec<ComponentDefinition>>, (StatusCode, &'static str)> {
    match crate::sql::component_definition::list(&pool).await {
        Ok(definitions) => Ok(Json(definitions)),
        Err(_) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to list component definitions",
        )),
    }
}

async fn create_component_definition(
    State(pool): State<sqlx::PgPool>,
    ComponentDefinitionExtractor(definition): ComponentDefinitionExtractor,
) -> Result<Json<ComponentDefinition>, (StatusCode, &'static str)> {
    if let Err(_e) = definition.validate_schema() {
        return Err((StatusCode::BAD_REQUEST, "invalid schema"));
    }

    match crate::sql::component_definition::create(&pool, &definition).await {
        Ok(()) => Ok(Json(definition)),
        Err(crate::DataStoreError::AlreadyExists) => Err((StatusCode::CONFLICT, "already exists")),
        Err(_) => Err((StatusCode::INTERNAL_SERVER_ERROR, "internal server error")),
    }
}

async fn update_component_definition(
    State(pool): State<sqlx::PgPool>,
    ComponentDefinitionExtractor(definition): ComponentDefinitionExtractor,
) -> Result<Json<ComponentDefinition>, (StatusCode, &'static str)> {
    if let Err(_e) = definition.validate_schema() {
        return Err((StatusCode::BAD_REQUEST, "invalid schema"));
    }

    match crate::sql::component_definition::update(&pool, &definition).await {
        Ok(_) => Ok(Json(definition)),
        Err(_) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to update component definition",
        )),
    }
}

async fn patch_component_definition(
    State(pool): State<sqlx::PgPool>,
    Json(patch): Json<Value>,
) -> Result<Json<ComponentDefinition>, (StatusCode, &'static str)> {
    let component = Component::new("PatchedComponent").unwrap();
    let definition = ComponentDefinition {
        component: component.clone(),
        schema: patch.clone(),
    };

    if let Err(_e) = definition.validate_schema() {
        return Err((StatusCode::BAD_REQUEST, "invalid schema"));
    }

    match crate::sql::component_definition::update(&pool, &definition).await {
        Ok(_) => Ok(Json(definition)),
        Err(_) => Err((StatusCode::INTERNAL_SERVER_ERROR, "internal server error")),
    }
}

async fn delete_component_definitions(
    State(pool): State<sqlx::PgPool>,
) -> Result<StatusCode, (StatusCode, &'static str)> {
    let definitions = match crate::sql::component_definition::list(&pool).await {
        Ok(defs) => defs,
        Err(_) => return Err((StatusCode::INTERNAL_SERVER_ERROR, "internal server error")),
    };

    for definition in definitions {
        if crate::sql::component_definition::delete(&pool, &definition.component)
            .await
            .is_err()
        {
            return Err((StatusCode::INTERNAL_SERVER_ERROR, "internal server error"));
        }
    }

    Ok(StatusCode::NO_CONTENT)
}

async fn get_component_definition_by_id(
    State(pool): State<sqlx::PgPool>,
    Path(id): Path<String>,
) -> Result<Json<ComponentDefinition>, (StatusCode, &'static str)> {
    let component =
        Component::new(&id).ok_or((StatusCode::BAD_REQUEST, "invalid component name"))?;

    match crate::sql::component_definition::get(&pool, &component).await {
        Ok(Some(record)) => Ok(Json(record.definition)),
        Ok(None) => Err((StatusCode::NOT_FOUND, "not found")),
        Err(_) => Err((StatusCode::INTERNAL_SERVER_ERROR, "internal server error")),
    }
}

async fn update_component_definition_by_id(
    State(pool): State<sqlx::PgPool>,
    Path(id): Path<String>,
    ComponentDefinitionExtractor(definition): ComponentDefinitionExtractor,
) -> Result<Json<ComponentDefinition>, (StatusCode, &'static str)> {
    let component =
        Component::new(&id).ok_or((StatusCode::BAD_REQUEST, "invalid component name"))?;

    if let Err(_e) = definition.validate_schema() {
        return Err((StatusCode::BAD_REQUEST, "invalid schema"));
    }

    if component != definition.component {
        return Err((StatusCode::BAD_REQUEST, "component name mismatch"));
    }

    match crate::sql::component_definition::update(&pool, &definition).await {
        Ok(_) => Ok(Json(definition)),
        Err(_) => Err((StatusCode::INTERNAL_SERVER_ERROR, "internal server error")),
    }
}

async fn patch_component_definition_by_id(
    State(pool): State<sqlx::PgPool>,
    Path(id): Path<String>,
    Json(patch): Json<Value>,
) -> Result<Json<ComponentDefinition>, (StatusCode, &'static str)> {
    let component =
        Component::new(&id).ok_or((StatusCode::BAD_REQUEST, "invalid component name"))?;
    let definition = ComponentDefinition {
        component: component.clone(),
        schema: patch.clone(),
    };

    if let Err(_e) = definition.validate_schema() {
        return Err((StatusCode::BAD_REQUEST, "invalid schema"));
    }

    match crate::sql::component_definition::update(&pool, &definition).await {
        Ok(_) => Ok(Json(definition)),
        Err(_) => Err((StatusCode::INTERNAL_SERVER_ERROR, "internal server error")),
    }
}

async fn delete_component_definition_by_id(
    State(pool): State<sqlx::PgPool>,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, &'static str)> {
    let component =
        Component::new(&id).ok_or((StatusCode::BAD_REQUEST, "invalid component name"))?;

    match crate::sql::component_definition::delete(&pool, &component).await {
        Ok(true) => Ok(StatusCode::NO_CONTENT),
        Ok(false) => Err((StatusCode::NOT_FOUND, "not found")),
        Err(_) => Err((StatusCode::INTERNAL_SERVER_ERROR, "internal server error")),
    }
}

/// Creates the HTTP router for component definition endpoints.
///
/// This function sets up all the routes for managing component definitions using PostgreSQL.
///
/// # Arguments
/// * `pool` - PostgreSQL connection pool
///
/// # Returns
/// An Axum Router configured with component definition routes
pub fn create_component_definition_router(pool: sqlx::PgPool) -> Router {
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
        .with_state(pool)
}
