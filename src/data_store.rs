use serde_json::Value;
use std::collections::HashMap;
use std::sync::Mutex;

use crate::{ComponentDefinition, Entity};

/// Trait for data store operations
pub trait DataStore: Send + Sync {
    // Entity operations
    fn create_entity(&self, entity: &Entity) -> Result<(), DataStoreError>;
    fn get_entity(&self, entity_id: &str) -> Result<Option<Entity>, DataStoreError>;
    fn delete_entity(&self, entity_id: &str) -> Result<bool, DataStoreError>;
    fn list_entities(&self) -> Result<Vec<Entity>, DataStoreError>;

    // Component Definition operations
    fn create_component_definition(
        &self,
        id: &str,
        definition: &ComponentDefinition,
    ) -> Result<(), DataStoreError>;
    fn get_component_definition(
        &self,
        id: &str,
    ) -> Result<Option<ComponentDefinition>, DataStoreError>;
    fn update_component_definition(
        &self,
        id: &str,
        definition: &ComponentDefinition,
    ) -> Result<bool, DataStoreError>;
    fn delete_component_definition(&self, id: &str) -> Result<bool, DataStoreError>;
    fn delete_all_component_definitions(&self) -> Result<u32, DataStoreError>;
    fn list_component_definitions(
        &self,
    ) -> Result<Vec<(String, ComponentDefinition)>, DataStoreError>;

    // Component Instance operations
    fn create_component(&self, id: &str, data: &Value) -> Result<(), DataStoreError>;
    fn get_component(&self, id: &str) -> Result<Option<Value>, DataStoreError>;
    fn update_component(&self, id: &str, data: &Value) -> Result<bool, DataStoreError>;
    fn delete_component(&self, id: &str) -> Result<bool, DataStoreError>;
    fn delete_all_components(&self) -> Result<u32, DataStoreError>;
    fn list_components(&self) -> Result<Vec<(String, Value)>, DataStoreError>;
}

/// Errors that can occur during data store operations
#[derive(Debug, Clone)]
pub enum DataStoreError {
    NotFound,
    AlreadyExists,
    SerializationError(String),
    IoError(String),
    Internal(String),
}

impl std::fmt::Display for DataStoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound => write!(f, "Item not found in data store"),
            Self::AlreadyExists => write!(f, "Item already exists in data store"),
            Self::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
            Self::IoError(msg) => write!(f, "IO error: {}", msg),
            Self::Internal(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl std::error::Error for DataStoreError {}

/// Simple in-memory data store implementation
pub struct InMemoryDataStore {
    entities: Mutex<HashMap<String, Entity>>,
    component_definitions: Mutex<HashMap<String, ComponentDefinition>>,
    components: Mutex<HashMap<String, Value>>,
}

impl InMemoryDataStore {
    pub fn new() -> Self {
        Self {
            entities: Mutex::new(HashMap::new()),
            component_definitions: Mutex::new(HashMap::new()),
            components: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for InMemoryDataStore {
    fn default() -> Self {
        Self::new()
    }
}

impl DataStore for InMemoryDataStore {
    fn create_entity(&self, entity: &Entity) -> Result<(), DataStoreError> {
        let mut entities = self.entities.lock().unwrap();
        let entity_id = entity.to_string();

        if entities.contains_key(&entity_id) {
            return Err(DataStoreError::AlreadyExists);
        }

        entities.insert(entity_id, *entity);
        Ok(())
    }

    fn get_entity(&self, entity_id: &str) -> Result<Option<Entity>, DataStoreError> {
        let entities = self.entities.lock().unwrap();
        Ok(entities.get(entity_id).copied())
    }

    fn delete_entity(&self, entity_id: &str) -> Result<bool, DataStoreError> {
        let mut entities = self.entities.lock().unwrap();
        Ok(entities.remove(entity_id).is_some())
    }

    fn list_entities(&self) -> Result<Vec<Entity>, DataStoreError> {
        let entities = self.entities.lock().unwrap();
        Ok(entities.values().copied().collect())
    }

    fn create_component_definition(
        &self,
        id: &str,
        definition: &ComponentDefinition,
    ) -> Result<(), DataStoreError> {
        let mut definitions = self.component_definitions.lock().unwrap();

        if definitions.contains_key(id) {
            return Err(DataStoreError::AlreadyExists);
        }

        definitions.insert(id.to_string(), definition.clone());
        Ok(())
    }

    fn get_component_definition(
        &self,
        id: &str,
    ) -> Result<Option<ComponentDefinition>, DataStoreError> {
        let definitions = self.component_definitions.lock().unwrap();
        Ok(definitions.get(id).cloned())
    }

    fn update_component_definition(
        &self,
        id: &str,
        definition: &ComponentDefinition,
    ) -> Result<bool, DataStoreError> {
        let mut definitions = self.component_definitions.lock().unwrap();

        if definitions.contains_key(id) {
            definitions.insert(id.to_string(), definition.clone());
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn delete_component_definition(&self, id: &str) -> Result<bool, DataStoreError> {
        let mut definitions = self.component_definitions.lock().unwrap();
        Ok(definitions.remove(id).is_some())
    }

    fn delete_all_component_definitions(&self) -> Result<u32, DataStoreError> {
        let mut definitions = self.component_definitions.lock().unwrap();
        let count = definitions.len() as u32;
        definitions.clear();
        Ok(count)
    }

    fn list_component_definitions(
        &self,
    ) -> Result<Vec<(String, ComponentDefinition)>, DataStoreError> {
        let definitions = self.component_definitions.lock().unwrap();
        Ok(definitions
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect())
    }

    fn create_component(&self, id: &str, data: &Value) -> Result<(), DataStoreError> {
        let mut components = self.components.lock().unwrap();

        if components.contains_key(id) {
            return Err(DataStoreError::AlreadyExists);
        }

        components.insert(id.to_string(), data.clone());
        Ok(())
    }

    fn get_component(&self, id: &str) -> Result<Option<Value>, DataStoreError> {
        let components = self.components.lock().unwrap();
        Ok(components.get(id).cloned())
    }

    fn update_component(&self, id: &str, data: &Value) -> Result<bool, DataStoreError> {
        let mut components = self.components.lock().unwrap();

        if components.contains_key(id) {
            components.insert(id.to_string(), data.clone());
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn delete_component(&self, id: &str) -> Result<bool, DataStoreError> {
        let mut components = self.components.lock().unwrap();
        Ok(components.remove(id).is_some())
    }

    fn delete_all_components(&self) -> Result<u32, DataStoreError> {
        let mut components = self.components.lock().unwrap();
        let count = components.len() as u32;
        components.clear();
        Ok(count)
    }

    fn list_components(&self) -> Result<Vec<(String, Value)>, DataStoreError> {
        let components = self.components.lock().unwrap();
        Ok(components
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Component;
    use serde_json::json;

    fn test_entity() -> Entity {
        Entity::new([1u8; 32])
    }

    fn test_component_definition() -> ComponentDefinition {
        ComponentDefinition::new(
            Component::new("TestComponent").unwrap(),
            json!({"type": "string"}),
        )
    }

    #[test]
    fn entity_create_and_get() {
        let store = InMemoryDataStore::new();
        let entity = test_entity();
        let entity_id = entity.to_string();

        // Create entity
        assert!(store.create_entity(&entity).is_ok());

        // Get entity
        let retrieved = store.get_entity(&entity_id).unwrap();
        assert_eq!(retrieved, Some(entity));

        // Try to create duplicate
        assert!(matches!(
            store.create_entity(&entity),
            Err(DataStoreError::AlreadyExists)
        ));
    }

    #[test]
    fn entity_delete() {
        let store = InMemoryDataStore::new();
        let entity = test_entity();
        let entity_id = entity.to_string();

        // Create and delete entity
        store.create_entity(&entity).unwrap();
        assert!(store.delete_entity(&entity_id).unwrap());

        // Verify deleted
        assert_eq!(store.get_entity(&entity_id).unwrap(), None);

        // Delete non-existent
        assert!(!store.delete_entity(&entity_id).unwrap());
    }

    #[test]
    fn component_definition_crud() {
        let store = InMemoryDataStore::new();
        let definition = test_component_definition();
        let def_id = "test_def";

        // Create
        assert!(
            store
                .create_component_definition(def_id, &definition)
                .is_ok()
        );

        // Get
        let retrieved = store.get_component_definition(def_id).unwrap();
        assert_eq!(retrieved, Some(definition.clone()));

        // Update
        let mut updated_def = definition.clone();
        updated_def.schema = json!({"type": "number"});
        assert!(
            store
                .update_component_definition(def_id, &updated_def)
                .unwrap()
        );

        let retrieved = store.get_component_definition(def_id).unwrap();
        assert_eq!(retrieved.unwrap().schema, json!({"type": "number"}));

        // Delete
        assert!(store.delete_component_definition(def_id).unwrap());
        assert_eq!(store.get_component_definition(def_id).unwrap(), None);
    }

    #[test]
    fn component_instance_crud() {
        let store = InMemoryDataStore::new();
        let component_data = json!({"color": "red"});
        let comp_id = "test_comp";

        // Create
        assert!(store.create_component(comp_id, &component_data).is_ok());

        // Get
        let retrieved = store.get_component(comp_id).unwrap();
        assert_eq!(retrieved, Some(component_data));

        // Update
        let updated_data = json!({"color": "blue"});
        assert!(store.update_component(comp_id, &updated_data).unwrap());

        let retrieved = store.get_component(comp_id).unwrap();
        assert_eq!(retrieved, Some(updated_data));

        // Delete
        assert!(store.delete_component(comp_id).unwrap());
        assert_eq!(store.get_component(comp_id).unwrap(), None);
    }

    #[test]
    fn delete_all_operations() {
        let store = InMemoryDataStore::new();

        // Create some test data
        let def1 = test_component_definition();
        let def2 = test_component_definition();
        store.create_component_definition("def1", &def1).unwrap();
        store.create_component_definition("def2", &def2).unwrap();

        let comp_data = json!({"test": "data"});
        store.create_component("comp1", &comp_data).unwrap();
        store.create_component("comp2", &comp_data).unwrap();

        // Delete all
        assert_eq!(store.delete_all_component_definitions().unwrap(), 2);
        assert_eq!(store.delete_all_components().unwrap(), 2);

        // Verify empty
        assert!(store.list_component_definitions().unwrap().is_empty());
        assert!(store.list_components().unwrap().is_empty());
    }
}
