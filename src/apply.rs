//! Batch apply operations endpoint.
//!
//! This module provides a transactional batch operation endpoint that allows
//! multiple create/update/delete operations to be applied atomically.

use axum::Router;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Json;
use axum::routing::post;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{Component, ComponentDefinition, Entity, InvariantID};

/// A batch operation that can be applied to the system.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Operation {
    /// Creates an entity if it doesn't exist.
    CreateEntity {
        /// Optional entity ID. If None, a random entity will be generated.
        #[serde(skip_serializing_if = "Option::is_none")]
        entity: Option<Entity>,
    },
    /// Deletes an entity and all its components.
    DeleteEntity {
        /// Entity to delete.
        entity: Entity,
    },
    /// Creates or updates a component on an entity.
    UpsertComponent {
        /// Entity to attach component to.
        entity: Entity,
        /// Component type.
        component: Component,
        /// Component data.
        data: Value,
    },
    /// Deletes a component from an entity.
    DeleteComponent {
        /// Entity to delete component from.
        entity: Entity,
        /// Component type to delete.
        component: Component,
    },
    /// Creates or updates a component definition.
    UpsertComponentDefinition {
        /// Component definition to create or update.
        definition: ComponentDefinition,
    },
    /// Deletes a component definition.
    DeleteComponentDefinition {
        /// Component type to delete definition for.
        component: Component,
    },
    /// Creates or updates an invariant.
    UpsertInvariant {
        /// Invariant ID. If None, a random one will be generated.
        #[serde(skip_serializing_if = "Option::is_none")]
        invariant_id: Option<InvariantID>,
        /// The assertion expression.
        asserts: String,
    },
    /// Deletes an invariant.
    DeleteInvariant {
        /// Invariant ID to delete.
        invariant_id: InvariantID,
    },
}

/// Request containing a batch of operations to apply.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplyRequest {
    /// Operations to apply in order.
    pub operations: Vec<Operation>,
}

/// Result of a single operation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OperationResult {
    /// Entity creation result.
    CreateEntity {
        /// The entity that was created or already existed.
        entity: Entity,
        /// True if entity was created, false if it already existed.
        created: bool,
    },
    /// Entity deletion result.
    DeleteEntity {
        /// The entity that was deleted.
        entity: Entity,
        /// True if entity was deleted, false if it didn't exist.
        deleted: bool,
    },
    /// Component upsert result.
    UpsertComponent {
        /// The entity the component was attached to.
        entity: Entity,
        /// The component type.
        component: Component,
        /// True if component was created, false if it was updated.
        created: bool,
    },
    /// Component deletion result.
    DeleteComponent {
        /// The entity the component was deleted from.
        entity: Entity,
        /// The component type.
        component: Component,
        /// True if component was deleted, false if it didn't exist.
        deleted: bool,
    },
    /// Component definition upsert result.
    UpsertComponentDefinition {
        /// The component type.
        component: Component,
        /// True if definition was created, false if it was updated.
        created: bool,
    },
    /// Component definition deletion result.
    DeleteComponentDefinition {
        /// The component type.
        component: Component,
        /// True if definition was deleted, false if it didn't exist.
        deleted: bool,
    },
    /// Invariant upsert result.
    UpsertInvariant {
        /// The invariant ID.
        invariant_id: InvariantID,
        /// The assertion expression.
        asserts: String,
        /// True if invariant was created, false if it was updated.
        created: bool,
    },
    /// Invariant deletion result.
    DeleteInvariant {
        /// The invariant ID.
        invariant_id: InvariantID,
        /// True if invariant was deleted, false if it didn't exist.
        deleted: bool,
    },
    /// Operation error.
    Error {
        /// Index of the operation that failed.
        operation_index: usize,
        /// Error message.
        error: String,
    },
}

/// Response from applying a batch of operations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApplyResponse {
    /// Results for each operation in order.
    pub results: Vec<OperationResult>,
    /// True if transaction was committed, false if rolled back due to errors.
    pub committed: bool,
}

/// Applies a batch of operations transactionally.
///
/// All operations are executed even if an early operation fails. This is
/// intentional to provide complete error feedback in a single request.
/// The transaction will rollback if any operation fails, ensuring atomicity.
///
/// Design rationale: Executing all operations allows clients to see all
/// validation errors and issues in one round-trip, rather than discovering
/// them incrementally. The performance overhead is minimal since all operations
/// occur within the same transaction.
async fn apply_operations(
    State(pool): State<sqlx::PgPool>,
    Json(request): Json<ApplyRequest>,
) -> Result<Json<ApplyResponse>, (StatusCode, String)> {
    let mut tx = pool.begin().await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("failed to begin transaction: {}", e),
        )
    })?;

    let mut results = Vec::new();

    for (idx, operation) in request.operations.iter().enumerate() {
        let result = match operation {
            Operation::CreateEntity { entity } => {
                let entity = entity.unwrap_or_else(|| {
                    Entity::random_url_safe().expect("failed to generate random entity")
                });

                match crate::sql::entity::create_idempotent(&mut tx, &entity).await {
                    Ok(created) => OperationResult::CreateEntity { entity, created },
                    Err(e) => OperationResult::Error {
                        operation_index: idx,
                        error: format!("failed to create entity: {}", e),
                    },
                }
            }
            Operation::DeleteEntity { entity } => {
                match crate::sql::entity::delete(&mut tx, entity).await {
                    Ok(deleted) => OperationResult::DeleteEntity {
                        entity: *entity,
                        deleted,
                    },
                    Err(e) => OperationResult::Error {
                        operation_index: idx,
                        error: format!("failed to delete entity: {}", e),
                    },
                }
            }
            Operation::UpsertComponent {
                entity,
                component,
                data,
            } => match crate::sql::component_definition::get(&mut tx, component).await {
                Ok(Some(def_record)) => {
                    if let Err(e) = def_record.definition.validate_component_data(data) {
                        OperationResult::Error {
                            operation_index: idx,
                            error: format!("component data validation failed: {}", e),
                        }
                    } else {
                        match crate::sql::component::upsert(&mut tx, entity, component, data).await
                        {
                            Ok(created) => OperationResult::UpsertComponent {
                                entity: *entity,
                                component: component.clone(),
                                created,
                            },
                            Err(crate::DataStoreError::NotFound) => OperationResult::Error {
                                operation_index: idx,
                                error: "entity not found".to_string(),
                            },
                            Err(e) => OperationResult::Error {
                                operation_index: idx,
                                error: format!("failed to upsert component: {}", e),
                            },
                        }
                    }
                }
                Ok(None) => OperationResult::Error {
                    operation_index: idx,
                    error: format!("component definition not found: {}", component.as_str()),
                },
                Err(e) => OperationResult::Error {
                    operation_index: idx,
                    error: format!("failed to retrieve component definition: {}", e),
                },
            },
            Operation::DeleteComponent { entity, component } => {
                match crate::sql::component::delete(&mut tx, entity, component).await {
                    Ok(deleted) => OperationResult::DeleteComponent {
                        entity: *entity,
                        component: component.clone(),
                        deleted,
                    },
                    Err(e) => OperationResult::Error {
                        operation_index: idx,
                        error: format!("failed to delete component: {}", e),
                    },
                }
            }
            Operation::UpsertComponentDefinition { definition } => {
                if let Err(e) = definition.validate_schema() {
                    OperationResult::Error {
                        operation_index: idx,
                        error: format!("component definition schema validation failed: {}", e),
                    }
                } else {
                    match crate::sql::component_definition::get(&mut tx, &definition.component)
                        .await
                    {
                        Ok(Some(_)) => {
                            match crate::sql::component_definition::update(&mut tx, definition)
                                .await
                            {
                                Ok(_) => OperationResult::UpsertComponentDefinition {
                                    component: definition.component.clone(),
                                    created: false,
                                },
                                Err(e) => OperationResult::Error {
                                    operation_index: idx,
                                    error: format!("failed to update component definition: {}", e),
                                },
                            }
                        }
                        Ok(None) => {
                            match crate::sql::component_definition::create(&mut tx, definition)
                                .await
                            {
                                Ok(_) => OperationResult::UpsertComponentDefinition {
                                    component: definition.component.clone(),
                                    created: true,
                                },
                                Err(e) => OperationResult::Error {
                                    operation_index: idx,
                                    error: format!("failed to create component definition: {}", e),
                                },
                            }
                        }
                        Err(e) => OperationResult::Error {
                            operation_index: idx,
                            error: format!("failed to check component definition: {}", e),
                        },
                    }
                }
            }
            Operation::DeleteComponentDefinition { component } => {
                match crate::sql::component_definition::delete(&mut tx, component).await {
                    Ok(deleted) => OperationResult::DeleteComponentDefinition {
                        component: component.clone(),
                        deleted,
                    },
                    Err(e) => OperationResult::Error {
                        operation_index: idx,
                        error: format!("failed to delete component definition: {}", e),
                    },
                }
            }
            Operation::UpsertInvariant {
                invariant_id,
                asserts,
            } => {
                let invariant_id = invariant_id.unwrap_or_else(|| {
                    InvariantID::random_url_safe().expect("failed to generate random invariant")
                });

                match crate::sql::invariants::upsert(&mut tx, &invariant_id, asserts).await {
                    Ok(created) => OperationResult::UpsertInvariant {
                        invariant_id,
                        asserts: asserts.clone(),
                        created,
                    },
                    Err(e) => OperationResult::Error {
                        operation_index: idx,
                        error: format!("failed to upsert invariant: {}", e),
                    },
                }
            }
            Operation::DeleteInvariant { invariant_id } => {
                match crate::sql::invariants::delete(&mut tx, invariant_id).await {
                    Ok(deleted) => OperationResult::DeleteInvariant {
                        invariant_id: *invariant_id,
                        deleted,
                    },
                    Err(e) => OperationResult::Error {
                        operation_index: idx,
                        error: format!("failed to delete invariant: {}", e),
                    },
                }
            }
        };
        results.push(result);
    }

    let has_errors = results
        .iter()
        .any(|r| matches!(r, OperationResult::Error { .. }));

    let committed = if has_errors {
        tx.rollback().await.map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed to rollback transaction: {}", e),
            )
        })?;
        false
    } else {
        tx.commit().await.map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed to commit transaction: {}", e),
            )
        })?;
        true
    };

    Ok(Json(ApplyResponse { results, committed }))
}

/// Creates the apply router with batch operation endpoint.
///
/// # Example
///
/// ```no_run
/// # use stigmergy::create_apply_router;
/// # use sqlx::PgPool;
/// # async fn example(pool: PgPool) {
/// let router = create_apply_router(pool);
/// # }
/// ```
pub fn create_apply_router(pool: sqlx::PgPool) -> Router {
    Router::new()
        .route("/apply", post(apply_operations))
        .with_state(pool)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum_test::TestServer;
    use serde_json::json;

    fn unique_entity(test_name: &str) -> Entity {
        use std::time::{SystemTime, UNIX_EPOCH};
        let pid = std::process::id();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_micros() as u64;

        let mut bytes = [0u8; 32];
        bytes[0..4].copy_from_slice(&pid.to_le_bytes());
        bytes[4..12].copy_from_slice(&now.to_le_bytes());

        let test_bytes = test_name.as_bytes();
        let copy_len = test_bytes.len().min(20);
        bytes[12..12 + copy_len].copy_from_slice(&test_bytes[..copy_len]);

        Entity::new(bytes)
    }

    async fn create_test_entity(pool: &sqlx::PgPool, entity: &Entity) {
        let mut tx = pool.begin().await.unwrap();
        crate::sql::entity::create(&mut tx, entity).await.unwrap();
        tx.commit().await.unwrap();
    }

    async fn setup_component_definition(pool: &sqlx::PgPool, component: &Component, schema: Value) {
        let mut tx = pool.begin().await.unwrap();
        let def = crate::ComponentDefinition::new(component.clone(), schema);
        crate::sql::component_definition::create(&mut tx, &def)
            .await
            .unwrap();
        tx.commit().await.unwrap();
    }

    fn simple_object_schema(properties: &[(&str, &str)]) -> Value {
        let props: serde_json::Map<String, Value> = properties
            .iter()
            .map(|(name, type_name)| ((*name).to_string(), json!({"type": type_name})))
            .collect();
        json!({
            "type": "object",
            "properties": props
        })
    }

    #[test]
    fn serialize_operations() {
        let entity = Entity::new([1u8; 32]);
        let component = Component::new("TestComponent").unwrap();

        let ops = vec![
            Operation::CreateEntity { entity: None },
            Operation::CreateEntity {
                entity: Some(entity),
            },
            Operation::DeleteEntity { entity },
            Operation::UpsertComponent {
                entity,
                component: component.clone(),
                data: serde_json::json!({"value": 42}),
            },
            Operation::DeleteComponent { entity, component },
        ];

        let json = serde_json::to_string_pretty(&ops).unwrap();
        println!("Operations JSON:\n{}", json);

        let deserialized: Vec<Operation> = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.len(), 5);
    }

    #[test]
    fn serialize_results() {
        let entity = Entity::new([1u8; 32]);
        let component = Component::new("TestComponent").unwrap();

        let results = vec![
            OperationResult::CreateEntity {
                entity,
                created: true,
            },
            OperationResult::DeleteEntity {
                entity,
                deleted: false,
            },
            OperationResult::UpsertComponent {
                entity,
                component: component.clone(),
                created: true,
            },
            OperationResult::DeleteComponent {
                entity,
                component,
                deleted: false,
            },
            OperationResult::Error {
                operation_index: 4,
                error: "test error".to_string(),
            },
        ];

        let response = ApplyResponse {
            results,
            committed: false,
        };

        let json = serde_json::to_string_pretty(&response).unwrap();
        println!("Response JSON:\n{}", json);

        let deserialized: ApplyResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.results.len(), 5);
        assert!(!deserialized.committed);
    }

    #[tokio::test]
    async fn empty_operations() {
        let pool = crate::sql::tests::setup_test_db().await;
        let router = create_apply_router(pool.clone());
        let server = TestServer::new(router).unwrap();

        let response = server.post("/apply").json(&json!({"operations": []})).await;

        response.assert_status_ok();
        let apply_response: ApplyResponse = response.json();
        println!("empty_operations response: {:?}", apply_response);

        assert_eq!(
            apply_response,
            ApplyResponse {
                results: vec![],
                committed: true
            }
        );
    }

    #[tokio::test]
    async fn create_entity_with_explicit_id() {
        let pool = crate::sql::tests::setup_test_db().await;
        let router = create_apply_router(pool.clone());
        let server = TestServer::new(router).unwrap();

        let entity = unique_entity("create_explicit");

        let response = server
            .post("/apply")
            .json(&json!({
                "operations": [
                    {"type": "create_entity", "entity": entity}
                ]
            }))
            .await;

        response.assert_status_ok();
        let apply_response: ApplyResponse = response.json();
        println!(
            "create_entity_with_explicit_id response: {:?}",
            apply_response
        );

        assert_eq!(
            apply_response,
            ApplyResponse {
                results: vec![OperationResult::CreateEntity {
                    entity,
                    created: true
                }],
                committed: true
            }
        );

        let mut tx = pool.begin().await.unwrap();
        let record = crate::sql::entity::get(&mut tx, &entity).await.unwrap();
        tx.commit().await.unwrap();
        assert!(record.is_some());
    }

    #[tokio::test]
    async fn create_entity_with_generated_id() {
        let pool = crate::sql::tests::setup_test_db().await;
        let router = create_apply_router(pool.clone());
        let server = TestServer::new(router).unwrap();

        let response = server
            .post("/apply")
            .json(&json!({
                "operations": [
                    {"type": "create_entity"}
                ]
            }))
            .await;

        response.assert_status_ok();
        let apply_response: ApplyResponse = response.json();
        println!(
            "create_entity_with_generated_id response: {:?}",
            apply_response
        );

        assert!(apply_response.committed);
        assert_eq!(apply_response.results.len(), 1);
        match &apply_response.results[0] {
            OperationResult::CreateEntity { entity, created } => {
                assert!(*created);
                let mut tx = pool.begin().await.unwrap();
                let record = crate::sql::entity::get(&mut tx, entity).await.unwrap();
                tx.commit().await.unwrap();
                assert!(record.is_some());
            }
            _ => panic!("Expected CreateEntity result"),
        }
    }

    #[tokio::test]
    async fn create_entity_idempotent() {
        let pool = crate::sql::tests::setup_test_db().await;
        let router = create_apply_router(pool.clone());
        let server = TestServer::new(router).unwrap();

        let entity = unique_entity("create_idempotent");
        create_test_entity(&pool, &entity).await;

        let response = server
            .post("/apply")
            .json(&json!({
                "operations": [
                    {"type": "create_entity", "entity": entity}
                ]
            }))
            .await;

        response.assert_status_ok();
        let apply_response: ApplyResponse = response.json();
        println!("create_entity_idempotent response: {:?}", apply_response);

        assert_eq!(
            apply_response,
            ApplyResponse {
                results: vec![OperationResult::CreateEntity {
                    entity,
                    created: false
                }],
                committed: true
            }
        );
    }

    #[tokio::test]
    async fn delete_existing_entity() {
        let pool = crate::sql::tests::setup_test_db().await;
        let router = create_apply_router(pool.clone());
        let server = TestServer::new(router).unwrap();

        let entity = unique_entity("delete_existing");
        create_test_entity(&pool, &entity).await;

        let response = server
            .post("/apply")
            .json(&json!({
                "operations": [
                    {"type": "delete_entity", "entity": entity}
                ]
            }))
            .await;

        response.assert_status_ok();
        let apply_response: ApplyResponse = response.json();
        println!("delete_existing_entity response: {:?}", apply_response);

        assert_eq!(
            apply_response,
            ApplyResponse {
                results: vec![OperationResult::DeleteEntity {
                    entity,
                    deleted: true
                }],
                committed: true
            }
        );

        let mut tx = pool.begin().await.unwrap();
        let record = crate::sql::entity::get(&mut tx, &entity).await.unwrap();
        tx.commit().await.unwrap();
        assert!(record.is_none());
    }

    #[tokio::test]
    async fn delete_nonexistent_entity() {
        let pool = crate::sql::tests::setup_test_db().await;
        let router = create_apply_router(pool.clone());
        let server = TestServer::new(router).unwrap();

        let entity = unique_entity("delete_nonexistent");

        let response = server
            .post("/apply")
            .json(&json!({
                "operations": [
                    {"type": "delete_entity", "entity": entity}
                ]
            }))
            .await;

        response.assert_status_ok();
        let apply_response: ApplyResponse = response.json();
        println!("delete_nonexistent_entity response: {:?}", apply_response);

        assert_eq!(
            apply_response,
            ApplyResponse {
                results: vec![OperationResult::DeleteEntity {
                    entity,
                    deleted: false
                }],
                committed: true
            }
        );
    }

    #[tokio::test]
    async fn upsert_component_creates_new() {
        let pool = crate::sql::tests::setup_test_db().await;
        let router = create_apply_router(pool.clone());
        let server = TestServer::new(router).unwrap();

        let entity = unique_entity("upsert_creates");
        let component = Component::new("Health").unwrap();
        let schema = simple_object_schema(&[("hp", "number")]);
        let data = json!({"hp": 100});

        create_test_entity(&pool, &entity).await;
        setup_component_definition(&pool, &component, schema).await;

        let response = server
            .post("/apply")
            .json(&json!({
                "operations": [
                    {
                        "type": "upsert_component",
                        "entity": entity,
                        "component": component,
                        "data": data
                    }
                ]
            }))
            .await;

        response.assert_status_ok();
        let apply_response: ApplyResponse = response.json();
        println!(
            "upsert_component_creates_new response: {:?}",
            apply_response
        );

        assert_eq!(
            apply_response,
            ApplyResponse {
                results: vec![OperationResult::UpsertComponent {
                    entity,
                    component: component.clone(),
                    created: true
                }],
                committed: true
            }
        );

        let mut tx = pool.begin().await.unwrap();
        let retrieved = crate::sql::component::get(&mut tx, &entity, &component)
            .await
            .unwrap();
        tx.commit().await.unwrap();
        assert_eq!(retrieved, Some(data));
    }

    #[tokio::test]
    async fn upsert_component_updates_existing() {
        let pool = crate::sql::tests::setup_test_db().await;
        let router = create_apply_router(pool.clone());
        let server = TestServer::new(router).unwrap();

        let entity = unique_entity("upsert_updates");
        let component = Component::new("Score").unwrap();
        let schema = simple_object_schema(&[("points", "number")]);
        let initial_data = json!({"points": 100});
        let updated_data = json!({"points": 200});

        create_test_entity(&pool, &entity).await;
        setup_component_definition(&pool, &component, schema).await;

        let mut tx = pool.begin().await.unwrap();
        crate::sql::component::create(&mut tx, &entity, &component, &initial_data)
            .await
            .unwrap();
        tx.commit().await.unwrap();

        let response = server
            .post("/apply")
            .json(&json!({
                "operations": [
                    {
                        "type": "upsert_component",
                        "entity": entity,
                        "component": component,
                        "data": updated_data
                    }
                ]
            }))
            .await;

        response.assert_status_ok();
        let apply_response: ApplyResponse = response.json();
        println!(
            "upsert_component_updates_existing response: {:?}",
            apply_response
        );

        assert!(apply_response.committed);
        assert_eq!(apply_response.results.len(), 1);

        match &apply_response.results[0] {
            OperationResult::UpsertComponent {
                entity: result_entity,
                component: result_component,
                created,
            } => {
                assert_eq!(*result_entity, entity);
                assert_eq!(*result_component, component);
                assert!(!created);
            }
            r => panic!("Expected UpsertComponent with created=false, got: {:?}", r),
        }

        let mut tx = pool.begin().await.unwrap();
        let retrieved = crate::sql::component::get(&mut tx, &entity, &component)
            .await
            .unwrap();
        tx.commit().await.unwrap();
        assert_eq!(retrieved, Some(updated_data));
    }

    #[tokio::test]
    async fn upsert_component_entity_not_found() {
        let pool = crate::sql::tests::setup_test_db().await;
        let router = create_apply_router(pool.clone());
        let server = TestServer::new(router).unwrap();

        let entity = unique_entity("upsert_no_entity");
        let component = Component::new("Invalid").unwrap();
        let schema = simple_object_schema(&[("value", "number")]);
        let data = json!({"value": 42});

        setup_component_definition(&pool, &component, schema).await;

        let response = server
            .post("/apply")
            .json(&json!({
                "operations": [
                    {
                        "type": "upsert_component",
                        "entity": entity,
                        "component": component,
                        "data": data
                    }
                ]
            }))
            .await;

        response.assert_status_ok();
        let apply_response: ApplyResponse = response.json();
        println!(
            "upsert_component_entity_not_found response: {:?}",
            apply_response
        );

        assert!(!apply_response.committed);
        assert_eq!(apply_response.results.len(), 1);
        match &apply_response.results[0] {
            OperationResult::Error {
                operation_index,
                error,
            } => {
                assert_eq!(*operation_index, 0);
                assert_eq!(*error, "entity not found");
            }
            _ => panic!("Expected Error result"),
        }
    }

    #[tokio::test]
    async fn upsert_component_definition_not_found() {
        let pool = crate::sql::tests::setup_test_db().await;
        let router = create_apply_router(pool.clone());
        let server = TestServer::new(router).unwrap();

        let entity = unique_entity("upsert_no_def");
        let component = Component::new("Undefined").unwrap();
        let data = json!({"value": 42});

        create_test_entity(&pool, &entity).await;

        let response = server
            .post("/apply")
            .json(&json!({
                "operations": [
                    {
                        "type": "upsert_component",
                        "entity": entity,
                        "component": component,
                        "data": data
                    }
                ]
            }))
            .await;

        response.assert_status_ok();
        let apply_response: ApplyResponse = response.json();
        println!(
            "upsert_component_definition_not_found response: {:?}",
            apply_response
        );

        assert!(!apply_response.committed);
        assert_eq!(apply_response.results.len(), 1);
        match &apply_response.results[0] {
            OperationResult::Error {
                operation_index,
                error,
            } => {
                assert_eq!(*operation_index, 0);
                assert!(error.contains("component definition not found"));
            }
            _ => panic!("Expected Error result"),
        }
    }

    #[tokio::test]
    async fn upsert_component_validation_failure() {
        let pool = crate::sql::tests::setup_test_db().await;
        let router = create_apply_router(pool.clone());
        let server = TestServer::new(router).unwrap();

        let entity = unique_entity("upsert_invalid_data");
        let component = Component::new("Validated").unwrap();
        let schema = json!({
            "type": "object",
            "properties": {
                "required_number": {"type": "number"}
            },
            "required": ["required_number"]
        });
        let invalid_data = json!({"wrong_field": "string"});

        let mut tx = pool.begin().await.unwrap();
        crate::sql::entity::create(&mut tx, &entity).await.unwrap();
        tx.commit().await.unwrap();

        setup_component_definition(&pool, &component, schema).await;

        let response = server
            .post("/apply")
            .json(&json!({
                "operations": [
                    {
                        "type": "upsert_component",
                        "entity": entity,
                        "component": component,
                        "data": invalid_data
                    }
                ]
            }))
            .await;

        response.assert_status_ok();
        let apply_response: ApplyResponse = response.json();
        println!(
            "upsert_component_validation_failure response: {:?}",
            apply_response
        );

        assert!(!apply_response.committed);
        assert_eq!(apply_response.results.len(), 1);
        match &apply_response.results[0] {
            OperationResult::Error {
                operation_index,
                error,
            } => {
                assert_eq!(*operation_index, 0);
                assert!(error.contains("component data validation failed"));
            }
            _ => panic!("Expected Error result"),
        }
    }

    #[tokio::test]
    async fn delete_existing_component() {
        let pool = crate::sql::tests::setup_test_db().await;
        let router = create_apply_router(pool.clone());
        let server = TestServer::new(router).unwrap();

        let entity = unique_entity("delete_comp_existing");
        let component = Component::new("Tag").unwrap();
        let schema = simple_object_schema(&[("label", "string")]);
        let data = json!({"label": "test"});

        create_test_entity(&pool, &entity).await;
        setup_component_definition(&pool, &component, schema).await;

        let mut tx = pool.begin().await.unwrap();
        crate::sql::component::create(&mut tx, &entity, &component, &data)
            .await
            .unwrap();
        tx.commit().await.unwrap();

        let response = server
            .post("/apply")
            .json(&json!({
                "operations": [
                    {
                        "type": "delete_component",
                        "entity": entity,
                        "component": component
                    }
                ]
            }))
            .await;

        response.assert_status_ok();
        let apply_response: ApplyResponse = response.json();
        println!("delete_existing_component response: {:?}", apply_response);

        assert_eq!(
            apply_response,
            ApplyResponse {
                results: vec![OperationResult::DeleteComponent {
                    entity,
                    component: component.clone(),
                    deleted: true
                }],
                committed: true
            }
        );

        let mut tx = pool.begin().await.unwrap();
        let retrieved = crate::sql::component::get(&mut tx, &entity, &component)
            .await
            .unwrap();
        tx.commit().await.unwrap();
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn delete_nonexistent_component() {
        let pool = crate::sql::tests::setup_test_db().await;
        let router = create_apply_router(pool.clone());
        let server = TestServer::new(router).unwrap();

        let entity = unique_entity("delete_comp_nonexistent");
        let component = Component::new("NotThere").unwrap();

        create_test_entity(&pool, &entity).await;

        let response = server
            .post("/apply")
            .json(&json!({
                "operations": [
                    {
                        "type": "delete_component",
                        "entity": entity,
                        "component": component
                    }
                ]
            }))
            .await;

        response.assert_status_ok();
        let apply_response: ApplyResponse = response.json();
        println!(
            "delete_nonexistent_component response: {:?}",
            apply_response
        );

        assert_eq!(
            apply_response,
            ApplyResponse {
                results: vec![OperationResult::DeleteComponent {
                    entity,
                    component,
                    deleted: false
                }],
                committed: true
            }
        );
    }

    #[tokio::test]
    async fn batch_all_success_commits() {
        let pool = crate::sql::tests::setup_test_db().await;
        let router = create_apply_router(pool.clone());
        let server = TestServer::new(router).unwrap();

        let entity1 = unique_entity("batch_success_1");
        let entity2 = unique_entity("batch_success_2");
        let component = Component::new("Position").unwrap();
        let schema = simple_object_schema(&[("x", "number"), ("y", "number")]);
        let data = json!({"x": 1.0, "y": 2.0});

        setup_component_definition(&pool, &component, schema).await;

        let response = server
            .post("/apply")
            .json(&json!({
                "operations": [
                    {"type": "create_entity", "entity": entity1},
                    {"type": "create_entity", "entity": entity2},
                    {
                        "type": "upsert_component",
                        "entity": entity1,
                        "component": component,
                        "data": data
                    }
                ]
            }))
            .await;

        response.assert_status_ok();
        let apply_response: ApplyResponse = response.json();
        println!("batch_all_success_commits response: {:?}", apply_response);

        assert!(apply_response.committed);
        assert_eq!(apply_response.results.len(), 3);

        let mut tx = pool.begin().await.unwrap();
        assert!(
            crate::sql::entity::get(&mut tx, &entity1)
                .await
                .unwrap()
                .is_some()
        );
        assert!(
            crate::sql::entity::get(&mut tx, &entity2)
                .await
                .unwrap()
                .is_some()
        );
        assert!(
            crate::sql::component::get(&mut tx, &entity1, &component)
                .await
                .unwrap()
                .is_some()
        );
        tx.commit().await.unwrap();
    }

    #[tokio::test]
    async fn batch_with_error_rolls_back() {
        let pool = crate::sql::tests::setup_test_db().await;
        let router = create_apply_router(pool.clone());
        let server = TestServer::new(router).unwrap();

        let entity = unique_entity("batch_rollback");
        let component = Component::new("Invalid").unwrap();
        let data = json!({"value": 42});

        let response = server
            .post("/apply")
            .json(&json!({
                "operations": [
                    {"type": "create_entity", "entity": entity},
                    {
                        "type": "upsert_component",
                        "entity": entity,
                        "component": component,
                        "data": data
                    }
                ]
            }))
            .await;

        response.assert_status_ok();
        let apply_response: ApplyResponse = response.json();
        println!("batch_with_error_rolls_back response: {:?}", apply_response);

        assert!(!apply_response.committed);
        assert_eq!(apply_response.results.len(), 2);
        match &apply_response.results[0] {
            OperationResult::CreateEntity { created, .. } => assert!(*created),
            _ => panic!("Expected CreateEntity result"),
        }
        match &apply_response.results[1] {
            OperationResult::Error {
                operation_index, ..
            } => {
                assert_eq!(*operation_index, 1);
            }
            _ => panic!("Expected Error result"),
        }

        let mut tx = pool.begin().await.unwrap();
        let record = crate::sql::entity::get(&mut tx, &entity).await.unwrap();
        tx.commit().await.unwrap();
        assert!(record.is_none());
    }

    #[tokio::test]
    async fn mixed_operations_complex_scenario() {
        let pool = crate::sql::tests::setup_test_db().await;
        let router = create_apply_router(pool.clone());
        let server = TestServer::new(router).unwrap();

        let entity1 = unique_entity("mixed_1");
        let entity2 = unique_entity("mixed_2");
        let entity3 = unique_entity("mixed_3");
        let comp1 = Component::new("Health").unwrap();
        let comp2 = Component::new("Position").unwrap();
        let schema1 = simple_object_schema(&[("hp", "number")]);
        let schema2 = simple_object_schema(&[("x", "number")]);
        let data1 = json!({"hp": 100});
        let data2 = json!({"x": 5.0});

        create_test_entity(&pool, &entity2).await;
        setup_component_definition(&pool, &comp1, schema1).await;
        setup_component_definition(&pool, &comp2, schema2).await;

        let mut tx = pool.begin().await.unwrap();
        crate::sql::component::create(&mut tx, &entity2, &comp1, &data1)
            .await
            .unwrap();
        tx.commit().await.unwrap();

        let response = server
            .post("/apply")
            .json(&json!({
                "operations": [
                    {"type": "create_entity", "entity": entity1},
                    {"type": "create_entity", "entity": entity2},
                    {"type": "create_entity"},
                    {
                        "type": "upsert_component",
                        "entity": entity1,
                        "component": comp1,
                        "data": data1
                    },
                    {
                        "type": "upsert_component",
                        "entity": entity2,
                        "component": comp2,
                        "data": data2
                    },
                    {
                        "type": "delete_component",
                        "entity": entity2,
                        "component": comp1
                    },
                    {"type": "delete_entity", "entity": entity3}
                ]
            }))
            .await;

        response.assert_status_ok();
        let apply_response: ApplyResponse = response.json();
        println!(
            "mixed_operations_complex_scenario response: {:?}",
            apply_response
        );

        assert!(apply_response.committed);
        assert_eq!(apply_response.results.len(), 7);

        match &apply_response.results[0] {
            OperationResult::CreateEntity {
                entity,
                created: true,
            } => assert_eq!(*entity, entity1),
            r => panic!(
                "Expected CreateEntity result with created=true, got: {:?}",
                r
            ),
        }

        match &apply_response.results[1] {
            OperationResult::CreateEntity {
                entity,
                created: false,
            } => assert_eq!(*entity, entity2),
            r => panic!(
                "Expected CreateEntity result with created=false, got: {:?}",
                r
            ),
        }

        match &apply_response.results[2] {
            OperationResult::CreateEntity { created: true, .. } => {}
            r => panic!(
                "Expected CreateEntity result with created=true, got: {:?}",
                r
            ),
        }

        match &apply_response.results[3] {
            OperationResult::UpsertComponent {
                entity,
                component,
                created: true,
            } => {
                assert_eq!(*entity, entity1);
                assert_eq!(*component, comp1);
            }
            r => panic!(
                "Expected UpsertComponent result with created=true, got: {:?}",
                r
            ),
        }

        match &apply_response.results[4] {
            OperationResult::UpsertComponent {
                entity, component, ..
            } => {
                assert_eq!(*entity, entity2);
                assert_eq!(*component, comp2);
            }
            r => panic!("Expected UpsertComponent result, got: {:?}", r),
        }

        match &apply_response.results[5] {
            OperationResult::DeleteComponent {
                entity,
                component,
                deleted: true,
            } => {
                assert_eq!(*entity, entity2);
                assert_eq!(*component, comp1);
            }
            r => panic!(
                "Expected DeleteComponent result with deleted=true, got: {:?}",
                r
            ),
        }

        match &apply_response.results[6] {
            OperationResult::DeleteEntity {
                entity,
                deleted: false,
            } => assert_eq!(*entity, entity3),
            r => panic!(
                "Expected DeleteEntity result with deleted=false, got: {:?}",
                r
            ),
        }
    }

    #[tokio::test]
    async fn large_batch_stress_test() {
        let pool = crate::sql::tests::setup_test_db().await;
        let router = create_apply_router(pool.clone());
        let server = TestServer::new(router).unwrap();

        let mut operations = Vec::new();
        let base_name = format!("stress_test_{}", std::process::id());

        for i in 0u32..50 {
            let mut bytes = [0u8; 32];
            bytes[0..4].copy_from_slice(&i.to_le_bytes());
            let test_bytes = base_name.as_bytes();
            let copy_len = test_bytes.len().min(28);
            bytes[4..4 + copy_len].copy_from_slice(&test_bytes[..copy_len]);

            operations.push(json!({
                "type": "create_entity",
                "entity": Entity::new(bytes)
            }));
        }

        let response = server
            .post("/apply")
            .json(&json!({"operations": operations}))
            .await;

        response.assert_status_ok();
        let apply_response: ApplyResponse = response.json();
        println!(
            "large_batch_stress_test response length: {}",
            apply_response.results.len()
        );

        assert!(apply_response.committed);
        assert_eq!(apply_response.results.len(), 50);

        for result in &apply_response.results {
            match result {
                OperationResult::CreateEntity { created, .. } => assert!(*created),
                r => panic!("Expected CreateEntity result, got: {:?}", r),
            }
        }
    }
}
