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
//! - **Flexible Schemas**: Support for complex schemas including oneOf unions and enums
//! - **Entity Scoping**: Components are scoped to specific entities
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

use axum::Router;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::Json;
use axum::routing::get;
use serde::{Deserialize, Serialize};
use serde_json::Value;

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
    pub entity: crate::Entity,
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

////////////////////////////////////////////// Routes //////////////////////////////////////////////

/// Lists all component instances for a specific entity.
async fn get_components_for_entity(
    State(pool): State<sqlx::PgPool>,
    Path(entity_str): Path<String>,
) -> Result<Json<Vec<ComponentListItem>>, (StatusCode, &'static str)> {
    let entity: crate::Entity = entity_str
        .parse()
        .map_err(|_| (StatusCode::BAD_REQUEST, "invalid entity ID"))?;

    match crate::sql::component::list_for_entity(&pool, &entity).await {
        Ok(components) => {
            let items: Vec<ComponentListItem> = components
                .into_iter()
                .map(|(component, data)| ComponentListItem { component, data })
                .collect();
            Ok(Json(items))
        }
        Err(_) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to list components",
        )),
    }
}

/// Lists all component instances in the system.
async fn get_all_components(
    State(pool): State<sqlx::PgPool>,
) -> Result<Json<Vec<(String, ComponentListItem)>>, (StatusCode, &'static str)> {
    match crate::sql::component::list_all(&pool).await {
        Ok(components) => {
            let items: Vec<(String, ComponentListItem)> = components
                .into_iter()
                .map(|((entity, component), data)| {
                    (entity.to_string(), ComponentListItem { component, data })
                })
                .collect();
            Ok(Json(items))
        }
        Err(_) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to list all components",
        )),
    }
}

/// Creates a new component instance for an entity.
async fn create_component_for_entity(
    State(pool): State<sqlx::PgPool>,
    Path(entity_str): Path<String>,
    Json(request): Json<CreateComponentRequest>,
) -> Result<Json<CreateComponentResponse>, (StatusCode, String)> {
    let entity: crate::Entity = entity_str
        .parse()
        .map_err(|_| (StatusCode::BAD_REQUEST, "invalid entity ID".to_string()))?;

    // Validate the component data against the schema
    let definition = match crate::sql::component_definition::get(&pool, &request.component).await {
        Ok(Some(def_record)) => def_record.definition,
        Ok(None) => {
            return Err((
                StatusCode::NOT_FOUND,
                format!(
                    "component definition not found: {}",
                    request.component.as_str()
                ),
            ));
        }
        Err(_) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to retrieve component definition".to_string(),
            ));
        }
    };

    if let Err(e) = definition.validate_component_data(&request.data) {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("component data validation failed: {}", e),
        ));
    }

    match crate::sql::component::create(&pool, &entity, &request.component, &request.data).await {
        Ok(()) => {
            let response = CreateComponentResponse {
                entity,
                component: request.component,
                data: request.data,
            };
            Ok(Json(response))
        }
        Err(crate::DataStoreError::AlreadyExists) => Err((
            StatusCode::CONFLICT,
            "component instance already exists for this entity".to_string(),
        )),
        Err(crate::DataStoreError::NotFound) => {
            Err((StatusCode::NOT_FOUND, "entity not found".to_string()))
        }
        Err(_) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to create component instance".to_string(),
        )),
    }
}

/// Gets a specific component instance for an entity.
async fn get_component_by_id_for_entity(
    State(pool): State<sqlx::PgPool>,
    Path((entity_str, component_str)): Path<(String, String)>,
) -> Result<Json<Value>, (StatusCode, &'static str)> {
    let entity: crate::Entity = entity_str
        .parse()
        .map_err(|_| (StatusCode::BAD_REQUEST, "invalid entity ID"))?;

    let component =
        Component::new(component_str).ok_or((StatusCode::BAD_REQUEST, "invalid component name"))?;

    match crate::sql::component::get(&pool, &entity, &component).await {
        Ok(Some(data)) => Ok(Json(data)),
        Ok(None) => Err((StatusCode::NOT_FOUND, "component instance not found")),
        Err(_) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to retrieve component instance",
        )),
    }
}

/// Updates a specific component instance for an entity.
async fn update_component_by_id_for_entity(
    State(pool): State<sqlx::PgPool>,
    Path((entity_str, component_str)): Path<(String, String)>,
    Json(data): Json<Value>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let entity: crate::Entity = entity_str
        .parse()
        .map_err(|_| (StatusCode::BAD_REQUEST, "invalid entity ID".to_string()))?;

    let component = Component::new(component_str).ok_or((
        StatusCode::BAD_REQUEST,
        "invalid component name".to_string(),
    ))?;

    // Validate the component data against the schema
    let definition = match crate::sql::component_definition::get(&pool, &component).await {
        Ok(Some(def_record)) => def_record.definition,
        Ok(None) => {
            return Err((
                StatusCode::NOT_FOUND,
                format!("component definition not found: {}", component.as_str()),
            ));
        }
        Err(_) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to retrieve component definition".to_string(),
            ));
        }
    };

    if let Err(e) = definition.validate_component_data(&data) {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("component data validation failed: {}", e),
        ));
    }

    match crate::sql::component::update(&pool, &entity, &component, &data).await {
        Ok(true) => Ok(Json(data)),
        Ok(false) => Err((
            StatusCode::NOT_FOUND,
            "component instance not found".to_string(),
        )),
        Err(_) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to update component instance".to_string(),
        )),
    }
}

/// Deletes a specific component instance for an entity.
async fn delete_component_by_id_for_entity(
    State(pool): State<sqlx::PgPool>,
    Path((entity_str, component_str)): Path<(String, String)>,
) -> Result<StatusCode, (StatusCode, &'static str)> {
    let entity: crate::Entity = entity_str
        .parse()
        .map_err(|_| (StatusCode::BAD_REQUEST, "invalid entity ID"))?;

    let component =
        Component::new(component_str).ok_or((StatusCode::BAD_REQUEST, "invalid component name"))?;

    match crate::sql::component::delete(&pool, &entity, &component).await {
        Ok(true) => Ok(StatusCode::NO_CONTENT),
        Ok(false) => Err((StatusCode::NOT_FOUND, "component instance not found")),
        Err(_) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to delete component instance",
        )),
    }
}

/// Deletes all component instances for an entity.
async fn delete_components_for_entity(
    State(pool): State<sqlx::PgPool>,
    Path(entity_str): Path<String>,
) -> Result<StatusCode, (StatusCode, &'static str)> {
    let entity: crate::Entity = entity_str
        .parse()
        .map_err(|_| (StatusCode::BAD_REQUEST, "invalid entity ID"))?;

    match crate::sql::component::delete_all_for_entity(&pool, &entity).await {
        Ok(_) => Ok(StatusCode::NO_CONTENT),
        Err(_) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to delete component instances",
        )),
    }
}

////////////////////////////////////////////// Router //////////////////////////////////////////////

/// Creates an Axum router with component instance management endpoints.
pub fn create_component_instance_router(pool: sqlx::PgPool) -> Router {
    Router::new()
        .route("/component", get(get_all_components))
        .route(
            "/entity/:entity_id/component",
            get(get_components_for_entity).delete(delete_components_for_entity),
        )
        .route(
            "/entity/:entity_id/component/:component_id",
            get(get_component_by_id_for_entity)
                .put(update_component_by_id_for_entity)
                .delete(delete_component_by_id_for_entity),
        )
        .route(
            "/entity/:entity_id/component",
            axum::routing::post(create_component_for_entity),
        )
        .with_state(pool)
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
}
