use axum::{
    Router,
    routing::{delete, get, patch, post, put},
};

use crate::component::{
    create_component, create_component_definition, delete_component_by_id,
    delete_component_definition_by_id, delete_component_definitions, delete_components,
    get_component_by_id, get_component_definition_by_id, get_component_definitions, get_components,
    patch_component, patch_component_by_id, patch_component_definition,
    patch_component_definition_by_id, update_component, update_component_by_id,
    update_component_definition, update_component_definition_by_id,
};

pub fn create_router() -> Router {
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
