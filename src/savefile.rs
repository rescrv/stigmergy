//! # Persistent Operation Logging
//!
//! This module provides comprehensive logging and replay capabilities for all operations
//! in the stigmergy system. Every state transition is recorded to JSONL files with
//! timestamps, operation details, and metadata for debugging, auditing, and system replay.
//!
//! ## Key Features
//!
//! - **Complete Operation Logging**: Records all entity, component definition, and component instance operations
//! - **JSONL Format**: Each operation is stored as a single line of JSON for easy parsing
//! - **Metadata Tracking**: Includes timestamps, operation context, and success/failure status
//! - **Replay Support**: Ability to replay operations for state reconstruction
//! - **Validation Tracking**: Records schema validation results and performance metrics
//!
//! ## File Format
//!
//! Each log entry is stored as a single line of JSON with the following structure:
//! ```json
//! {
//!   "id": "unique-entry-id",
//!   "timestamp": "2024-01-01T12:00:00Z",
//!   "operation": { /* operation-specific data */ },
//!   "metadata": { /* context and status information */ }
//! }
//! ```
//!
//! ## Usage Examples
//!
//! ```rust
//! use stigmergy::{SavefileManager, SaveEntry, SaveOperation, SaveMetadata, Entity};
//! use std::path::PathBuf;
//!
//! // Create a savefile manager
//! let manager = SavefileManager::new(PathBuf::from("operations.jsonl"));
//!
//! // Create and log an operation
//! let entity = Entity::new([1u8; 32]);
//! let operation = SaveOperation::EntityCreate { entity, was_random: false };
//! let entry = SaveEntry::new(operation, SaveMetadata::system());
//! manager.save_or_error(&entry);
//! ```

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

use crate::{
    Component, ComponentDefinition, DataStore, DataStoreOperations, Entity, data_operations,
};

/// A complete record of a single operation in the stigmergy system.
///
/// Each save entry represents one operation that occurred, including its unique identifier,
/// timestamp, the specific operation details, and contextual metadata. This enables
/// comprehensive auditing, debugging, and replay capabilities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveEntry {
    /// Unique identifier for this save entry
    pub id: String,

    /// Timestamp when the operation occurred
    pub timestamp: DateTime<Utc>,

    /// The specific operation that was performed
    pub operation: SaveOperation,

    /// Additional metadata about the operation context
    pub metadata: SaveMetadata,
}

/// All possible operations that can be logged in the stigmergy system.
///
/// This enum represents every type of operation that can occur, from entity management
/// to component definitions and component instances. Each variant contains the specific
/// data needed to understand and potentially replay the operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SaveOperation {
    // Entity operations
    /// An entity was created in the system.
    EntityCreate {
        /// The entity that was created
        entity: Entity,
        /// Whether the entity ID was randomly generated
        was_random: bool,
    },

    /// An entity was deleted from the system.
    EntityDelete {
        /// The ID string of the entity that was deleted
        entity_id: String,
        /// Whether the deletion was successful
        success: bool,
    },

    // Component Definition operations
    /// A component definition was created.
    ComponentDefinitionCreate {
        /// The component definition that was created
        definition: ComponentDefinition,
        /// The result of validating the definition's schema
        validation_result: ValidationResult,
    },
    /// A component definition was updated.
    ComponentDefinitionUpdate {
        /// The ID of the component definition being updated
        definition_id: String,
        /// The previous definition (if it existed)
        old_definition: Option<ComponentDefinition>,
        /// The new definition after update
        new_definition: ComponentDefinition,
        /// The result of validating the new definition's schema
        validation_result: ValidationResult,
    },
    /// A component definition was modified via patch operation.
    ComponentDefinitionPatch {
        /// The ID of the component definition being patched
        definition_id: String,
        /// The patch data applied to the definition
        patch_data: Value,
        /// The resulting definition after applying the patch
        result_definition: ComponentDefinition,
    },
    /// A component definition was deleted.
    ComponentDefinitionDelete {
        /// The ID of the component definition that was deleted
        definition_id: String,
        /// The definition that was deleted (if it existed)
        deleted_definition: Option<ComponentDefinition>,
    },
    /// All component definitions were deleted.
    ComponentDefinitionDeleteAll {
        /// The number of definitions that were deleted
        count_deleted: u32,
    },
    /// A component definition was retrieved (read operation).
    ComponentDefinitionGet {
        /// The ID of the definition that was requested (None for list operations)
        definition_id: Option<String>,
        /// Whether the definition was found
        found: bool,
    },

    // Component Instance operations
    /// A component instance was created and attached to an entity.
    ComponentCreate {
        /// The ID of the entity the component was attached to
        entity_id: String,
        /// The type identifier of the component
        component_id: String,
        /// The component data that was stored
        component_data: Value,
        /// The result of validating the component data (if validation was performed)
        validation_result: Option<ValidationResult>,
    },
    /// A component instance was updated.
    ComponentUpdate {
        /// The ID of the entity owning the component
        entity_id: String,
        /// The type identifier of the component
        component_id: String,
        /// The previous component data (if it existed)
        old_data: Option<Value>,
        /// The new component data after update
        new_data: Value,
        /// The result of validating the new data (if validation was performed)
        validation_result: Option<ValidationResult>,
    },
    /// A component instance was modified via patch operation.
    ComponentPatch {
        /// The ID of the entity owning the component
        entity_id: String,
        /// The type identifier of the component
        component_id: String,
        /// The patch data applied to the component
        patch_data: Value,
        /// The resulting component data after applying the patch
        result_data: Value,
    },
    /// A component instance was deleted from an entity.
    ComponentDelete {
        /// The ID of the entity the component was removed from
        entity_id: String,
        /// The type identifier of the component
        component_id: String,
        /// The component data that was deleted (if it existed)
        deleted_data: Option<Value>,
    },
    /// All component instances were deleted.
    ComponentDeleteAll {
        /// The number of component instances that were deleted
        count_deleted: u32,
    },
    /// A component instance was retrieved (read operation).
    ComponentGet {
        /// The type identifier of the component that was requested (None for list operations)
        component_id: Option<String>,
        /// Whether the component was found
        found: bool,
    },

    // System operations
    /// A system was created.
    SystemCreate {
        /// The ID of the system that was created
        system_id: String,
        /// The system configuration (if creation was successful)
        config: Option<crate::SystemConfig>,
        /// Whether the creation was successful
        success: bool,
    },
    /// A system was updated.
    SystemUpdate {
        /// The ID of the system that was updated
        system_id: String,
        /// The previous system configuration (if it existed)
        old_config: Option<crate::SystemConfig>,
        /// The new system configuration
        new_config: crate::SystemConfig,
        /// Whether the update was successful
        success: bool,
    },
    /// A system was modified via patch operation.
    SystemPatch {
        /// The ID of the system that was patched
        system_id: String,
        /// The patch data applied to the system
        patch_data: Value,
        /// Whether the patch was successful
        success: bool,
    },
    /// A system was deleted.
    SystemDelete {
        /// The ID of the system that was deleted
        system_id: String,
        /// Whether the deletion was successful
        success: bool,
    },
    /// All systems were deleted.
    SystemDeleteAll {
        /// The number of systems that were deleted
        count_deleted: u32,
    },
    /// A system was retrieved (read operation).
    SystemGet {
        /// The ID of the system that was requested
        system_id: String,
        /// Whether the system was found and retrieved successfully
        success: bool,
    },

    // Validation operations
    /// A validation operation was performed.
    ValidationPerformed {
        /// The type of validation that was performed
        target_type: ValidationType,
        /// The ID of the target that was validated
        target_id: String,
        /// The result of the validation operation
        result: ValidationResult,
    },

    // Schema operations
    /// A schema generation operation was performed.
    SchemaGeneration {
        /// The type of schema that was generated
        schema_type: String,
        /// The generated schema
        result_schema: Value,
        /// Whether the generation was successful
        success: bool,
    },
}

/// Result of a validation operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationResult {
    /// The validation was successful
    Success,
    /// The validation failed with error details
    Failed {
        /// The primary error message
        error: String,
        /// Additional details about the error (optional)
        details: Option<String>,
    },
}

/// Types of validation that can be performed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationType {
    /// Validation of a component definition's JSON schema
    ComponentDefinitionSchema,
    /// Validation of component instance data against a schema
    ComponentInstanceData,
    /// Validation of an enumeration schema
    EnumSchema,
    /// General JSON schema validation
    GeneralSchema,
}

/// Metadata about the operation context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveMetadata {
    /// Source of the operation (e.g., "REST API", "Internal")
    pub source: String,

    /// User or system identifier that initiated the operation
    pub initiator: Option<String>,

    /// Request ID for correlation across distributed systems
    pub request_id: Option<String>,

    /// Additional context-specific data
    pub context: Option<Value>,

    /// Performance timing information
    pub duration_ms: Option<u64>,

    /// Result status of the operation
    pub status: OperationStatus,
}

/// Status of an operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OperationStatus {
    /// The operation completed successfully
    Success,
    /// The operation failed
    Failed,
    /// The operation partially completed
    Partial,
    /// The operation was cancelled
    Cancelled,
}

impl SaveEntry {
    /// Creates a new save entry with a generated ID and current timestamp
    pub fn new(operation: SaveOperation, metadata: SaveMetadata) -> Self {
        Self {
            id: Self::generate_id(),
            timestamp: Utc::now(),
            operation,
            metadata,
        }
    }

    /// Creates a new save entry with a specific timestamp (useful for testing)
    pub fn with_timestamp(
        operation: SaveOperation,
        metadata: SaveMetadata,
        timestamp: DateTime<Utc>,
    ) -> Self {
        Self {
            id: Self::generate_id(),
            timestamp,
            operation,
            metadata,
        }
    }

    /// Generates a unique ID for the save entry
    fn generate_id() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        format!("save_{}", timestamp)
    }

    /// Returns the operation type as a string for filtering and indexing
    pub fn operation_type(&self) -> &'static str {
        match &self.operation {
            SaveOperation::EntityCreate { .. } => "EntityCreate",
            SaveOperation::EntityDelete { .. } => "EntityDelete",
            SaveOperation::ComponentDefinitionCreate { .. } => "ComponentDefinitionCreate",
            SaveOperation::ComponentDefinitionUpdate { .. } => "ComponentDefinitionUpdate",
            SaveOperation::ComponentDefinitionPatch { .. } => "ComponentDefinitionPatch",
            SaveOperation::ComponentDefinitionDelete { .. } => "ComponentDefinitionDelete",
            SaveOperation::ComponentDefinitionDeleteAll { .. } => "ComponentDefinitionDeleteAll",
            SaveOperation::ComponentDefinitionGet { .. } => "ComponentDefinitionGet",
            SaveOperation::ComponentCreate { .. } => "ComponentCreate",
            SaveOperation::ComponentUpdate { .. } => "ComponentUpdate",
            SaveOperation::ComponentPatch { .. } => "ComponentPatch",
            SaveOperation::ComponentDelete { .. } => "ComponentDelete",
            SaveOperation::ComponentDeleteAll { .. } => "ComponentDeleteAll",
            SaveOperation::ComponentGet { .. } => "ComponentGet",
            SaveOperation::SystemCreate { .. } => "SystemCreate",
            SaveOperation::SystemUpdate { .. } => "SystemUpdate",
            SaveOperation::SystemPatch { .. } => "SystemPatch",
            SaveOperation::SystemDelete { .. } => "SystemDelete",
            SaveOperation::SystemDeleteAll { .. } => "SystemDeleteAll",
            SaveOperation::SystemGet { .. } => "SystemGet",
            SaveOperation::ValidationPerformed { .. } => "ValidationPerformed",
            SaveOperation::SchemaGeneration { .. } => "SchemaGeneration",
        }
    }

    /// Returns true if the operation was successful
    pub fn is_success(&self) -> bool {
        matches!(self.metadata.status, OperationStatus::Success)
    }

    /// Returns true if the operation failed
    pub fn is_failure(&self) -> bool {
        matches!(self.metadata.status, OperationStatus::Failed)
    }

    /// Returns the entity ID if this operation involves an entity
    pub fn entity_id(&self) -> Option<String> {
        match &self.operation {
            SaveOperation::EntityCreate { entity, .. } => Some(entity.to_string()),
            SaveOperation::EntityDelete { entity_id, .. } => Some(entity_id.clone()),
            _ => None,
        }
    }

    /// Returns the component definition ID if this operation involves a component definition
    pub fn component_definition_id(&self) -> Option<String> {
        match &self.operation {
            SaveOperation::ComponentDefinitionCreate { definition, .. } => {
                Some(format!("{:?}", definition.component))
            }
            SaveOperation::ComponentDefinitionUpdate { definition_id, .. }
            | SaveOperation::ComponentDefinitionPatch { definition_id, .. }
            | SaveOperation::ComponentDefinitionDelete { definition_id, .. }
            | SaveOperation::ComponentDefinitionGet {
                definition_id: Some(definition_id),
                ..
            } => Some(definition_id.clone()),
            _ => None,
        }
    }

    /// Returns the component instance ID if this operation involves a component instance
    pub fn component_id(&self) -> Option<String> {
        match &self.operation {
            SaveOperation::ComponentCreate { component_id, .. }
            | SaveOperation::ComponentUpdate { component_id, .. }
            | SaveOperation::ComponentPatch { component_id, .. }
            | SaveOperation::ComponentDelete { component_id, .. }
            | SaveOperation::ComponentGet {
                component_id: Some(component_id),
                ..
            } => Some(component_id.clone()),
            _ => None,
        }
    }

    /// Returns the system ID if this operation involves a system
    pub fn system_id(&self) -> Option<String> {
        match &self.operation {
            SaveOperation::SystemCreate { system_id, .. }
            | SaveOperation::SystemUpdate { system_id, .. }
            | SaveOperation::SystemPatch { system_id, .. }
            | SaveOperation::SystemDelete { system_id, .. }
            | SaveOperation::SystemGet { system_id, .. } => Some(system_id.clone()),
            _ => None,
        }
    }
}

impl SaveMetadata {
    /// Creates basic metadata for REST API operations
    pub fn rest_api(request_id: Option<String>) -> Self {
        Self {
            source: "REST API".to_string(),
            initiator: None,
            request_id,
            context: None,
            duration_ms: None,
            status: OperationStatus::Success,
        }
    }

    /// Creates metadata for internal system operations
    pub fn internal(initiator: Option<String>) -> Self {
        Self {
            source: "Internal".to_string(),
            initiator,
            request_id: None,
            context: None,
            duration_ms: None,
            status: OperationStatus::Success,
        }
    }

    /// Creates metadata for system operations
    pub fn system() -> Self {
        Self {
            source: "System".to_string(),
            initiator: None,
            request_id: None,
            context: None,
            duration_ms: None,
            status: OperationStatus::Success,
        }
    }

    /// Sets the operation status
    pub fn with_status(mut self, status: OperationStatus) -> Self {
        self.status = status;
        self
    }

    /// Sets the operation duration
    pub fn with_duration(mut self, duration_ms: u64) -> Self {
        self.duration_ms = Some(duration_ms);
        self
    }

    /// Sets additional context data
    pub fn with_context(mut self, context: Value) -> Self {
        self.context = Some(context);
        self
    }

    /// Sets the initiator
    pub fn with_initiator(mut self, initiator: String) -> Self {
        self.initiator = Some(initiator);
        self
    }
}

impl ValidationResult {
    /// Creates a successful validation result
    pub fn success() -> Self {
        Self::Success
    }

    /// Creates a failed validation result with error details
    pub fn failed(error: impl Into<String>) -> Self {
        Self::Failed {
            error: error.into(),
            details: None,
        }
    }

    /// Creates a failed validation result with error and detailed information
    pub fn failed_with_details(error: impl Into<String>, details: impl Into<String>) -> Self {
        Self::Failed {
            error: error.into(),
            details: Some(details.into()),
        }
    }

    /// Returns true if the validation was successful
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success)
    }

    /// Returns true if the validation failed
    pub fn is_failure(&self) -> bool {
        matches!(self, Self::Failed { .. })
    }

    /// Gets the error message if validation failed
    pub fn error_message(&self) -> Option<&str> {
        match self {
            Self::Failed { error, .. } => Some(error),
            Self::Success => None,
        }
    }
}

/// Convenience macros for creating save entries
#[macro_export]
macro_rules! save_entity_create {
    ($entity:expr, $was_random:expr) => {
        $crate::SaveEntry::new(
            $crate::SaveOperation::EntityCreate {
                entity: $entity,
                was_random: $was_random,
            },
            $crate::SaveMetadata::rest_api(None),
        )
    };
}

#[macro_export]
/// Create a SaveEntry for an entity deletion operation.
///
/// # Arguments
///
/// * `entity_id` - The entity ID that was deleted
/// * `success` - Whether the deletion was successful
///
/// # Examples
///
/// ```rust
/// # use stigmergy::save_entity_delete;
/// let log_entry = save_entity_delete!("entity:ABC123", true);
/// ```
macro_rules! save_entity_delete {
    ($entity_id:expr, $success:expr) => {
        $crate::SaveEntry::new(
            $crate::SaveOperation::EntityDelete {
                entity_id: $entity_id.to_string(),
                success: $success,
            },
            $crate::SaveMetadata::rest_api(None),
        )
    };
}

#[macro_export]
/// Create a SaveEntry for a component creation operation.
///
/// # Arguments
///
/// * `entity_id` - The entity ID the component was attached to
/// * `component_id` - The component type identifier
/// * `data` - The component data that was stored
/// * `validation` - Optional validation result
///
/// # Examples
///
/// ```rust
/// # use stigmergy::{save_component_create, ValidationResult};
/// # use serde_json::json;
/// let log_entry = save_component_create!(
///     "entity:ABC123",
///     "Position",
///     json!({"x": 1.0, "y": 2.0}),
///     Some(ValidationResult::success())
/// );
/// ```
macro_rules! save_component_create {
    ($entity_id:expr, $component_id:expr, $data:expr, $validation:expr) => {
        $crate::SaveEntry::new(
            $crate::SaveOperation::ComponentCreate {
                entity_id: $entity_id.to_string(),
                component_id: $component_id.to_string(),
                component_data: $data,
                validation_result: $validation,
            },
            $crate::SaveMetadata::rest_api(None),
        )
    };
}

/// Savefile manager that writes save entries to JSONL files
pub struct SavefileManager {
    savefile_path: PathBuf,
}

impl SavefileManager {
    /// Creates a new durable logger with the specified file path
    pub fn new(savefile_path: PathBuf) -> Self {
        Self { savefile_path }
    }

    /// Creates a new durable logger using the default log file path
    pub fn with_default_path() -> Self {
        Self::new(PathBuf::from("stigmergy.jsonl"))
    }

    /// Writes a save entry to the JSONL file
    pub fn save(&self, entry: &SaveEntry) -> Result<(), std::io::Error> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.savefile_path)?;

        let json_line = serde_json::to_string(entry)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        writeln!(file, "{}", json_line)?;
        file.flush()?;
        Ok(())
    }

    /// Saves an entry and prints an error message if saving fails
    pub fn save_or_error(&self, entry: &SaveEntry) {
        if let Err(e) = self.save(entry) {
            eprintln!("Failed to write save entry: {}", e);
        }
    }

    /// Loads all save entries from the JSONL file
    pub fn load_entries(&self) -> Result<Vec<SaveEntry>, std::io::Error> {
        if !self.savefile_path.exists() {
            return Ok(Vec::new());
        }

        let file = std::fs::File::open(&self.savefile_path)?;
        let reader = BufReader::new(file);
        let mut entries = Vec::new();

        for line in reader.lines() {
            let line = line?;
            if !line.trim().is_empty() {
                match serde_json::from_str::<SaveEntry>(&line) {
                    Ok(entry) => entries.push(entry),
                    Err(e) => {
                        eprintln!("Failed to parse save entry: {} - Line: {}", e, line);
                    }
                }
            }
        }

        Ok(entries)
    }

    /// Restores successful operations from the savefile against a data store
    /// Only restores operations that were originally successful to avoid duplicating failures
    pub fn restore_to_store(
        &self,
        data_store: &dyn DataStore,
    ) -> Result<RestoreResult, std::io::Error> {
        let entries = self.load_entries()?;
        let mut result = RestoreResult::new();

        for entry in entries {
            // Only restore operations that were originally successful
            if !entry.is_success() {
                result.skipped += 1;
                continue;
            }

            match self.restore_single_operation(&entry, data_store) {
                Ok(_) => result.successful += 1,
                Err(e) => {
                    result.failed += 1;
                    result.errors.push(format!(
                        "Failed to restore {}: {}",
                        entry.operation_type(),
                        e
                    ));
                }
            }
        }

        Ok(result)
    }

    /// Restores a single save operation against a data store
    fn restore_single_operation(
        &self,
        entry: &SaveEntry,
        data_store: &dyn DataStore,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match &entry.operation {
            SaveOperation::EntityCreate { entity, .. } => {
                // Create entity - AlreadyExists is ok for restore, other errors fail
                let result = data_operations::replay::create_entity(data_store, entity);
                if !result.success {
                    // This is a real error (not AlreadyExists), so fail the restore
                    return Err(result.into_error().into());
                }
                // result.data indicates if entity was created (true) or already existed (false)
                // Both are acceptable for restore
                Ok(())
            }

            SaveOperation::EntityDelete { entity_id, success } => {
                if *success {
                    let entity = entity_id.parse::<Entity>().map_err(|_| {
                        std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            format!("invalid entity id: {}", entity_id),
                        )
                    })?;
                    let result = DataStoreOperations::delete_entity(data_store, &entity);
                    if !result.success {
                        return Err(result.into_error().into());
                    }
                }
                Ok(())
            }

            SaveOperation::ComponentDefinitionCreate { definition, .. } => {
                // Create component definition - AlreadyExists is ok for replay, other errors fail
                let result = data_operations::replay::create_component_definition(
                    data_store,
                    &definition.component,
                    definition,
                );
                if !result.success {
                    // This is a real error (not AlreadyExists), so fail the restore
                    return Err(result.into_error().into());
                }
                Ok(())
            }

            SaveOperation::ComponentDefinitionUpdate { new_definition, .. } => {
                let result = DataStoreOperations::update_component_definition(
                    data_store,
                    &new_definition.component,
                    new_definition,
                );
                if !result.success {
                    return Err(result.into_error().into());
                }
                Ok(())
            }

            SaveOperation::ComponentDefinitionDelete { definition_id, .. } => {
                let component = Component::new(definition_id).ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("invalid component name: {}", definition_id),
                    )
                })?;
                let result =
                    DataStoreOperations::delete_component_definition(data_store, &component);
                if !result.success {
                    return Err(result.into_error().into());
                }
                Ok(())
            }

            SaveOperation::ComponentDefinitionDeleteAll { .. } => {
                let result = DataStoreOperations::delete_all_component_definitions(data_store);
                if !result.success {
                    return Err(result.into_error().into());
                }
                Ok(())
            }

            SaveOperation::ComponentCreate {
                entity_id,
                component_id,
                component_data,
                ..
            } => {
                let entity = entity_id.parse::<Entity>()?;
                let component = Component::new(component_id).ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("invalid component name: {}", component_id),
                    )
                })?;
                // Create component - AlreadyExists is ok for replay, other errors fail
                let result = data_operations::replay::create_component(
                    data_store,
                    &entity,
                    &component,
                    component_data,
                );
                if !result.success {
                    // This is a real error (not AlreadyExists), so fail the restore
                    return Err(result.into_error().into());
                }
                Ok(())
            }

            SaveOperation::ComponentUpdate {
                entity_id,
                component_id,
                new_data,
                ..
            } => {
                let entity = entity_id.parse::<Entity>()?;
                let component = Component::new(component_id).ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("invalid component name: {}", component_id),
                    )
                })?;
                let result = DataStoreOperations::update_component(
                    data_store, &entity, &component, new_data,
                );
                if !result.success {
                    return Err(result.into_error().into());
                }
                Ok(())
            }

            SaveOperation::ComponentDelete {
                entity_id,
                component_id,
                ..
            } => {
                let entity = entity_id.parse::<Entity>()?;
                let component = Component::new(component_id).ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("invalid component name: {}", component_id),
                    )
                })?;
                let result = DataStoreOperations::delete_component(data_store, &entity, &component);
                if !result.success {
                    return Err(result.into_error().into());
                }
                Ok(())
            }

            SaveOperation::ComponentDeleteAll { .. } => {
                let result = DataStoreOperations::delete_all_components(data_store);
                if !result.success {
                    return Err(result.into_error().into());
                }
                Ok(())
            }

            SaveOperation::ComponentDefinitionPatch {
                result_definition, ..
            } => {
                let result = DataStoreOperations::update_component_definition(
                    data_store,
                    &result_definition.component,
                    result_definition,
                );
                if !result.success {
                    return Err(result.into_error().into());
                }
                Ok(())
            }

            SaveOperation::ComponentPatch {
                entity_id,
                component_id,
                result_data,
                ..
            } => {
                let entity = entity_id.parse::<Entity>()?;
                let component = Component::new(component_id).ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("invalid component name: {}", component_id),
                    )
                })?;
                let result = DataStoreOperations::update_component(
                    data_store,
                    &entity,
                    &component,
                    result_data,
                );
                if !result.success {
                    return Err(result.into_error().into());
                }
                Ok(())
            }

            // System operations - currently no-op since we don't have data store integration yet
            SaveOperation::SystemCreate { .. }
            | SaveOperation::SystemUpdate { .. }
            | SaveOperation::SystemPatch { .. }
            | SaveOperation::SystemDelete { .. }
            | SaveOperation::SystemDeleteAll { .. }
            | SaveOperation::SystemGet { .. } => {
                // Skip system operations until data store integration is implemented
                Ok(())
            }

            // These operations are read-only or metadata, so we skip them in replay
            SaveOperation::ComponentDefinitionGet { .. }
            | SaveOperation::ComponentGet { .. }
            | SaveOperation::ValidationPerformed { .. }
            | SaveOperation::SchemaGeneration { .. } => {
                // Skip read-only operations and metadata
                Ok(())
            }
        }
    }
}

/// Result of restoring a savefile against a data store
#[derive(Debug, Clone)]
pub struct RestoreResult {
    /// The number of operations successfully restored
    pub successful: u32,
    /// The number of operations that failed during restoration
    pub failed: u32,
    /// The number of operations that were skipped (e.g., failed original operations)
    pub skipped: u32,
    /// Error messages from failed restoration operations
    pub errors: Vec<String>,
}

impl RestoreResult {
    /// Create a new RestoreResult with zero counts and empty error list.
    pub fn new() -> Self {
        Self {
            successful: 0,
            failed: 0,
            skipped: 0,
            errors: Vec::new(),
        }
    }

    /// Calculate the total number of operations that were processed (successful + failed).
    ///
    /// Skipped operations are not included in this count as they were not actually processed.
    pub fn total_processed(&self) -> u32 {
        self.successful + self.failed
    }
}

impl Default for RestoreResult {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for RestoreResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Restore completed: {} successful, {} failed, {} skipped",
            self.successful, self.failed, self.skipped
        )?;
        if !self.errors.is_empty() {
            write!(f, "\nErrors:\n{}", self.errors.join("\n"))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Component;
    use chrono::{TimeZone, Utc};
    use serde_json::json;

    fn test_entity() -> Entity {
        Entity::new([1u8; 32])
    }

    fn test_component() -> Component {
        Component::new("TestComponent").unwrap()
    }

    fn test_component_definition() -> ComponentDefinition {
        ComponentDefinition::new(test_component(), json!({"type": "string"}))
    }

    #[test]
    fn save_entry_creation() {
        let operation = SaveOperation::EntityCreate {
            entity: test_entity(),
            was_random: true,
        };
        let metadata = SaveMetadata::rest_api(Some("req_123".to_string()));
        let entry = SaveEntry::new(operation, metadata);

        assert!(!entry.id.is_empty());
        assert!(entry.id.starts_with("save_"));
        assert_eq!(entry.operation_type(), "EntityCreate");
        assert!(entry.is_success());
        assert!(!entry.is_failure());
    }

    #[test]
    fn save_entry_with_timestamp() {
        let timestamp = Utc.with_ymd_and_hms(2023, 1, 1, 12, 0, 0).unwrap();
        let operation = SaveOperation::EntityDelete {
            entity_id: "entity_123".to_string(),
            success: true,
        };
        let metadata = SaveMetadata::internal(Some("system".to_string()));
        let entry = SaveEntry::with_timestamp(operation, metadata, timestamp);

        assert_eq!(entry.timestamp, timestamp);
        assert_eq!(entry.operation_type(), "EntityDelete");
    }

    #[test]
    fn entity_operations() {
        let entity = test_entity();
        let entity_id = entity.to_string();

        let create_op = SaveOperation::EntityCreate {
            entity,
            was_random: false,
        };
        let create_entry = SaveEntry::new(create_op, SaveMetadata::rest_api(None));
        assert_eq!(create_entry.operation_type(), "EntityCreate");
        assert_eq!(create_entry.entity_id(), Some(entity_id.clone()));

        let delete_op = SaveOperation::EntityDelete {
            entity_id: entity_id.clone(),
            success: true,
        };
        let delete_entry = SaveEntry::new(delete_op, SaveMetadata::rest_api(None));
        assert_eq!(delete_entry.operation_type(), "EntityDelete");
        assert_eq!(delete_entry.entity_id(), Some(entity_id));
    }

    #[test]
    fn component_definition_operations() {
        let definition = test_component_definition();

        let create_op = SaveOperation::ComponentDefinitionCreate {
            definition: definition.clone(),
            validation_result: ValidationResult::success(),
        };
        let create_entry = SaveEntry::new(create_op, SaveMetadata::rest_api(None));
        assert_eq!(create_entry.operation_type(), "ComponentDefinitionCreate");
        assert!(create_entry.component_definition_id().is_some());

        let update_op = SaveOperation::ComponentDefinitionUpdate {
            definition_id: "def_123".to_string(),
            old_definition: Some(definition.clone()),
            new_definition: definition,
            validation_result: ValidationResult::failed("Invalid schema"),
        };
        let update_entry = SaveEntry::new(update_op, SaveMetadata::rest_api(None));
        assert_eq!(update_entry.operation_type(), "ComponentDefinitionUpdate");
        assert_eq!(
            update_entry.component_definition_id(),
            Some("def_123".to_string())
        );

        let patch_op = SaveOperation::ComponentDefinitionPatch {
            definition_id: "def_456".to_string(),
            patch_data: json!({"schema": {"type": "number"}}),
            result_definition: test_component_definition(),
        };
        let patch_entry = SaveEntry::new(patch_op, SaveMetadata::rest_api(None));
        assert_eq!(patch_entry.operation_type(), "ComponentDefinitionPatch");

        let delete_op = SaveOperation::ComponentDefinitionDelete {
            definition_id: "def_789".to_string(),
            deleted_definition: None,
        };
        let delete_entry = SaveEntry::new(delete_op, SaveMetadata::rest_api(None));
        assert_eq!(delete_entry.operation_type(), "ComponentDefinitionDelete");

        let delete_all_op = SaveOperation::ComponentDefinitionDeleteAll { count_deleted: 5 };
        let delete_all_entry = SaveEntry::new(delete_all_op, SaveMetadata::rest_api(None));
        assert_eq!(
            delete_all_entry.operation_type(),
            "ComponentDefinitionDeleteAll"
        );

        let get_op = SaveOperation::ComponentDefinitionGet {
            definition_id: Some("def_get".to_string()),
            found: true,
        };
        let get_entry = SaveEntry::new(get_op, SaveMetadata::rest_api(None));
        assert_eq!(get_entry.operation_type(), "ComponentDefinitionGet");
    }

    #[test]
    fn component_instance_operations() {
        let component_data = json!({"color": "red"});

        let create_op = SaveOperation::ComponentCreate {
            entity_id: "entity:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA".to_string(),
            component_id: "comp_123".to_string(),
            component_data: component_data.clone(),
            validation_result: Some(ValidationResult::success()),
        };
        let create_entry = SaveEntry::new(create_op, SaveMetadata::rest_api(None));
        assert_eq!(create_entry.operation_type(), "ComponentCreate");
        assert_eq!(create_entry.component_id(), Some("comp_123".to_string()));

        let update_op = SaveOperation::ComponentUpdate {
            entity_id: "entity:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA".to_string(),
            component_id: "comp_456".to_string(),
            old_data: Some(json!({"color": "blue"})),
            new_data: component_data.clone(),
            validation_result: None,
        };
        let update_entry = SaveEntry::new(update_op, SaveMetadata::rest_api(None));
        assert_eq!(update_entry.operation_type(), "ComponentUpdate");

        let patch_op = SaveOperation::ComponentPatch {
            entity_id: "entity:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA".to_string(),
            component_id: "comp_789".to_string(),
            patch_data: json!({"color": "green"}),
            result_data: component_data,
        };
        let patch_entry = SaveEntry::new(patch_op, SaveMetadata::rest_api(None));
        assert_eq!(patch_entry.operation_type(), "ComponentPatch");

        let delete_op = SaveOperation::ComponentDelete {
            entity_id: "entity:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA".to_string(),
            component_id: "comp_delete".to_string(),
            deleted_data: Some(json!({"color": "yellow"})),
        };
        let delete_entry = SaveEntry::new(delete_op, SaveMetadata::rest_api(None));
        assert_eq!(delete_entry.operation_type(), "ComponentDelete");

        let delete_all_op = SaveOperation::ComponentDeleteAll { count_deleted: 3 };
        let delete_all_entry = SaveEntry::new(delete_all_op, SaveMetadata::rest_api(None));
        assert_eq!(delete_all_entry.operation_type(), "ComponentDeleteAll");

        let get_op = SaveOperation::ComponentGet {
            component_id: None,
            found: false,
        };
        let get_entry = SaveEntry::new(get_op, SaveMetadata::rest_api(None));
        assert_eq!(get_entry.operation_type(), "ComponentGet");
        assert_eq!(get_entry.component_id(), None);
    }

    #[test]
    fn validation_and_schema_operations() {
        let validation_op = SaveOperation::ValidationPerformed {
            target_type: ValidationType::ComponentDefinitionSchema,
            target_id: "target_123".to_string(),
            result: ValidationResult::failed_with_details(
                "Schema validation failed",
                "Missing required field 'type'",
            ),
        };
        let validation_entry = SaveEntry::new(validation_op, SaveMetadata::internal(None));
        assert_eq!(validation_entry.operation_type(), "ValidationPerformed");

        let schema_op = SaveOperation::SchemaGeneration {
            schema_type: "enum".to_string(),
            result_schema: json!({"oneOf": [{"type": "string"}]}),
            success: true,
        };
        let schema_entry = SaveEntry::new(schema_op, SaveMetadata::internal(None));
        assert_eq!(schema_entry.operation_type(), "SchemaGeneration");
    }

    #[test]
    fn validation_result_functionality() {
        let success = ValidationResult::success();
        assert!(success.is_success());
        assert!(!success.is_failure());
        assert!(success.error_message().is_none());

        let failure = ValidationResult::failed("Validation error");
        assert!(!failure.is_success());
        assert!(failure.is_failure());
        assert_eq!(failure.error_message(), Some("Validation error"));

        let detailed_failure =
            ValidationResult::failed_with_details("Schema error", "Property 'name' is required");
        assert!(!detailed_failure.is_success());
        assert_eq!(detailed_failure.error_message(), Some("Schema error"));
    }

    #[test]
    fn metadata_builders() {
        let rest_metadata = SaveMetadata::rest_api(Some("req_456".to_string()));
        assert_eq!(rest_metadata.source, "REST API");
        assert_eq!(rest_metadata.request_id, Some("req_456".to_string()));
        assert!(matches!(rest_metadata.status, OperationStatus::Success));

        let internal_metadata = SaveMetadata::internal(Some("scheduler".to_string()));
        assert_eq!(internal_metadata.source, "Internal");
        assert_eq!(internal_metadata.initiator, Some("scheduler".to_string()));

        let enhanced_metadata = SaveMetadata::rest_api(None)
            .with_status(OperationStatus::Failed)
            .with_duration(150)
            .with_context(json!({"retry_count": 3}))
            .with_initiator("user_123".to_string());

        assert!(matches!(enhanced_metadata.status, OperationStatus::Failed));
        assert_eq!(enhanced_metadata.duration_ms, Some(150));
        assert_eq!(enhanced_metadata.initiator, Some("user_123".to_string()));
        assert!(enhanced_metadata.context.is_some());
    }

    #[test]
    fn serialization_round_trip() {
        let operation = SaveOperation::ComponentCreate {
            entity_id: "entity:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA".to_string(),
            component_id: "test_comp".to_string(),
            component_data: json!({"value": 42}),
            validation_result: Some(ValidationResult::success()),
        };
        let metadata = SaveMetadata::rest_api(Some("req_789".to_string()))
            .with_duration(100)
            .with_status(OperationStatus::Success);
        let entry = SaveEntry::new(operation, metadata);

        let serialized = serde_json::to_string(&entry).unwrap();
        assert!(!serialized.is_empty());

        let deserialized: SaveEntry = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.id, entry.id);
        assert_eq!(deserialized.operation_type(), entry.operation_type());
    }

    #[test]
    fn macro_usage() {
        let entity = test_entity();
        let create_log = save_entity_create!(entity, true);
        assert_eq!(create_log.operation_type(), "EntityCreate");

        let delete_log = save_entity_delete!("entity_456", false);
        assert_eq!(delete_log.operation_type(), "EntityDelete");

        let component_log = save_component_create!(
            "entity:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
            "comp_789",
            json!({"test": "data"}),
            Some(ValidationResult::success())
        );
        assert_eq!(component_log.operation_type(), "ComponentCreate");
    }

    #[test]
    fn operation_status_checks() {
        let success_entry = SaveEntry::new(
            SaveOperation::EntityCreate {
                entity: test_entity(),
                was_random: false,
            },
            SaveMetadata::rest_api(None),
        );
        assert!(success_entry.is_success());
        assert!(!success_entry.is_failure());

        let failed_entry = SaveEntry::new(
            SaveOperation::EntityDelete {
                entity_id: "bad_id".to_string(),
                success: false,
            },
            SaveMetadata::rest_api(None).with_status(OperationStatus::Failed),
        );
        assert!(!failed_entry.is_success());
        assert!(failed_entry.is_failure());
    }

    #[test]
    fn id_extraction_methods() {
        let entity = test_entity();
        let entity_id = entity.to_string();

        let entity_entry = SaveEntry::new(
            SaveOperation::EntityCreate {
                entity,
                was_random: false,
            },
            SaveMetadata::rest_api(None),
        );
        assert_eq!(entity_entry.entity_id(), Some(entity_id));
        assert_eq!(entity_entry.component_definition_id(), None);
        assert_eq!(entity_entry.component_id(), None);

        let comp_def_entry = SaveEntry::new(
            SaveOperation::ComponentDefinitionUpdate {
                definition_id: "def_123".to_string(),
                old_definition: None,
                new_definition: test_component_definition(),
                validation_result: ValidationResult::success(),
            },
            SaveMetadata::rest_api(None),
        );
        assert_eq!(comp_def_entry.entity_id(), None);
        assert_eq!(
            comp_def_entry.component_definition_id(),
            Some("def_123".to_string())
        );
        assert_eq!(comp_def_entry.component_id(), None);

        let comp_entry = SaveEntry::new(
            SaveOperation::ComponentPatch {
                entity_id: "entity:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA".to_string(),
                component_id: "comp_456".to_string(),
                patch_data: json!({}),
                result_data: json!({}),
            },
            SaveMetadata::rest_api(None),
        );
        assert_eq!(comp_entry.entity_id(), None);
        assert_eq!(comp_entry.component_definition_id(), None);
        assert_eq!(comp_entry.component_id(), Some("comp_456".to_string()));
    }

    #[test]
    fn validation_type_variants() {
        let validation_types = vec![
            ValidationType::ComponentDefinitionSchema,
            ValidationType::ComponentInstanceData,
            ValidationType::EnumSchema,
            ValidationType::GeneralSchema,
        ];

        for validation_type in validation_types {
            let operation = SaveOperation::ValidationPerformed {
                target_type: validation_type,
                target_id: "test_target".to_string(),
                result: ValidationResult::success(),
            };
            let entry = SaveEntry::new(operation, SaveMetadata::internal(None));
            assert_eq!(entry.operation_type(), "ValidationPerformed");
        }
    }

    #[test]
    fn operation_status_variants() {
        let statuses = vec![
            OperationStatus::Success,
            OperationStatus::Failed,
            OperationStatus::Partial,
            OperationStatus::Cancelled,
        ];

        for status in statuses {
            let metadata = SaveMetadata::rest_api(None).with_status(status);
            let entry = SaveEntry::new(
                SaveOperation::EntityCreate {
                    entity: test_entity(),
                    was_random: false,
                },
                metadata,
            );

            match entry.metadata.status {
                OperationStatus::Success => assert!(entry.is_success()),
                OperationStatus::Failed => assert!(entry.is_failure()),
                OperationStatus::Partial => assert!(!entry.is_success()),
                OperationStatus::Cancelled => assert!(!entry.is_success()),
            }
        }
    }

    #[test]
    fn durable_logger_writes_to_file() {
        use std::fs;
        use std::process;
        use std::time::{SystemTime, UNIX_EPOCH};

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let test_path = PathBuf::from(format!(
            "test_savefile_manager_{}_{}.jsonl",
            process::id(),
            timestamp
        ));

        let logger = SavefileManager::new(test_path.clone());
        let operation = SaveOperation::EntityCreate {
            entity: test_entity(),
            was_random: true,
        };
        let metadata = SaveMetadata::rest_api(Some("test_request".to_string()));
        let entry = SaveEntry::new(operation, metadata);

        logger.save_or_error(&entry);

        let contents = fs::read_to_string(&test_path).expect("Failed to read log file");
        assert!(!contents.is_empty());
        assert!(contents.contains("EntityCreate"));
        assert!(contents.contains("test_request"));

        fs::remove_file(test_path).ok();
    }

    #[test]
    fn log_replay_reads_entries_correctly() {
        use std::fs;
        use std::process;
        use std::time::{SystemTime, UNIX_EPOCH};

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let test_path = PathBuf::from(format!(
            "test_replay_read_{}_{}.jsonl",
            process::id(),
            timestamp
        ));

        let logger = SavefileManager::new(test_path.clone());

        // Create multiple log entries
        let entity1 = test_entity();
        let entity2 = Entity::new([2u8; 32]);

        let entry1 = SaveEntry::new(
            SaveOperation::EntityCreate {
                entity: entity1,
                was_random: false,
            },
            SaveMetadata::rest_api(None).with_status(OperationStatus::Success),
        );

        let entry2 = SaveEntry::new(
            SaveOperation::EntityCreate {
                entity: entity2,
                was_random: true,
            },
            SaveMetadata::rest_api(None).with_status(OperationStatus::Success),
        );

        logger.save_or_error(&entry1);
        logger.save_or_error(&entry2);

        // Read back the entries
        let read_entries = logger.load_entries().expect("Failed to read log entries");
        assert_eq!(read_entries.len(), 2);

        assert_eq!(read_entries[0].operation_type(), "EntityCreate");
        assert_eq!(read_entries[1].operation_type(), "EntityCreate");

        fs::remove_file(test_path).ok();
    }

    #[test]
    fn log_replay_against_empty_store() {
        use crate::InMemoryDataStore;
        use std::fs;
        use std::process;
        use std::sync::Arc;
        use std::time::{SystemTime, UNIX_EPOCH};

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let test_path = PathBuf::from(format!(
            "test_replay_empty_{}_{}.jsonl",
            process::id(),
            timestamp
        ));

        let logger = SavefileManager::new(test_path.clone());
        let data_store = Arc::new(InMemoryDataStore::new());

        // Create log entries for various operations
        let entity = test_entity();
        let definition = test_component_definition();

        let entries = vec![
            SaveEntry::new(
                SaveOperation::EntityCreate {
                    entity,
                    was_random: false,
                },
                SaveMetadata::rest_api(None).with_status(OperationStatus::Success),
            ),
            SaveEntry::new(
                SaveOperation::ComponentDefinitionCreate {
                    definition: definition.clone(),
                    validation_result: ValidationResult::Success,
                },
                SaveMetadata::rest_api(None).with_status(OperationStatus::Success),
            ),
        ];

        // Write entries to log
        for entry in &entries {
            logger.save_or_error(entry);
        }

        // Replay against empty store
        let result = logger
            .restore_to_store(&*data_store)
            .expect("Failed to replay log");

        assert_eq!(result.successful, 2);
        assert_eq!(result.failed, 0);
        assert_eq!(result.skipped, 0);

        // Verify data was replayed correctly
        assert!(data_store.get_entity(&entity).unwrap().is_some());

        assert!(
            data_store
                .get_component_definition(&definition.component)
                .unwrap()
                .is_some()
        );

        fs::remove_file(test_path).ok();
    }

    #[test]
    fn log_replay_skips_failed_operations() {
        use crate::InMemoryDataStore;
        use std::fs;
        use std::process;
        use std::sync::Arc;
        use std::time::{SystemTime, UNIX_EPOCH};

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let test_path = PathBuf::from(format!(
            "test_replay_failed_{}_{}.jsonl",
            process::id(),
            timestamp
        ));

        let logger = SavefileManager::new(test_path.clone());
        let data_store = Arc::new(InMemoryDataStore::new());

        let entity = test_entity();

        let entries = vec![
            // Successful operation
            SaveEntry::new(
                SaveOperation::EntityCreate {
                    entity,
                    was_random: false,
                },
                SaveMetadata::rest_api(None).with_status(OperationStatus::Success),
            ),
            // Failed operation - should be skipped
            SaveEntry::new(
                SaveOperation::EntityDelete {
                    entity_id: "invalid_entity".to_string(),
                    success: false,
                },
                SaveMetadata::rest_api(None).with_status(OperationStatus::Failed),
            ),
        ];

        for entry in &entries {
            logger.save_or_error(entry);
        }

        let result = logger
            .restore_to_store(&*data_store)
            .expect("Failed to replay log");

        assert_eq!(result.successful, 1);
        assert_eq!(result.failed, 0);
        assert_eq!(result.skipped, 1);

        fs::remove_file(test_path).ok();
    }

    #[test]
    fn log_replay_handles_patch_operations() {
        use crate::{Component, InMemoryDataStore};
        use serde_json::json;
        use std::fs;
        use std::process;
        use std::sync::Arc;
        use std::time::{SystemTime, UNIX_EPOCH};

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let test_path = PathBuf::from(format!(
            "test_replay_patch_{}_{}.jsonl",
            process::id(),
            timestamp
        ));

        let logger = SavefileManager::new(test_path.clone());
        let data_store = Arc::new(InMemoryDataStore::new());

        // First create a component definition to patch
        let component = Component::new("TestComponent").unwrap();
        let mut definition = ComponentDefinition::new(component.clone(), json!({"type": "string"}));
        let def_id = format!("{:?}", component);
        data_store
            .create_component_definition(&component, &definition)
            .unwrap();

        // Create patch operation
        definition.schema = json!({"type": "number"});
        let patch_entry = SaveEntry::new(
            SaveOperation::ComponentDefinitionPatch {
                definition_id: def_id.clone(),
                patch_data: json!({"schema": {"type": "number"}}),
                result_definition: definition.clone(),
            },
            SaveMetadata::rest_api(None).with_status(OperationStatus::Success),
        );

        logger.save_or_error(&patch_entry);

        let result = logger
            .restore_to_store(&*data_store)
            .expect("Failed to replay log");

        assert_eq!(result.successful, 1);
        assert_eq!(result.failed, 0);

        // Verify patch was applied
        let updated_def = data_store
            .get_component_definition(&component)
            .unwrap()
            .unwrap();
        assert_eq!(updated_def.schema, json!({"type": "number"}));

        fs::remove_file(test_path).ok();
    }

    #[test]
    fn replay_result_display_formatting() {
        let mut result = RestoreResult::new();
        result.successful = 5;
        result.failed = 2;
        result.skipped = 1;
        result.errors.push("Test error 1".to_string());
        result.errors.push("Test error 2".to_string());

        let display_str = format!("{}", result);
        assert!(display_str.contains("5 successful"));
        assert!(display_str.contains("2 failed"));
        assert!(display_str.contains("1 skipped"));
        assert!(display_str.contains("Test error 1"));
        assert!(display_str.contains("Test error 2"));
    }
}
