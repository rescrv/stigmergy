use serde_json::Value;
use std::collections::HashMap;
use std::sync::Mutex;

use crate::{ComponentDefinition, Entity};

/// Type alias for component key-value pairs returned by list operations
pub type ComponentList = Vec<((Entity, String), Value)>;

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

    // Component Instance operations - now entity-scoped
    fn create_component(
        &self,
        entity: &Entity,
        id: &str,
        data: &Value,
    ) -> Result<(), DataStoreError>;
    fn get_component(&self, entity: &Entity, id: &str) -> Result<Option<Value>, DataStoreError>;
    fn update_component(
        &self,
        entity: &Entity,
        id: &str,
        data: &Value,
    ) -> Result<bool, DataStoreError>;
    fn delete_component(&self, entity: &Entity, id: &str) -> Result<bool, DataStoreError>;
    fn delete_all_components_for_entity(&self, entity: &Entity) -> Result<u32, DataStoreError>;
    fn delete_all_components(&self) -> Result<u32, DataStoreError>;
    fn list_components_for_entity(
        &self,
        entity: &Entity,
    ) -> Result<Vec<(String, Value)>, DataStoreError>;
    fn list_components(&self) -> Result<ComponentList, DataStoreError>;
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
    components: Mutex<HashMap<(Entity, String), Value>>,
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

        if let Some(entity) = entities.remove(entity_id) {
            // Cascade delete: remove all components belonging to this entity
            drop(entities); // Release the lock
            let mut components = self.components.lock().unwrap();

            // Remove all components that belong to this entity
            components.retain(|(comp_entity, _), _| comp_entity != &entity);

            Ok(true)
        } else {
            Ok(false)
        }
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

    fn create_component(
        &self,
        entity: &Entity,
        id: &str,
        data: &Value,
    ) -> Result<(), DataStoreError> {
        // First verify the entity exists
        let entities = self.entities.lock().unwrap();
        let entity_id = entity.to_string();
        if !entities.contains_key(&entity_id) {
            return Err(DataStoreError::NotFound);
        }
        drop(entities);

        let mut components = self.components.lock().unwrap();
        let key = (*entity, id.to_string());

        if components.contains_key(&key) {
            return Err(DataStoreError::AlreadyExists);
        }

        components.insert(key, data.clone());
        Ok(())
    }

    fn get_component(&self, entity: &Entity, id: &str) -> Result<Option<Value>, DataStoreError> {
        let components = self.components.lock().unwrap();
        let key = (*entity, id.to_string());
        Ok(components.get(&key).cloned())
    }

    fn update_component(
        &self,
        entity: &Entity,
        id: &str,
        data: &Value,
    ) -> Result<bool, DataStoreError> {
        let mut components = self.components.lock().unwrap();
        let key = (*entity, id.to_string());

        if let std::collections::hash_map::Entry::Occupied(mut e) = components.entry(key) {
            e.insert(data.clone());
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn delete_component(&self, entity: &Entity, id: &str) -> Result<bool, DataStoreError> {
        let mut components = self.components.lock().unwrap();
        let key = (*entity, id.to_string());
        Ok(components.remove(&key).is_some())
    }

    fn delete_all_components_for_entity(&self, entity: &Entity) -> Result<u32, DataStoreError> {
        let mut components = self.components.lock().unwrap();
        let initial_count = components.len();

        components.retain(|(comp_entity, _), _| comp_entity != entity);

        let removed_count = initial_count - components.len();
        Ok(removed_count as u32)
    }

    fn delete_all_components(&self) -> Result<u32, DataStoreError> {
        let mut components = self.components.lock().unwrap();
        let count = components.len() as u32;
        components.clear();
        Ok(count)
    }

    fn list_components_for_entity(
        &self,
        entity: &Entity,
    ) -> Result<Vec<(String, Value)>, DataStoreError> {
        let components = self.components.lock().unwrap();
        Ok(components
            .iter()
            .filter_map(|((comp_entity, comp_id), value)| {
                if comp_entity == entity {
                    Some((comp_id.clone(), value.clone()))
                } else {
                    None
                }
            })
            .collect())
    }

    fn list_components(&self) -> Result<Vec<((Entity, String), Value)>, DataStoreError> {
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
        let entity = test_entity();
        let component_data = json!({"color": "red"});
        let comp_id = "test_comp";

        // First create the entity
        store.create_entity(&entity).unwrap();

        // Create component
        assert!(
            store
                .create_component(&entity, comp_id, &component_data)
                .is_ok()
        );

        // Get component
        let retrieved = store.get_component(&entity, comp_id).unwrap();
        assert_eq!(retrieved, Some(component_data));

        // Update component
        let updated_data = json!({"color": "blue"});
        assert!(
            store
                .update_component(&entity, comp_id, &updated_data)
                .unwrap()
        );

        let retrieved = store.get_component(&entity, comp_id).unwrap();
        assert_eq!(retrieved, Some(updated_data));

        // Delete component
        assert!(store.delete_component(&entity, comp_id).unwrap());
        assert_eq!(store.get_component(&entity, comp_id).unwrap(), None);
    }

    #[test]
    fn delete_all_operations() {
        let store = InMemoryDataStore::new();
        let entity = test_entity();

        // First create the entity
        store.create_entity(&entity).unwrap();

        // Create some test data
        let def1 = test_component_definition();
        let def2 = test_component_definition();
        store.create_component_definition("def1", &def1).unwrap();
        store.create_component_definition("def2", &def2).unwrap();

        let comp_data = json!({"test": "data"});
        store
            .create_component(&entity, "comp1", &comp_data)
            .unwrap();
        store
            .create_component(&entity, "comp2", &comp_data)
            .unwrap();

        // Delete all
        assert_eq!(store.delete_all_component_definitions().unwrap(), 2);
        assert_eq!(store.delete_all_components().unwrap(), 2);

        // Verify empty
        assert!(store.list_component_definitions().unwrap().is_empty());
        assert!(store.list_components().unwrap().is_empty());
    }

    #[test]
    fn cascade_delete_on_entity_removal() {
        let store = InMemoryDataStore::new();
        let entity = test_entity();

        // Create entity and components
        store.create_entity(&entity).unwrap();
        let comp_data = json!({"test": "value"});
        store
            .create_component(&entity, "comp1", &comp_data)
            .unwrap();
        store
            .create_component(&entity, "comp2", &comp_data)
            .unwrap();

        // Verify components exist
        assert_eq!(store.list_components_for_entity(&entity).unwrap().len(), 2);

        // Delete entity - should cascade delete components
        assert!(store.delete_entity(&entity.to_string()).unwrap());

        // Verify all components are gone
        assert_eq!(store.list_components_for_entity(&entity).unwrap().len(), 0);
        assert_eq!(store.list_components().unwrap().len(), 0);
    }

    #[test]
    fn component_requires_existing_entity() {
        let store = InMemoryDataStore::new();
        let entity = test_entity();
        let comp_data = json!({"test": "value"});

        // Try to create component for non-existent entity
        assert!(matches!(
            store.create_component(&entity, "comp1", &comp_data),
            Err(DataStoreError::NotFound)
        ));
    }

    #[test]
    fn entity_scoped_component_isolation() {
        let store = InMemoryDataStore::new();
        let entity1 = Entity::new([1u8; 32]);
        let entity2 = Entity::new([2u8; 32]);

        // Create both entities
        store.create_entity(&entity1).unwrap();
        store.create_entity(&entity2).unwrap();

        // Create components with same ID for different entities
        let comp_data1 = json!({"entity": "one"});
        let comp_data2 = json!({"entity": "two"});

        store
            .create_component(&entity1, "same_id", &comp_data1)
            .unwrap();
        store
            .create_component(&entity2, "same_id", &comp_data2)
            .unwrap();

        // Verify components are separate
        assert_eq!(
            store.get_component(&entity1, "same_id").unwrap(),
            Some(comp_data1)
        );
        assert_eq!(
            store.get_component(&entity2, "same_id").unwrap(),
            Some(comp_data2.clone())
        );

        // Verify entity-scoped listing
        assert_eq!(store.list_components_for_entity(&entity1).unwrap().len(), 1);
        assert_eq!(store.list_components_for_entity(&entity2).unwrap().len(), 1);

        // Delete component from one entity shouldn't affect the other
        assert!(store.delete_component(&entity1, "same_id").unwrap());
        assert_eq!(store.get_component(&entity1, "same_id").unwrap(), None);
        assert_eq!(
            store.get_component(&entity2, "same_id").unwrap(),
            Some(comp_data2.clone())
        );
    }
}
