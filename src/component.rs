use std::collections::HashMap;

use axum::Router;
use axum::extract::{Path, Query};
use axum::http::StatusCode;
use axum::response::Json;
use axum::routing::get;
use serde::{Deserialize, Serialize};
use serde_json::Value;

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

////////////////////////////////////////////// routes //////////////////////////////////////////////

async fn get_component_definitions(
    Query(_params): Query<HashMap<String, String>>,
) -> Result<Json<Vec<ComponentDefinition>>, StatusCode> {
    Ok(Json(vec![]))
}

async fn create_component_definition(
    Json(definition): Json<ComponentDefinition>,
) -> Result<Json<ComponentDefinition>, StatusCode> {
    Ok(Json(definition))
}

async fn update_component_definition(
    Json(definition): Json<ComponentDefinition>,
) -> Result<Json<ComponentDefinition>, StatusCode> {
    Ok(Json(definition))
}

async fn patch_component_definition(
    Json(patch): Json<Value>,
) -> Result<Json<ComponentDefinition>, StatusCode> {
    let component = Component::new("PatchedComponent").unwrap();
    let definition = ComponentDefinition {
        component,
        schema: patch,
    };
    Ok(Json(definition))
}

async fn delete_component_definitions() -> Result<StatusCode, StatusCode> {
    Ok(StatusCode::NO_CONTENT)
}

async fn get_component_definition_by_id(
    Path(id): Path<String>,
) -> Result<Json<ComponentDefinition>, StatusCode> {
    let component = Component::new(format!("Component{}", id))
        .unwrap_or_else(|| Component::new("DefaultComponent").unwrap());
    let definition = ComponentDefinition {
        component,
        schema: serde_json::json!({}),
    };
    Ok(Json(definition))
}

async fn update_component_definition_by_id(
    Path(_id): Path<String>,
    Json(definition): Json<ComponentDefinition>,
) -> Result<Json<ComponentDefinition>, StatusCode> {
    Ok(Json(definition))
}

async fn patch_component_definition_by_id(
    Path(id): Path<String>,
    Json(patch): Json<Value>,
) -> Result<Json<ComponentDefinition>, StatusCode> {
    let component = Component::new(format!("Component{}", id))
        .unwrap_or_else(|| Component::new("PatchedComponent").unwrap());
    let definition = ComponentDefinition {
        component,
        schema: patch,
    };
    Ok(Json(definition))
}

async fn delete_component_definition_by_id(
    Path(_id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    Ok(StatusCode::NO_CONTENT)
}

async fn get_components(
    Query(_params): Query<HashMap<String, String>>,
) -> Result<Json<Vec<Value>>, StatusCode> {
    Ok(Json(vec![]))
}

async fn create_component(Json(component): Json<Value>) -> Result<Json<Value>, StatusCode> {
    Ok(Json(component))
}

async fn update_component(Json(component): Json<Value>) -> Result<Json<Value>, StatusCode> {
    Ok(Json(component))
}

async fn patch_component(Json(patch): Json<Value>) -> Result<Json<Value>, StatusCode> {
    Ok(Json(patch))
}

async fn delete_components() -> Result<StatusCode, StatusCode> {
    Ok(StatusCode::NO_CONTENT)
}

async fn get_component_by_id(Path(id): Path<String>) -> Result<Json<Value>, StatusCode> {
    let component = serde_json::json!({
        "id": id,
        "data": {}
    });
    Ok(Json(component))
}

async fn update_component_by_id(
    Path(_id): Path<String>,
    Json(component): Json<Value>,
) -> Result<Json<Value>, StatusCode> {
    Ok(Json(component))
}

async fn patch_component_by_id(
    Path(id): Path<String>,
    Json(patch): Json<Value>,
) -> Result<Json<Value>, StatusCode> {
    let mut component = patch;
    if let Some(obj) = component.as_object_mut() {
        obj.insert("id".to_string(), serde_json::Value::String(id));
    }
    Ok(Json(component))
}

async fn delete_component_by_id(Path(_id): Path<String>) -> Result<StatusCode, StatusCode> {
    Ok(StatusCode::NO_CONTENT)
}

////////////////////////////////////////////// router //////////////////////////////////////////////

pub fn create_component_router() -> Router {
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
