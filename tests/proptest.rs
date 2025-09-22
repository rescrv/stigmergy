use std::fs;
use std::path::PathBuf;
use std::process;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use axum::Router;
use axum_test::TestServer;
use proptest::prelude::*;
use reqwest::StatusCode;
use serde_json::{Value, json};

use stigmergy::{
    Component, ComponentDefinition, CreateComponentRequest, CreateEntityRequest, DurableLogger,
    Entity, InMemoryDataStore, create_component_router, create_entity_router,
};

/// Test infrastructure for property testing the stigmergy API
pub struct ApiTestServer {
    pub server: TestServer,
    pub data_store: Arc<InMemoryDataStore>,
    pub logger: Arc<DurableLogger>,
    pub log_path: PathBuf,
}

impl Default for ApiTestServer {
    fn default() -> Self {
        Self::new()
    }
}

impl ApiTestServer {
    /// Create a new test server with fresh in-memory data store and logger
    pub fn new() -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let pid = process::id();
        let log_path = PathBuf::from(format!("prop_test_{}_{}.jsonl", pid, timestamp));

        let logger = Arc::new(DurableLogger::new(log_path.clone()));
        let data_store = Arc::new(InMemoryDataStore::new());

        let app = Router::new()
            .nest(
                "/api/v1",
                create_entity_router(logger.clone(), data_store.clone()),
            )
            .nest(
                "/api/v1",
                create_component_router(logger.clone(), data_store.clone()),
            );

        let server = TestServer::new(app).unwrap();

        Self {
            server,
            data_store: data_store.clone(),
            logger,
            log_path,
        }
    }
}

impl Drop for ApiTestServer {
    fn drop(&mut self) {
        if let Err(e) = fs::remove_file(&self.log_path) {
            eprintln!(
                "Warning: failed to cleanup test log file {:?}: {}",
                self.log_path, e
            );
        }
    }
}

/// Property test strategies for generating test data
pub mod strategies {
    use super::*;
    use proptest::collection::hash_map;
    use proptest::option;
    use proptest::string::string_regex;

    /// Strategy for generating valid Entity instances
    pub fn entity_strategy() -> impl Strategy<Value = Entity> {
        any::<[u8; 32]>().prop_map(Entity::new)
    }

    /// Strategy for generating valid Component names
    pub fn component_name_strategy() -> impl Strategy<Value = String> {
        // Component names must be valid Rust identifiers
        string_regex(r"[A-Za-z][A-Za-z0-9_]{0,63}").unwrap()
    }

    /// Strategy for generating valid Component instances
    pub fn component_strategy() -> impl Strategy<Value = Component> {
        component_name_strategy().prop_filter_map("valid component", Component::new)
    }

    /// Strategy for generating JSON values that we can then create schemas for
    pub fn json_value_strategy() -> impl Strategy<Value = Value> {
        prop_oneof![
            // Simple values
            any::<String>().prop_map(Value::String),
            any::<f64>().prop_map(|f| json!(f)),
            any::<i64>().prop_map(|i| json!(i)),
            any::<bool>().prop_map(Value::Bool),
            Just(Value::Null),
            // Simple objects with 1-3 string properties
            hash_map(
                string_regex(r"[a-zA-Z][a-zA-Z0-9_]*").unwrap(),
                prop_oneof![
                    any::<String>().prop_map(Value::String),
                    any::<f64>().prop_map(|f| json!(f)),
                    any::<bool>().prop_map(Value::Bool),
                ],
                1..4
            )
            .prop_map(|map| json!(map))
        ]
    }

    /// Create a JSON schema that matches a given JSON value
    pub fn schema_for_value(value: &Value) -> Value {
        match value {
            Value::Null => json!({"type": "null"}),
            Value::Bool(_) => json!({"type": "boolean"}),
            Value::Number(n) => {
                if n.is_i64() {
                    json!({"type": "integer"})
                } else {
                    json!({"type": "number"})
                }
            }
            Value::String(_) => json!({"type": "string"}),
            Value::Array(arr) => {
                if arr.is_empty() {
                    json!({"type": "array"})
                } else {
                    // Use schema of first element for all items
                    json!({
                        "type": "array",
                        "items": schema_for_value(&arr[0])
                    })
                }
            }
            Value::Object(obj) => {
                let properties: serde_json::Map<String, Value> = obj
                    .iter()
                    .map(|(k, v)| (k.clone(), schema_for_value(v)))
                    .collect();

                let required: Vec<String> = obj.keys().cloned().collect();

                json!({
                    "type": "object",
                    "properties": properties,
                    "required": required
                })
            }
        }
    }

    /// Strategy for generating (ComponentDefinition, matching_data) pairs
    pub fn component_definition_with_data_strategy()
    -> impl Strategy<Value = (ComponentDefinition, Value)> {
        (component_strategy(), json_value_strategy()).prop_map(|(component, data)| {
            let schema = schema_for_value(&data);
            let definition = ComponentDefinition::new(component, schema);
            (definition, data)
        })
    }

    /// Strategy for generating ComponentDefinition instances
    pub fn component_definition_strategy() -> impl Strategy<Value = ComponentDefinition> {
        component_definition_with_data_strategy().prop_map(|(definition, _data)| definition)
    }

    /// Strategy for generating data that matches a simple schema
    pub fn data_for_schema_strategy(schema: &Value) -> BoxedStrategy<Value> {
        match schema {
            schema if schema == &json!({"type": "string"}) => {
                any::<String>().prop_map(Value::String).boxed()
            }
            schema if schema == &json!({"type": "number"}) => {
                any::<f64>().prop_map(|n| json!(n)).boxed()
            }
            schema if schema == &json!({"type": "integer"}) => {
                any::<i64>().prop_map(|n| json!(n)).boxed()
            }
            schema if schema == &json!({"type": "boolean"}) => {
                any::<bool>().prop_map(Value::Bool).boxed()
            }
            schema if schema == &json!({"type": "null"}) => Just(Value::Null).boxed(),
            // For complex schemas, generate simple string data as fallback
            _ => any::<String>().prop_map(Value::String).boxed(),
        }
    }

    /// Strategy for generating CreateEntityRequest instances
    pub fn create_entity_request_strategy() -> impl Strategy<Value = CreateEntityRequest> {
        option::of(entity_strategy()).prop_map(|entity| CreateEntityRequest { entity })
    }

    /// Strategy for generating CreateComponentRequest instances
    pub fn create_component_request_strategy(
        schema: Value,
    ) -> BoxedStrategy<CreateComponentRequest> {
        (component_strategy(), data_for_schema_strategy(&schema))
            .prop_map(|(component, data)| CreateComponentRequest { component, data })
            .boxed()
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(10))]

    #[test]
    fn entity_creation_roundtrip(
        request in strategies::create_entity_request_strategy()
    ) {
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            let test_server = ApiTestServer::new();

            // Create entity
            let response = test_server.server
                .post("/api/v1/entity")
                .json(&request)
                .await;

            response.assert_status_ok();
            let created: stigmergy::CreateEntityResponse = response.json();

            // Verify entity was created
            let entity_list = test_server.server
                .get("/api/v1/entity")
                .await;

            entity_list.assert_status_ok();
            let entities: Vec<Entity> = entity_list.json();

            prop_assert!(entities.contains(&created.entity));
            prop_assert_eq!(created.created, true);
            Ok(())
        }).unwrap()
    }

    #[test]
    fn entity_deletion_after_creation(
        request in strategies::create_entity_request_strategy()
    ) {
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            let test_server = ApiTestServer::new();

            // Create entity
            let create_response = test_server.server
                .post("/api/v1/entity")
                .json(&request)
                .await;

            create_response.assert_status_ok();
            let created: stigmergy::CreateEntityResponse = create_response.json();

            // Delete entity
            let entity_str = created.entity.to_string();
            let entity_base64 = entity_str.strip_prefix("entity:").unwrap();
            let delete_response = test_server.server
                .delete(&format!("/api/v1/entity/{}", entity_base64))
                .await;

            delete_response.assert_status(StatusCode::NO_CONTENT);

            // Verify entity was deleted
            let entity_list = test_server.server
                .get("/api/v1/entity")
                .await;

            entity_list.assert_status_ok();
            let entities: Vec<Entity> = entity_list.json();

            prop_assert!(!entities.contains(&created.entity));
            Ok(())
        }).unwrap()
    }

    #[test]
    fn component_definition_creation_roundtrip(
        definition in strategies::component_definition_strategy()
    ) {
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            let test_server = ApiTestServer::new();

            // Create component definition
            let create_response = test_server.server
                .post("/api/v1/componentdefinition")
                .json(&definition)
                .await;

            create_response.assert_status_ok();
            let created: ComponentDefinition = create_response.json();

            // Verify definition was created
            let list_response = test_server.server
                .get("/api/v1/componentdefinition")
                .await;

            list_response.assert_status_ok();
            let definitions: Vec<ComponentDefinition> = list_response.json();

            prop_assert!(definitions.iter().any(|def| def.component == created.component));
            Ok(())
        }).unwrap()
    }

    #[test]
    fn component_creation_requires_entity_and_definition(
        entity_request in strategies::create_entity_request_strategy(),
        (definition, data) in strategies::component_definition_with_data_strategy()
    ) {
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            let test_server = ApiTestServer::new();

            // Create entity
            let entity_response = test_server.server
                .post("/api/v1/entity")
                .json(&entity_request)
                .await;
            entity_response.assert_status_ok();
            let entity: stigmergy::CreateEntityResponse = entity_response.json();

            // Create component definition
            let def_response = test_server.server
                .post("/api/v1/componentdefinition")
                .json(&definition)
                .await;
            def_response.assert_status_ok();

            // Create component for entity using the pre-generated matching data
            let component_request = CreateComponentRequest {
                component: definition.component.clone(),
                data,
            };

            let entity_str = entity.entity.to_string();
            let entity_base64 = entity_str.strip_prefix("entity:").unwrap();
            let component_response = test_server.server
                .post(&format!("/api/v1/entity/{}/component", entity_base64))
                .json(&component_request)
                .await;

            component_response.assert_status_ok();
            let created: stigmergy::CreateComponentResponse = component_response.json();

            prop_assert_eq!(created.entity, entity.entity);
            prop_assert_eq!(created.component, definition.component);
            Ok(())
        }).unwrap()
    }
}

/// Test that component creation fails for non-existent entities
#[tokio::test]
async fn component_creation_fails_for_nonexistent_entity() {
    let test_server = ApiTestServer::new();

    // First create a component definition so the component type exists
    let definition = ComponentDefinition::new(
        Component::new("TestComponent").unwrap(),
        json!({"type": "object"}),
    );
    let def_response = test_server
        .server
        .post("/api/v1/componentdefinition")
        .json(&definition)
        .await;
    def_response.assert_status_ok();

    // Create a random entity ID that doesn't exist
    let fake_entity = Entity::new([1u8; 32]);
    let entity_str = fake_entity.to_string();
    let entity_base64 = entity_str.strip_prefix("entity:").unwrap();

    let component_request = CreateComponentRequest {
        component: Component::new("TestComponent").unwrap(),
        data: json!({"test": "value"}),
    };

    let response = test_server
        .server
        .post(&format!("/api/v1/entity/{}/component", entity_base64))
        .json(&component_request)
        .await;

    response.assert_status(StatusCode::NOT_FOUND);
}

/// Test API invariant: deleted entities should not have components
#[tokio::test]
async fn entity_deletion_removes_components() {
    let test_server = ApiTestServer::new();

    // Create entity
    let entity_request = CreateEntityRequest { entity: None };
    let entity_response = test_server
        .server
        .post("/api/v1/entity")
        .json(&entity_request)
        .await;
    entity_response.assert_status_ok();
    let entity: stigmergy::CreateEntityResponse = entity_response.json();

    // Create component definition
    let definition = ComponentDefinition::new(
        Component::new("TestComponent").unwrap(),
        json!({"type": "string"}),
    );
    let def_response = test_server
        .server
        .post("/api/v1/componentdefinition")
        .json(&definition)
        .await;
    def_response.assert_status_ok();

    // Create component for entity
    let component_request = CreateComponentRequest {
        component: definition.component.clone(),
        data: json!("test value"),
    };

    let entity_str = entity.entity.to_string();
    let entity_base64 = entity_str.strip_prefix("entity:").unwrap();
    let component_response = test_server
        .server
        .post(&format!("/api/v1/entity/{}/component", entity_base64))
        .json(&component_request)
        .await;
    component_response.assert_status_ok();

    // Verify component exists
    let list_response = test_server
        .server
        .get(&format!("/api/v1/entity/{}/component", entity_base64))
        .await;
    list_response.assert_status_ok();
    let components: Vec<stigmergy::ComponentListItem> = list_response.json();
    assert_eq!(components.len(), 1);

    // Delete entity
    let delete_response = test_server
        .server
        .delete(&format!("/api/v1/entity/{}", entity_base64))
        .await;
    delete_response.assert_status(StatusCode::NO_CONTENT);

    // Verify components for deleted entity return empty (or 404)
    let list_after_delete = test_server
        .server
        .get(&format!("/api/v1/entity/{}/component", entity_base64))
        .await;

    // Should either return 400 for bad entity or empty list
    // Both are acceptable since entity no longer exists
    if list_after_delete.status_code() == StatusCode::OK {
        let components: Vec<stigmergy::ComponentListItem> = list_after_delete.json();
        assert!(components.is_empty());
    } else {
        list_after_delete.assert_status(StatusCode::BAD_REQUEST);
    }
}

/// Comprehensive sequence test: complex multi-operation API workflow
#[tokio::test]
async fn complex_api_workflow_sequence() {
    let test_server = ApiTestServer::new();

    // Step 1: Create multiple entities
    let entity1_request = CreateEntityRequest { entity: None };
    let entity1_response = test_server
        .server
        .post("/api/v1/entity")
        .json(&entity1_request)
        .await;
    entity1_response.assert_status_ok();
    let entity1: stigmergy::CreateEntityResponse = entity1_response.json();

    let entity2_request = CreateEntityRequest { entity: None };
    let entity2_response = test_server
        .server
        .post("/api/v1/entity")
        .json(&entity2_request)
        .await;
    entity2_response.assert_status_ok();
    let entity2: stigmergy::CreateEntityResponse = entity2_response.json();

    // Step 2: Create multiple component definitions
    let definition1 = ComponentDefinition::new(
        Component::new("Position").unwrap(),
        json!({"type": "object", "properties": {"x": {"type": "number"}, "y": {"type": "number"}}}),
    );
    let def1_response = test_server
        .server
        .post("/api/v1/componentdefinition")
        .json(&definition1)
        .await;
    def1_response.assert_status_ok();

    let definition2 = ComponentDefinition::new(
        Component::new("Velocity").unwrap(),
        json!({"type": "object", "properties": {"dx": {"type": "number"}, "dy": {"type": "number"}}}),
    );
    let def2_response = test_server
        .server
        .post("/api/v1/componentdefinition")
        .json(&definition2)
        .await;
    def2_response.assert_status_ok();

    // Step 3: Create components for entities
    let entity1_str = entity1.entity.to_string();
    let entity1_base64 = entity1_str.strip_prefix("entity:").unwrap();
    let entity2_str = entity2.entity.to_string();
    let entity2_base64 = entity2_str.strip_prefix("entity:").unwrap();

    // Position component for entity1
    let pos_request1 = CreateComponentRequest {
        component: Component::new("Position").unwrap(),
        data: json!({"x": 10.0, "y": 20.0}),
    };
    let pos_response1 = test_server
        .server
        .post(&format!("/api/v1/entity/{}/component", entity1_base64))
        .json(&pos_request1)
        .await;
    pos_response1.assert_status_ok();

    // Velocity component for entity1
    let vel_request1 = CreateComponentRequest {
        component: Component::new("Velocity").unwrap(),
        data: json!({"dx": 5.0, "dy": -3.0}),
    };
    let vel_response1 = test_server
        .server
        .post(&format!("/api/v1/entity/{}/component", entity1_base64))
        .json(&vel_request1)
        .await;
    vel_response1.assert_status_ok();

    // Position component for entity2
    let pos_request2 = CreateComponentRequest {
        component: Component::new("Position").unwrap(),
        data: json!({"x": -5.0, "y": 15.0}),
    };
    let pos_response2 = test_server
        .server
        .post(&format!("/api/v1/entity/{}/component", entity2_base64))
        .json(&pos_request2)
        .await;
    pos_response2.assert_status_ok();

    // Step 4: Verify all components are created correctly
    let entity1_components = test_server
        .server
        .get(&format!("/api/v1/entity/{}/component", entity1_base64))
        .await;
    entity1_components.assert_status_ok();
    let components1: Vec<stigmergy::ComponentListItem> = entity1_components.json();
    assert_eq!(components1.len(), 2); // Position and Velocity

    let entity2_components = test_server
        .server
        .get(&format!("/api/v1/entity/{}/component", entity2_base64))
        .await;
    entity2_components.assert_status_ok();
    let components2: Vec<stigmergy::ComponentListItem> = entity2_components.json();
    assert_eq!(components2.len(), 1); // Only Position

    // Step 5: Update component data
    let updated_position = json!({"x": 15.0, "y": 25.0});
    let update_response = test_server
        .server
        .put(&format!(
            "/api/v1/entity/{}/component/Position",
            entity1_base64
        ))
        .json(&updated_position)
        .await;
    update_response.assert_status_ok();

    // Step 6: Verify update took effect
    let get_component_response = test_server
        .server
        .get(&format!(
            "/api/v1/entity/{}/component/Position",
            entity1_base64
        ))
        .await;
    get_component_response.assert_status_ok();
    let retrieved_component: serde_json::Value = get_component_response.json();
    assert_eq!(retrieved_component, updated_position);

    // Step 7: Delete one component
    let delete_component_response = test_server
        .server
        .delete(&format!(
            "/api/v1/entity/{}/component/Velocity",
            entity1_base64
        ))
        .await;
    delete_component_response.assert_status(StatusCode::NO_CONTENT);

    // Step 8: Verify component was deleted
    let final_components = test_server
        .server
        .get(&format!("/api/v1/entity/{}/component", entity1_base64))
        .await;
    final_components.assert_status_ok();
    let final_list: Vec<stigmergy::ComponentListItem> = final_components.json();
    assert_eq!(final_list.len(), 1); // Only Position remains
    assert_eq!(final_list[0].component, Component::new("Position").unwrap());

    // Step 9: Verify component definitions still exist
    let all_definitions = test_server.server.get("/api/v1/componentdefinition").await;
    all_definitions.assert_status_ok();
    let definitions: Vec<ComponentDefinition> = all_definitions.json();
    assert_eq!(definitions.len(), 2); // Both definitions should still be there

    // Step 10: Clean up - delete entities
    let delete_entity1 = test_server
        .server
        .delete(&format!("/api/v1/entity/{}", entity1_base64))
        .await;
    delete_entity1.assert_status(StatusCode::NO_CONTENT);

    let delete_entity2 = test_server
        .server
        .delete(&format!("/api/v1/entity/{}", entity2_base64))
        .await;
    delete_entity2.assert_status(StatusCode::NO_CONTENT);

    // Final verification: entities are gone
    let final_entities = test_server.server.get("/api/v1/entity").await;
    final_entities.assert_status_ok();
    let entities: Vec<Entity> = final_entities.json();
    assert!(entities.is_empty());
}

/// Test multiple component definitions with same component name should fail
#[tokio::test]
async fn duplicate_component_definition_fails() {
    let test_server = ApiTestServer::new();

    let definition1 = ComponentDefinition::new(
        Component::new("Health").unwrap(),
        json!({"type": "integer"}),
    );

    let definition2 = ComponentDefinition::new(
        Component::new("Health").unwrap(), // Same name
        json!({"type": "number"}),         // Different schema
    );

    // Create first definition - should succeed
    let first_response = test_server
        .server
        .post("/api/v1/componentdefinition")
        .json(&definition1)
        .await;
    first_response.assert_status_ok();

    // Create second definition with same name - should fail
    let second_response = test_server
        .server
        .post("/api/v1/componentdefinition")
        .json(&definition2)
        .await;
    second_response.assert_status(StatusCode::CONFLICT);
}

/// Test that component validation works correctly with complex schemas
#[tokio::test]
async fn component_validation_with_complex_schema() {
    let test_server = ApiTestServer::new();

    // Create entity
    let entity_request = CreateEntityRequest { entity: None };
    let entity_response = test_server
        .server
        .post("/api/v1/entity")
        .json(&entity_request)
        .await;
    entity_response.assert_status_ok();
    let entity: stigmergy::CreateEntityResponse = entity_response.json();

    // Create complex component definition
    let definition = ComponentDefinition::new(
        Component::new("Player").unwrap(),
        json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "level": {"type": "integer"},
                "health": {"type": "number"},
                "position": {
                    "type": "object",
                    "properties": {
                        "x": {"type": "number"},
                        "y": {"type": "number"}
                    },
                    "required": ["x", "y"]
                }
            },
            "required": ["name", "level", "health", "position"]
        }),
    );

    let def_response = test_server
        .server
        .post("/api/v1/componentdefinition")
        .json(&definition)
        .await;
    def_response.assert_status_ok();

    let entity_str = entity.entity.to_string();
    let entity_base64 = entity_str.strip_prefix("entity:").unwrap();

    // Test valid data
    let valid_request = CreateComponentRequest {
        component: Component::new("Player").unwrap(),
        data: json!({
            "name": "Hero",
            "level": 42,
            "health": 100.5,
            "position": {"x": 10.0, "y": 20.0}
        }),
    };

    let valid_response = test_server
        .server
        .post(&format!("/api/v1/entity/{}/component", entity_base64))
        .json(&valid_request)
        .await;
    valid_response.assert_status_ok();

    // Test invalid data (missing required field)
    let invalid_request = CreateComponentRequest {
        component: Component::new("Player").unwrap(),
        data: json!({
            "name": "Villain",
            "level": 30
            // Missing health and position
        }),
    };

    let invalid_response = test_server
        .server
        .post(&format!("/api/v1/entity/{}/component", entity_base64))
        .json(&invalid_request)
        .await;
    invalid_response.assert_status(StatusCode::BAD_REQUEST);
}
