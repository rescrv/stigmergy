use crate::{ComponentDefinition, DataStore, DataStoreError, Entity};
use serde_json::Value;

/// Result of a data store operation with success/failure information
#[derive(Debug, Clone)]
pub struct OperationResult<T = ()> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<DataStoreError>,
}

impl<T> OperationResult<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn success_void() -> Self {
        Self {
            success: true,
            data: None,
            error: None,
        }
    }

    pub fn failure(error: DataStoreError) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error),
        }
    }

    /// Get the error, panicking with a descriptive message if called on success
    pub fn into_error(self) -> DataStoreError {
        self.error.expect(
            "Called into_error() on successful OperationResult - this is a programming error",
        )
    }
}

/// Core data store operations that can be used by both HTTP handlers and replay
pub struct DataStoreOperations;

impl DataStoreOperations {
    /// Create an entity in the data store
    pub fn create_entity(data_store: &dyn DataStore, entity: &Entity) -> OperationResult<()> {
        match data_store.create_entity(entity) {
            Ok(()) => OperationResult::success_void(),
            Err(e) => OperationResult::failure(e),
        }
    }

    /// Delete an entity from the data store
    pub fn delete_entity(data_store: &dyn DataStore, entity_id: &str) -> OperationResult<bool> {
        match data_store.delete_entity(entity_id) {
            Ok(deleted) => OperationResult::success(deleted),
            Err(e) => OperationResult::failure(e),
        }
    }

    /// Create a component definition in the data store
    pub fn create_component_definition(
        data_store: &dyn DataStore,
        def_id: &str,
        definition: &ComponentDefinition,
    ) -> OperationResult<()> {
        match data_store.create_component_definition(def_id, definition) {
            Ok(()) => OperationResult::success_void(),
            Err(e) => OperationResult::failure(e),
        }
    }

    /// Update a component definition in the data store
    pub fn update_component_definition(
        data_store: &dyn DataStore,
        def_id: &str,
        definition: &ComponentDefinition,
    ) -> OperationResult<bool> {
        match data_store.update_component_definition(def_id, definition) {
            Ok(updated) => OperationResult::success(updated),
            Err(e) => OperationResult::failure(e),
        }
    }

    /// Delete a component definition from the data store
    pub fn delete_component_definition(
        data_store: &dyn DataStore,
        def_id: &str,
    ) -> OperationResult<bool> {
        match data_store.delete_component_definition(def_id) {
            Ok(deleted) => OperationResult::success(deleted),
            Err(e) => OperationResult::failure(e),
        }
    }

    /// Delete all component definitions from the data store
    pub fn delete_all_component_definitions(data_store: &dyn DataStore) -> OperationResult<u32> {
        match data_store.delete_all_component_definitions() {
            Ok(count) => OperationResult::success(count),
            Err(e) => OperationResult::failure(e),
        }
    }

    /// Create a component instance in the data store
    pub fn create_component(
        data_store: &dyn DataStore,
        component_id: &str,
        data: &Value,
    ) -> OperationResult<()> {
        match data_store.create_component(component_id, data) {
            Ok(()) => OperationResult::success_void(),
            Err(e) => OperationResult::failure(e),
        }
    }

    /// Update a component instance in the data store
    pub fn update_component(
        data_store: &dyn DataStore,
        component_id: &str,
        data: &Value,
    ) -> OperationResult<bool> {
        match data_store.update_component(component_id, data) {
            Ok(updated) => OperationResult::success(updated),
            Err(e) => OperationResult::failure(e),
        }
    }

    /// Delete a component instance from the data store
    pub fn delete_component(
        data_store: &dyn DataStore,
        component_id: &str,
    ) -> OperationResult<bool> {
        match data_store.delete_component(component_id) {
            Ok(deleted) => OperationResult::success(deleted),
            Err(e) => OperationResult::failure(e),
        }
    }

    /// Delete all component instances from the data store
    pub fn delete_all_components(data_store: &dyn DataStore) -> OperationResult<u32> {
        match data_store.delete_all_components() {
            Ok(count) => OperationResult::success(count),
            Err(e) => OperationResult::failure(e),
        }
    }
}

/// Create operations for replay - return results that let caller handle AlreadyExists policy
pub mod replay {
    use super::*;

    /// Creates an entity, returning a result that indicates if the error was AlreadyExists
    pub fn create_entity(data_store: &dyn DataStore, entity: &Entity) -> OperationResult<bool> {
        match data_store.create_entity(entity) {
            Ok(()) => OperationResult::success(true), // Created new
            Err(DataStoreError::AlreadyExists) => OperationResult::success(false), // Already existed
            Err(e) => OperationResult::failure(e),                                 // Real error
        }
    }

    pub fn create_component_definition(
        data_store: &dyn DataStore,
        def_id: &str,
        definition: &ComponentDefinition,
    ) -> OperationResult<bool> {
        match data_store.create_component_definition(def_id, definition) {
            Ok(()) => OperationResult::success(true), // Created new
            Err(DataStoreError::AlreadyExists) => OperationResult::success(false), // Already existed
            Err(e) => OperationResult::failure(e),                                 // Real error
        }
    }

    pub fn create_component(
        data_store: &dyn DataStore,
        component_id: &str,
        data: &Value,
    ) -> OperationResult<bool> {
        match data_store.create_component(component_id, data) {
            Ok(()) => OperationResult::success(true), // Created new
            Err(DataStoreError::AlreadyExists) => OperationResult::success(false), // Already existed
            Err(e) => OperationResult::failure(e),                                 // Real error
        }
    }
}
