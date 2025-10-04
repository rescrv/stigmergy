//! # Data Operations Layer
//!
//! This module provides a standardized wrapper around the DataStore trait,
//! offering operation results that include success/failure information and
//! specialized variants for replay operations that handle idempotent behavior.
//!
//! ## Key Features
//!
//! - **Standardized Results**: `OperationResult<T>` wraps all data store operations
//! - **Error Handling**: Consistent error reporting across all operations
//! - **Replay Support**: Special operations that handle "already exists" gracefully
//! - **Type Safety**: Generic result types preserve operation-specific return values
//!
//! ## Architecture
//!
//! ```text
//! HTTP Handlers
//!      ↓
//! DataStoreOperations (standardized wrapper)
//!      ↓
//! DataStore trait (actual storage)
//! ```
//!
//! ## Operation Types
//!
//! 1. **Regular Operations**: Standard CRUD operations that expect entities/definitions to not exist
//! 2. **Replay Operations**: Idempotent operations that can handle "already exists" gracefully
//!
//! ## Usage Examples
//!
//! ### Standard Operations
//!
//! ```rust
//! use stigmergy::{DataStoreOperations, InMemoryDataStore, Entity};
//! use std::sync::Arc;
//!
//! let store = Arc::new(InMemoryDataStore::new());
//! let entity = Entity::new([1u8; 32]);
//!
//! // Create entity - will fail if already exists
//! let result = DataStoreOperations::create_entity(&*store, &entity);
//! assert!(result.success);
//!
//! // Try to create again - will fail
//! let result2 = DataStoreOperations::create_entity(&*store, &entity);
//! assert!(!result2.success);
//! ```

use crate::{Component, ComponentDefinition, DataStore, DataStoreError, Entity};
use serde_json::Value;

/// Result of a data store operation with success/failure information.
///
/// This structure standardizes the return type for all data store operations,
/// providing both success/failure status and optional data or error information.
/// It allows callers to handle operations uniformly while preserving type safety
/// for operation-specific return values.
///
/// # Type Parameters
/// * `T` - The type of data returned on successful operations (default: `()`)
///
/// # Examples
///
/// ```rust
/// use stigmergy::{OperationResult, DataStoreError};
///
/// // Success with data
/// let result: OperationResult<u32> = OperationResult::success(42);
/// assert!(result.success);
/// assert_eq!(result.data, Some(42));
///
/// // Success without data
/// let result: OperationResult<()> = OperationResult::success_void();
/// assert!(result.success);
/// assert_eq!(result.data, None);
///
/// // Failure with error
/// let result: OperationResult<u32> = OperationResult::failure(DataStoreError::NotFound);
/// assert!(!result.success);
/// assert!(result.error.is_some());
/// ```
#[derive(Debug, Clone)]
pub struct OperationResult<T = ()> {
    /// Whether the operation completed successfully
    pub success: bool,
    /// The result data if the operation succeeded
    pub data: Option<T>,
    /// The error information if the operation failed
    pub error: Option<DataStoreError>,
}

impl<T> OperationResult<T> {
    /// Creates a successful operation result with data.
    ///
    /// # Arguments
    /// * `data` - The result data to include
    ///
    /// # Examples
    /// ```
    /// use stigmergy::OperationResult;
    ///
    /// let result = OperationResult::success("data");
    /// assert!(result.success);
    /// assert_eq!(result.data, Some("data"));
    /// ```
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    /// Creates a successful operation result without data.
    ///
    /// This is useful for operations that indicate success but don't return
    /// meaningful data (e.g., delete operations).
    ///
    /// # Examples
    /// ```
    /// use stigmergy::OperationResult;
    ///
    /// let result: OperationResult<()> = OperationResult::success_void();
    /// assert!(result.success);
    /// assert!(result.data.is_none());
    /// ```
    pub fn success_void() -> Self {
        Self {
            success: true,
            data: None,
            error: None,
        }
    }

    /// Creates a failed operation result with error information.
    ///
    /// # Arguments
    /// * `error` - The error that caused the operation to fail
    ///
    /// # Examples
    /// ```
    /// use stigmergy::{OperationResult, DataStoreError};
    ///
    /// let result: OperationResult<()> = OperationResult::failure(DataStoreError::NotFound);
    /// assert!(!result.success);
    /// assert!(result.error.is_some());
    /// ```
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

/// Standardized wrapper for data store operations.
///
/// This struct provides static methods that wrap DataStore trait operations
/// with consistent error handling and result formatting. It serves as the
/// primary interface for HTTP handlers and other components that need to
/// interact with the data store.
///
/// All methods return `OperationResult<T>` which provides success/failure
/// information along with optional data or error details.
///
/// # Examples
///
/// ```rust
/// use stigmergy::{DataStoreOperations, InMemoryDataStore, Entity};
/// use std::sync::Arc;
///
/// let store = Arc::new(InMemoryDataStore::new());
/// let entity = Entity::new([1u8; 32]);
///
/// // Create entity
/// let result = DataStoreOperations::create_entity(&*store, &entity);
/// if result.success {
///     println!("Entity created successfully");
/// } else {
///     println!("Failed to create entity: {:?}", result.error);
/// }
/// ```
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
    pub fn delete_entity(data_store: &dyn DataStore, entity: &Entity) -> OperationResult<bool> {
        match data_store.delete_entity(entity) {
            Ok(deleted) => OperationResult::success(deleted),
            Err(e) => OperationResult::failure(e),
        }
    }

    /// Create a component definition in the data store
    pub fn create_component_definition(
        data_store: &dyn DataStore,
        component: &Component,
        definition: &ComponentDefinition,
    ) -> OperationResult<()> {
        match data_store.create_component_definition(component, definition) {
            Ok(()) => OperationResult::success_void(),
            Err(e) => OperationResult::failure(e),
        }
    }

    /// Update a component definition in the data store
    pub fn update_component_definition(
        data_store: &dyn DataStore,
        component: &Component,
        definition: &ComponentDefinition,
    ) -> OperationResult<bool> {
        match data_store.update_component_definition(component, definition) {
            Ok(updated) => OperationResult::success(updated),
            Err(e) => OperationResult::failure(e),
        }
    }

    /// Delete a component definition from the data store
    pub fn delete_component_definition(
        data_store: &dyn DataStore,
        component: &Component,
    ) -> OperationResult<bool> {
        match data_store.delete_component_definition(component) {
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

    /// Create a component instance in the data store (entity-scoped)
    pub fn create_component(
        data_store: &dyn DataStore,
        entity: &Entity,
        component: &Component,
        data: &Value,
    ) -> OperationResult<()> {
        match data_store.create_component(entity, component, data) {
            Ok(()) => OperationResult::success_void(),
            Err(e) => OperationResult::failure(e),
        }
    }

    /// Update a component instance in the data store (entity-scoped)
    pub fn update_component(
        data_store: &dyn DataStore,
        entity: &Entity,
        component: &Component,
        data: &Value,
    ) -> OperationResult<bool> {
        match data_store.update_component(entity, component, data) {
            Ok(updated) => OperationResult::success(updated),
            Err(e) => OperationResult::failure(e),
        }
    }

    /// Delete a component instance from the data store (entity-scoped)
    pub fn delete_component(
        data_store: &dyn DataStore,
        entity: &Entity,
        component: &Component,
    ) -> OperationResult<bool> {
        match data_store.delete_component(entity, component) {
            Ok(deleted) => OperationResult::success(deleted),
            Err(e) => OperationResult::failure(e),
        }
    }

    /// Delete all components for a specific entity
    pub fn delete_all_components_for_entity(
        data_store: &dyn DataStore,
        entity: &Entity,
    ) -> OperationResult<u32> {
        match data_store.delete_all_components_for_entity(entity) {
            Ok(count) => OperationResult::success(count),
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
        component: &Component,
        definition: &ComponentDefinition,
    ) -> OperationResult<bool> {
        match data_store.create_component_definition(component, definition) {
            Ok(()) => OperationResult::success(true), // Created new
            Err(DataStoreError::AlreadyExists) => OperationResult::success(false), // Already existed
            Err(e) => OperationResult::failure(e),                                 // Real error
        }
    }

    pub fn create_component(
        data_store: &dyn DataStore,
        entity: &Entity,
        component: &Component,
        data: &Value,
    ) -> OperationResult<bool> {
        match data_store.create_component(entity, component, data) {
            Ok(()) => OperationResult::success(true), // Created new
            Err(DataStoreError::AlreadyExists) => OperationResult::success(false), // Already existed
            Err(e) => OperationResult::failure(e),                                 // Real error
        }
    }
}
