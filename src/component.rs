use axum::{
    extract::{Path, Query},
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentDefinition {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub schema: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Component {
    pub id: String,
    pub definition_id: String,
    pub name: String,
    pub data: serde_json::Value,
}

pub async fn get_component_definitions(
    Query(_params): Query<HashMap<String, String>>,
) -> Result<Json<Vec<ComponentDefinition>>, StatusCode> {
    Ok(Json(vec![]))
}

pub async fn create_component_definition(
    Json(definition): Json<ComponentDefinition>,
) -> Result<Json<ComponentDefinition>, StatusCode> {
    Ok(Json(definition))
}

pub async fn update_component_definition(
    Json(definition): Json<ComponentDefinition>,
) -> Result<Json<ComponentDefinition>, StatusCode> {
    Ok(Json(definition))
}

pub async fn patch_component_definition(
    Json(patch): Json<Value>,
) -> Result<Json<ComponentDefinition>, StatusCode> {
    let definition = ComponentDefinition {
        id: "patched".to_string(),
        name: "Patched Definition".to_string(),
        description: Some("Patched via PATCH".to_string()),
        schema: patch,
    };
    Ok(Json(definition))
}

pub async fn delete_component_definitions() -> Result<StatusCode, StatusCode> {
    Ok(StatusCode::NO_CONTENT)
}

pub async fn get_component_definition_by_id(
    Path(id): Path<String>,
) -> Result<Json<ComponentDefinition>, StatusCode> {
    let definition = ComponentDefinition {
        id: id.clone(),
        name: format!("Definition {}", id),
        description: Some("A component definition".to_string()),
        schema: serde_json::json!({}),
    };
    Ok(Json(definition))
}

pub async fn update_component_definition_by_id(
    Path(id): Path<String>,
    Json(mut definition): Json<ComponentDefinition>,
) -> Result<Json<ComponentDefinition>, StatusCode> {
    definition.id = id;
    Ok(Json(definition))
}

pub async fn patch_component_definition_by_id(
    Path(id): Path<String>,
    Json(patch): Json<Value>,
) -> Result<Json<ComponentDefinition>, StatusCode> {
    let definition = ComponentDefinition {
        id,
        name: "Patched Definition".to_string(),
        description: Some("Patched via PATCH with ID".to_string()),
        schema: patch,
    };
    Ok(Json(definition))
}

pub async fn delete_component_definition_by_id(
    Path(_id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    Ok(StatusCode::NO_CONTENT)
}

pub async fn get_components(
    Query(_params): Query<HashMap<String, String>>,
) -> Result<Json<Vec<Component>>, StatusCode> {
    Ok(Json(vec![]))
}

pub async fn create_component(
    Json(component): Json<Component>,
) -> Result<Json<Component>, StatusCode> {
    Ok(Json(component))
}

pub async fn update_component(
    Json(component): Json<Component>,
) -> Result<Json<Component>, StatusCode> {
    Ok(Json(component))
}

pub async fn patch_component(Json(patch): Json<Value>) -> Result<Json<Component>, StatusCode> {
    let component = Component {
        id: "patched".to_string(),
        definition_id: "def1".to_string(),
        name: "Patched Component".to_string(),
        data: patch,
    };
    Ok(Json(component))
}

pub async fn delete_components() -> Result<StatusCode, StatusCode> {
    Ok(StatusCode::NO_CONTENT)
}

pub async fn get_component_by_id(Path(id): Path<String>) -> Result<Json<Component>, StatusCode> {
    let component = Component {
        id: id.clone(),
        definition_id: "def1".to_string(),
        name: format!("Component {}", id),
        data: serde_json::json!({}),
    };
    Ok(Json(component))
}

pub async fn update_component_by_id(
    Path(id): Path<String>,
    Json(mut component): Json<Component>,
) -> Result<Json<Component>, StatusCode> {
    component.id = id;
    Ok(Json(component))
}

pub async fn patch_component_by_id(
    Path(id): Path<String>,
    Json(patch): Json<Value>,
) -> Result<Json<Component>, StatusCode> {
    let component = Component {
        id,
        definition_id: "def1".to_string(),
        name: "Patched Component".to_string(),
        data: patch,
    };
    Ok(Json(component))
}

pub async fn delete_component_by_id(Path(_id): Path<String>) -> Result<StatusCode, StatusCode> {
    Ok(StatusCode::NO_CONTENT)
}
