use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

use crate::{ComponentDefinition, DataStore, DataStoreError, Entity};

/// Comprehensive logging system for all state transitions in the stigmergy system.
///
/// Each variant represents a different type of operation that can occur, providing
/// structured logging with timestamps, operation details, and metadata for auditing,
/// debugging, and system monitoring purposes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    /// Unique identifier for this log entry
    pub id: String,

    /// Timestamp when the operation occurred
    pub timestamp: DateTime<Utc>,

    /// The specific operation that was performed
    pub operation: LogOperation,

    /// Additional metadata about the operation context
    pub metadata: LogMetadata,
}

/// All possible operations that can be logged in the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogOperation {
    // Entity operations
    EntityCreate {
        entity: Entity,
        was_random: bool,
    },
    EntityDelete {
        entity_id: String,
        success: bool,
    },

    // Component Definition operations
    ComponentDefinitionCreate {
        definition: ComponentDefinition,
        validation_result: ValidationResult,
    },
    ComponentDefinitionUpdate {
        definition_id: String,
        old_definition: Option<ComponentDefinition>,
        new_definition: ComponentDefinition,
        validation_result: ValidationResult,
    },
    ComponentDefinitionPatch {
        definition_id: String,
        patch_data: Value,
        result_definition: ComponentDefinition,
    },
    ComponentDefinitionDelete {
        definition_id: String,
        deleted_definition: Option<ComponentDefinition>,
    },
    ComponentDefinitionDeleteAll {
        count_deleted: u32,
    },
    ComponentDefinitionGet {
        definition_id: Option<String>,
        found: bool,
    },

    // Component Instance operations
    ComponentCreate {
        component_id: String,
        component_data: Value,
        validation_result: Option<ValidationResult>,
    },
    ComponentUpdate {
        component_id: String,
        old_data: Option<Value>,
        new_data: Value,
        validation_result: Option<ValidationResult>,
    },
    ComponentPatch {
        component_id: String,
        patch_data: Value,
        result_data: Value,
    },
    ComponentDelete {
        component_id: String,
        deleted_data: Option<Value>,
    },
    ComponentDeleteAll {
        count_deleted: u32,
    },
    ComponentGet {
        component_id: Option<String>,
        found: bool,
    },

    // Validation operations
    ValidationPerformed {
        target_type: ValidationType,
        target_id: String,
        result: ValidationResult,
    },

    // Schema operations
    SchemaGeneration {
        schema_type: String,
        result_schema: Value,
        success: bool,
    },
}

/// Result of a validation operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationResult {
    Success,
    Failed {
        error: String,
        details: Option<String>,
    },
}

/// Types of validation that can be performed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationType {
    ComponentDefinitionSchema,
    ComponentInstanceData,
    EnumSchema,
    GeneralSchema,
}

/// Metadata about the operation context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogMetadata {
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
    Success,
    Failed,
    Partial,
    Cancelled,
}

impl LogEntry {
    /// Creates a new log entry with a generated ID and current timestamp
    pub fn new(operation: LogOperation, metadata: LogMetadata) -> Self {
        Self {
            id: Self::generate_id(),
            timestamp: Utc::now(),
            operation,
            metadata,
        }
    }

    /// Creates a new log entry with a specific timestamp (useful for testing)
    pub fn with_timestamp(
        operation: LogOperation,
        metadata: LogMetadata,
        timestamp: DateTime<Utc>,
    ) -> Self {
        Self {
            id: Self::generate_id(),
            timestamp,
            operation,
            metadata,
        }
    }

    /// Generates a unique ID for the log entry
    fn generate_id() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        format!("log_{}", timestamp)
    }

    /// Returns the operation type as a string for filtering and indexing
    pub fn operation_type(&self) -> &'static str {
        match &self.operation {
            LogOperation::EntityCreate { .. } => "EntityCreate",
            LogOperation::EntityDelete { .. } => "EntityDelete",
            LogOperation::ComponentDefinitionCreate { .. } => "ComponentDefinitionCreate",
            LogOperation::ComponentDefinitionUpdate { .. } => "ComponentDefinitionUpdate",
            LogOperation::ComponentDefinitionPatch { .. } => "ComponentDefinitionPatch",
            LogOperation::ComponentDefinitionDelete { .. } => "ComponentDefinitionDelete",
            LogOperation::ComponentDefinitionDeleteAll { .. } => "ComponentDefinitionDeleteAll",
            LogOperation::ComponentDefinitionGet { .. } => "ComponentDefinitionGet",
            LogOperation::ComponentCreate { .. } => "ComponentCreate",
            LogOperation::ComponentUpdate { .. } => "ComponentUpdate",
            LogOperation::ComponentPatch { .. } => "ComponentPatch",
            LogOperation::ComponentDelete { .. } => "ComponentDelete",
            LogOperation::ComponentDeleteAll { .. } => "ComponentDeleteAll",
            LogOperation::ComponentGet { .. } => "ComponentGet",
            LogOperation::ValidationPerformed { .. } => "ValidationPerformed",
            LogOperation::SchemaGeneration { .. } => "SchemaGeneration",
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
            LogOperation::EntityCreate { entity, .. } => Some(entity.to_string()),
            LogOperation::EntityDelete { entity_id, .. } => Some(entity_id.clone()),
            _ => None,
        }
    }

    /// Returns the component definition ID if this operation involves a component definition
    pub fn component_definition_id(&self) -> Option<String> {
        match &self.operation {
            LogOperation::ComponentDefinitionCreate { definition, .. } => {
                Some(format!("{:?}", definition.component))
            }
            LogOperation::ComponentDefinitionUpdate { definition_id, .. }
            | LogOperation::ComponentDefinitionPatch { definition_id, .. }
            | LogOperation::ComponentDefinitionDelete { definition_id, .. }
            | LogOperation::ComponentDefinitionGet {
                definition_id: Some(definition_id),
                ..
            } => Some(definition_id.clone()),
            _ => None,
        }
    }

    /// Returns the component instance ID if this operation involves a component instance
    pub fn component_id(&self) -> Option<String> {
        match &self.operation {
            LogOperation::ComponentCreate { component_id, .. }
            | LogOperation::ComponentUpdate { component_id, .. }
            | LogOperation::ComponentPatch { component_id, .. }
            | LogOperation::ComponentDelete { component_id, .. }
            | LogOperation::ComponentGet {
                component_id: Some(component_id),
                ..
            } => Some(component_id.clone()),
            _ => None,
        }
    }
}

impl LogMetadata {
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

/// Convenience macros for creating log entries
#[macro_export]
macro_rules! log_entity_create {
    ($entity:expr, $was_random:expr) => {
        $crate::LogEntry::new(
            $crate::LogOperation::EntityCreate {
                entity: $entity,
                was_random: $was_random,
            },
            $crate::LogMetadata::rest_api(None),
        )
    };
}

#[macro_export]
macro_rules! log_entity_delete {
    ($entity_id:expr, $success:expr) => {
        $crate::LogEntry::new(
            $crate::LogOperation::EntityDelete {
                entity_id: $entity_id.to_string(),
                success: $success,
            },
            $crate::LogMetadata::rest_api(None),
        )
    };
}

#[macro_export]
macro_rules! log_component_create {
    ($component_id:expr, $data:expr, $validation:expr) => {
        $crate::LogEntry::new(
            $crate::LogOperation::ComponentCreate {
                component_id: $component_id.to_string(),
                component_data: $data,
                validation_result: $validation,
            },
            $crate::LogMetadata::rest_api(None),
        )
    };
}

/// Durable logger that writes log entries to JSONL files
pub struct DurableLogger {
    log_file_path: PathBuf,
}

impl DurableLogger {
    /// Creates a new durable logger with the specified file path
    pub fn new(log_file_path: PathBuf) -> Self {
        Self { log_file_path }
    }

    /// Creates a new durable logger using the default log file path
    pub fn with_default_path() -> Self {
        Self::new(PathBuf::from("stigmergy.jsonl"))
    }

    /// Writes a log entry to the JSONL file
    pub fn log(&self, entry: &LogEntry) -> Result<(), std::io::Error> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_file_path)?;

        let json_line = serde_json::to_string(entry)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        writeln!(file, "{}", json_line)?;
        file.flush()?;
        Ok(())
    }

    /// Logs an entry and prints an error message if logging fails
    pub fn log_or_error(&self, entry: &LogEntry) {
        if let Err(e) = self.log(entry) {
            eprintln!("Failed to write log entry: {}", e);
        }
    }

    /// Reads all log entries from the JSONL file
    pub fn read_log_entries(&self) -> Result<Vec<LogEntry>, std::io::Error> {
        if !self.log_file_path.exists() {
            return Ok(Vec::new());
        }

        let file = std::fs::File::open(&self.log_file_path)?;
        let reader = BufReader::new(file);
        let mut entries = Vec::new();

        for line in reader.lines() {
            let line = line?;
            if !line.trim().is_empty() {
                match serde_json::from_str::<LogEntry>(&line) {
                    Ok(entry) => entries.push(entry),
                    Err(e) => {
                        eprintln!("Failed to parse log entry: {} - Line: {}", e, line);
                    }
                }
            }
        }

        Ok(entries)
    }

    /// Replays successful operations from the log against a data store
    /// Only replays operations that were originally successful to avoid duplicating failures
    pub fn replay_against_store(
        &self,
        data_store: &dyn DataStore,
    ) -> Result<ReplayResult, std::io::Error> {
        let entries = self.read_log_entries()?;
        let mut result = ReplayResult::new();

        for entry in entries {
            // Only replay operations that were originally successful
            if !entry.is_success() {
                result.skipped += 1;
                continue;
            }

            match self.replay_single_operation(&entry, data_store) {
                Ok(_) => result.successful += 1,
                Err(e) => {
                    result.failed += 1;
                    result.errors.push(format!(
                        "Failed to replay {}: {}",
                        entry.operation_type(),
                        e
                    ));
                }
            }
        }

        Ok(result)
    }

    /// Replays a single log operation against a data store
    fn replay_single_operation(
        &self,
        entry: &LogEntry,
        data_store: &dyn DataStore,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match &entry.operation {
            LogOperation::EntityCreate { entity, .. } => {
                // Ignore AlreadyExists errors as the entity might already be in the store
                if let Err(DataStoreError::AlreadyExists) = data_store.create_entity(entity) {
                    // Entity already exists, that's fine for replay
                }
                Ok(())
            }

            LogOperation::EntityDelete { entity_id, success } => {
                if *success {
                    data_store.delete_entity(entity_id)?;
                }
                Ok(())
            }

            LogOperation::ComponentDefinitionCreate { definition, .. } => {
                let def_id = format!("{:?}", definition.component);
                // Ignore AlreadyExists errors
                if let Err(DataStoreError::AlreadyExists) =
                    data_store.create_component_definition(&def_id, definition)
                {
                    // Definition already exists, that's fine for replay
                }
                Ok(())
            }

            LogOperation::ComponentDefinitionUpdate {
                definition_id,
                new_definition,
                ..
            } => {
                data_store.update_component_definition(definition_id, new_definition)?;
                Ok(())
            }

            LogOperation::ComponentDefinitionDelete { definition_id, .. } => {
                data_store.delete_component_definition(definition_id)?;
                Ok(())
            }

            LogOperation::ComponentDefinitionDeleteAll { .. } => {
                data_store.delete_all_component_definitions()?;
                Ok(())
            }

            LogOperation::ComponentCreate {
                component_id,
                component_data,
                ..
            } => {
                // Ignore AlreadyExists errors
                if let Err(DataStoreError::AlreadyExists) =
                    data_store.create_component(component_id, component_data)
                {
                    // Component already exists, that's fine for replay
                }
                Ok(())
            }

            LogOperation::ComponentUpdate {
                component_id,
                new_data,
                ..
            } => {
                data_store.update_component(component_id, new_data)?;
                Ok(())
            }

            LogOperation::ComponentDelete { component_id, .. } => {
                data_store.delete_component(component_id)?;
                Ok(())
            }

            LogOperation::ComponentDeleteAll { .. } => {
                data_store.delete_all_components()?;
                Ok(())
            }

            LogOperation::ComponentDefinitionPatch {
                definition_id,
                result_definition,
                ..
            } => {
                data_store.update_component_definition(definition_id, result_definition)?;
                Ok(())
            }

            LogOperation::ComponentPatch {
                component_id,
                result_data,
                ..
            } => {
                data_store.update_component(component_id, result_data)?;
                Ok(())
            }

            // These operations are read-only or metadata, so we skip them in replay
            LogOperation::ComponentDefinitionGet { .. }
            | LogOperation::ComponentGet { .. }
            | LogOperation::ValidationPerformed { .. }
            | LogOperation::SchemaGeneration { .. } => {
                // Skip read-only operations and metadata
                Ok(())
            }
        }
    }
}

/// Result of replaying a log against a data store
#[derive(Debug, Clone)]
pub struct ReplayResult {
    pub successful: u32,
    pub failed: u32,
    pub skipped: u32,
    pub errors: Vec<String>,
}

impl ReplayResult {
    pub fn new() -> Self {
        Self {
            successful: 0,
            failed: 0,
            skipped: 0,
            errors: Vec::new(),
        }
    }

    pub fn total_processed(&self) -> u32 {
        self.successful + self.failed
    }
}

impl Default for ReplayResult {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ReplayResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Replay completed: {} successful, {} failed, {} skipped",
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
    fn log_entry_creation() {
        let operation = LogOperation::EntityCreate {
            entity: test_entity(),
            was_random: true,
        };
        let metadata = LogMetadata::rest_api(Some("req_123".to_string()));
        let entry = LogEntry::new(operation, metadata);

        assert!(!entry.id.is_empty());
        assert!(entry.id.starts_with("log_"));
        assert_eq!(entry.operation_type(), "EntityCreate");
        assert!(entry.is_success());
        assert!(!entry.is_failure());
    }

    #[test]
    fn log_entry_with_timestamp() {
        let timestamp = Utc.with_ymd_and_hms(2023, 1, 1, 12, 0, 0).unwrap();
        let operation = LogOperation::EntityDelete {
            entity_id: "entity_123".to_string(),
            success: true,
        };
        let metadata = LogMetadata::internal(Some("system".to_string()));
        let entry = LogEntry::with_timestamp(operation, metadata, timestamp);

        assert_eq!(entry.timestamp, timestamp);
        assert_eq!(entry.operation_type(), "EntityDelete");
    }

    #[test]
    fn entity_operations() {
        let entity = test_entity();
        let entity_id = entity.to_string();

        let create_op = LogOperation::EntityCreate {
            entity,
            was_random: false,
        };
        let create_entry = LogEntry::new(create_op, LogMetadata::rest_api(None));
        assert_eq!(create_entry.operation_type(), "EntityCreate");
        assert_eq!(create_entry.entity_id(), Some(entity_id.clone()));

        let delete_op = LogOperation::EntityDelete {
            entity_id: entity_id.clone(),
            success: true,
        };
        let delete_entry = LogEntry::new(delete_op, LogMetadata::rest_api(None));
        assert_eq!(delete_entry.operation_type(), "EntityDelete");
        assert_eq!(delete_entry.entity_id(), Some(entity_id));
    }

    #[test]
    fn component_definition_operations() {
        let definition = test_component_definition();

        let create_op = LogOperation::ComponentDefinitionCreate {
            definition: definition.clone(),
            validation_result: ValidationResult::success(),
        };
        let create_entry = LogEntry::new(create_op, LogMetadata::rest_api(None));
        assert_eq!(create_entry.operation_type(), "ComponentDefinitionCreate");
        assert!(create_entry.component_definition_id().is_some());

        let update_op = LogOperation::ComponentDefinitionUpdate {
            definition_id: "def_123".to_string(),
            old_definition: Some(definition.clone()),
            new_definition: definition,
            validation_result: ValidationResult::failed("Invalid schema"),
        };
        let update_entry = LogEntry::new(update_op, LogMetadata::rest_api(None));
        assert_eq!(update_entry.operation_type(), "ComponentDefinitionUpdate");
        assert_eq!(
            update_entry.component_definition_id(),
            Some("def_123".to_string())
        );

        let patch_op = LogOperation::ComponentDefinitionPatch {
            definition_id: "def_456".to_string(),
            patch_data: json!({"schema": {"type": "number"}}),
            result_definition: test_component_definition(),
        };
        let patch_entry = LogEntry::new(patch_op, LogMetadata::rest_api(None));
        assert_eq!(patch_entry.operation_type(), "ComponentDefinitionPatch");

        let delete_op = LogOperation::ComponentDefinitionDelete {
            definition_id: "def_789".to_string(),
            deleted_definition: None,
        };
        let delete_entry = LogEntry::new(delete_op, LogMetadata::rest_api(None));
        assert_eq!(delete_entry.operation_type(), "ComponentDefinitionDelete");

        let delete_all_op = LogOperation::ComponentDefinitionDeleteAll { count_deleted: 5 };
        let delete_all_entry = LogEntry::new(delete_all_op, LogMetadata::rest_api(None));
        assert_eq!(
            delete_all_entry.operation_type(),
            "ComponentDefinitionDeleteAll"
        );

        let get_op = LogOperation::ComponentDefinitionGet {
            definition_id: Some("def_get".to_string()),
            found: true,
        };
        let get_entry = LogEntry::new(get_op, LogMetadata::rest_api(None));
        assert_eq!(get_entry.operation_type(), "ComponentDefinitionGet");
    }

    #[test]
    fn component_instance_operations() {
        let component_data = json!({"color": "red"});

        let create_op = LogOperation::ComponentCreate {
            component_id: "comp_123".to_string(),
            component_data: component_data.clone(),
            validation_result: Some(ValidationResult::success()),
        };
        let create_entry = LogEntry::new(create_op, LogMetadata::rest_api(None));
        assert_eq!(create_entry.operation_type(), "ComponentCreate");
        assert_eq!(create_entry.component_id(), Some("comp_123".to_string()));

        let update_op = LogOperation::ComponentUpdate {
            component_id: "comp_456".to_string(),
            old_data: Some(json!({"color": "blue"})),
            new_data: component_data.clone(),
            validation_result: None,
        };
        let update_entry = LogEntry::new(update_op, LogMetadata::rest_api(None));
        assert_eq!(update_entry.operation_type(), "ComponentUpdate");

        let patch_op = LogOperation::ComponentPatch {
            component_id: "comp_789".to_string(),
            patch_data: json!({"color": "green"}),
            result_data: component_data,
        };
        let patch_entry = LogEntry::new(patch_op, LogMetadata::rest_api(None));
        assert_eq!(patch_entry.operation_type(), "ComponentPatch");

        let delete_op = LogOperation::ComponentDelete {
            component_id: "comp_delete".to_string(),
            deleted_data: Some(json!({"color": "yellow"})),
        };
        let delete_entry = LogEntry::new(delete_op, LogMetadata::rest_api(None));
        assert_eq!(delete_entry.operation_type(), "ComponentDelete");

        let delete_all_op = LogOperation::ComponentDeleteAll { count_deleted: 3 };
        let delete_all_entry = LogEntry::new(delete_all_op, LogMetadata::rest_api(None));
        assert_eq!(delete_all_entry.operation_type(), "ComponentDeleteAll");

        let get_op = LogOperation::ComponentGet {
            component_id: None,
            found: false,
        };
        let get_entry = LogEntry::new(get_op, LogMetadata::rest_api(None));
        assert_eq!(get_entry.operation_type(), "ComponentGet");
        assert_eq!(get_entry.component_id(), None);
    }

    #[test]
    fn validation_and_schema_operations() {
        let validation_op = LogOperation::ValidationPerformed {
            target_type: ValidationType::ComponentDefinitionSchema,
            target_id: "target_123".to_string(),
            result: ValidationResult::failed_with_details(
                "Schema validation failed",
                "Missing required field 'type'",
            ),
        };
        let validation_entry = LogEntry::new(validation_op, LogMetadata::internal(None));
        assert_eq!(validation_entry.operation_type(), "ValidationPerformed");

        let schema_op = LogOperation::SchemaGeneration {
            schema_type: "enum".to_string(),
            result_schema: json!({"oneOf": [{"type": "string"}]}),
            success: true,
        };
        let schema_entry = LogEntry::new(schema_op, LogMetadata::internal(None));
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
        let rest_metadata = LogMetadata::rest_api(Some("req_456".to_string()));
        assert_eq!(rest_metadata.source, "REST API");
        assert_eq!(rest_metadata.request_id, Some("req_456".to_string()));
        assert!(matches!(rest_metadata.status, OperationStatus::Success));

        let internal_metadata = LogMetadata::internal(Some("scheduler".to_string()));
        assert_eq!(internal_metadata.source, "Internal");
        assert_eq!(internal_metadata.initiator, Some("scheduler".to_string()));

        let enhanced_metadata = LogMetadata::rest_api(None)
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
        let operation = LogOperation::ComponentCreate {
            component_id: "test_comp".to_string(),
            component_data: json!({"value": 42}),
            validation_result: Some(ValidationResult::success()),
        };
        let metadata = LogMetadata::rest_api(Some("req_789".to_string()))
            .with_duration(100)
            .with_status(OperationStatus::Success);
        let entry = LogEntry::new(operation, metadata);

        let serialized = serde_json::to_string(&entry).unwrap();
        assert!(!serialized.is_empty());

        let deserialized: LogEntry = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.id, entry.id);
        assert_eq!(deserialized.operation_type(), entry.operation_type());
    }

    #[test]
    fn macro_usage() {
        let entity = test_entity();
        let create_log = log_entity_create!(entity, true);
        assert_eq!(create_log.operation_type(), "EntityCreate");

        let delete_log = log_entity_delete!("entity_456", false);
        assert_eq!(delete_log.operation_type(), "EntityDelete");

        let component_log = log_component_create!(
            "comp_789",
            json!({"test": "data"}),
            Some(ValidationResult::success())
        );
        assert_eq!(component_log.operation_type(), "ComponentCreate");
    }

    #[test]
    fn operation_status_checks() {
        let success_entry = LogEntry::new(
            LogOperation::EntityCreate {
                entity: test_entity(),
                was_random: false,
            },
            LogMetadata::rest_api(None),
        );
        assert!(success_entry.is_success());
        assert!(!success_entry.is_failure());

        let failed_entry = LogEntry::new(
            LogOperation::EntityDelete {
                entity_id: "bad_id".to_string(),
                success: false,
            },
            LogMetadata::rest_api(None).with_status(OperationStatus::Failed),
        );
        assert!(!failed_entry.is_success());
        assert!(failed_entry.is_failure());
    }

    #[test]
    fn id_extraction_methods() {
        let entity = test_entity();
        let entity_id = entity.to_string();

        let entity_entry = LogEntry::new(
            LogOperation::EntityCreate {
                entity,
                was_random: false,
            },
            LogMetadata::rest_api(None),
        );
        assert_eq!(entity_entry.entity_id(), Some(entity_id));
        assert_eq!(entity_entry.component_definition_id(), None);
        assert_eq!(entity_entry.component_id(), None);

        let comp_def_entry = LogEntry::new(
            LogOperation::ComponentDefinitionUpdate {
                definition_id: "def_123".to_string(),
                old_definition: None,
                new_definition: test_component_definition(),
                validation_result: ValidationResult::success(),
            },
            LogMetadata::rest_api(None),
        );
        assert_eq!(comp_def_entry.entity_id(), None);
        assert_eq!(
            comp_def_entry.component_definition_id(),
            Some("def_123".to_string())
        );
        assert_eq!(comp_def_entry.component_id(), None);

        let comp_entry = LogEntry::new(
            LogOperation::ComponentPatch {
                component_id: "comp_456".to_string(),
                patch_data: json!({}),
                result_data: json!({}),
            },
            LogMetadata::rest_api(None),
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
            let operation = LogOperation::ValidationPerformed {
                target_type: validation_type,
                target_id: "test_target".to_string(),
                result: ValidationResult::success(),
            };
            let entry = LogEntry::new(operation, LogMetadata::internal(None));
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
            let metadata = LogMetadata::rest_api(None).with_status(status);
            let entry = LogEntry::new(
                LogOperation::EntityCreate {
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
        let test_path = PathBuf::from(format!("test_logger_{}_{}.jsonl", process::id(), timestamp));

        let logger = DurableLogger::new(test_path.clone());
        let operation = LogOperation::EntityCreate {
            entity: test_entity(),
            was_random: true,
        };
        let metadata = LogMetadata::rest_api(Some("test_request".to_string()));
        let entry = LogEntry::new(operation, metadata);

        logger.log_or_error(&entry);

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

        let logger = DurableLogger::new(test_path.clone());

        // Create multiple log entries
        let entity1 = test_entity();
        let entity2 = Entity::new([2u8; 32]);

        let entry1 = LogEntry::new(
            LogOperation::EntityCreate {
                entity: entity1,
                was_random: false,
            },
            LogMetadata::rest_api(None).with_status(OperationStatus::Success),
        );

        let entry2 = LogEntry::new(
            LogOperation::EntityCreate {
                entity: entity2,
                was_random: true,
            },
            LogMetadata::rest_api(None).with_status(OperationStatus::Success),
        );

        logger.log_or_error(&entry1);
        logger.log_or_error(&entry2);

        // Read back the entries
        let read_entries = logger
            .read_log_entries()
            .expect("Failed to read log entries");
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

        let logger = DurableLogger::new(test_path.clone());
        let data_store = Arc::new(InMemoryDataStore::new());

        // Create log entries for various operations
        let entity = test_entity();
        let definition = test_component_definition();

        let entries = vec![
            LogEntry::new(
                LogOperation::EntityCreate {
                    entity,
                    was_random: false,
                },
                LogMetadata::rest_api(None).with_status(OperationStatus::Success),
            ),
            LogEntry::new(
                LogOperation::ComponentDefinitionCreate {
                    definition: definition.clone(),
                    validation_result: ValidationResult::Success,
                },
                LogMetadata::rest_api(None).with_status(OperationStatus::Success),
            ),
        ];

        // Write entries to log
        for entry in &entries {
            logger.log_or_error(entry);
        }

        // Replay against empty store
        let result = logger
            .replay_against_store(&*data_store)
            .expect("Failed to replay log");

        assert_eq!(result.successful, 2);
        assert_eq!(result.failed, 0);
        assert_eq!(result.skipped, 0);

        // Verify data was replayed correctly
        let entity_id = entity.to_string();
        assert!(data_store.get_entity(&entity_id).unwrap().is_some());

        let def_id = format!("{:?}", definition.component);
        assert!(
            data_store
                .get_component_definition(&def_id)
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

        let logger = DurableLogger::new(test_path.clone());
        let data_store = Arc::new(InMemoryDataStore::new());

        let entity = test_entity();

        let entries = vec![
            // Successful operation
            LogEntry::new(
                LogOperation::EntityCreate {
                    entity,
                    was_random: false,
                },
                LogMetadata::rest_api(None).with_status(OperationStatus::Success),
            ),
            // Failed operation - should be skipped
            LogEntry::new(
                LogOperation::EntityDelete {
                    entity_id: "invalid_entity".to_string(),
                    success: false,
                },
                LogMetadata::rest_api(None).with_status(OperationStatus::Failed),
            ),
        ];

        for entry in &entries {
            logger.log_or_error(entry);
        }

        let result = logger
            .replay_against_store(&*data_store)
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

        let logger = DurableLogger::new(test_path.clone());
        let data_store = Arc::new(InMemoryDataStore::new());

        // First create a component definition to patch
        let component = Component::new("TestComponent").unwrap();
        let mut definition = ComponentDefinition::new(component.clone(), json!({"type": "string"}));
        let def_id = format!("{:?}", component);
        data_store
            .create_component_definition(&def_id, &definition)
            .unwrap();

        // Create patch operation
        definition.schema = json!({"type": "number"});
        let patch_entry = LogEntry::new(
            LogOperation::ComponentDefinitionPatch {
                definition_id: def_id.clone(),
                patch_data: json!({"schema": {"type": "number"}}),
                result_definition: definition.clone(),
            },
            LogMetadata::rest_api(None).with_status(OperationStatus::Success),
        );

        logger.log_or_error(&patch_entry);

        let result = logger
            .replay_against_store(&*data_store)
            .expect("Failed to replay log");

        assert_eq!(result.successful, 1);
        assert_eq!(result.failed, 0);

        // Verify patch was applied
        let updated_def = data_store
            .get_component_definition(&def_id)
            .unwrap()
            .unwrap();
        assert_eq!(updated_def.schema, json!({"type": "number"}));

        fs::remove_file(test_path).ok();
    }

    #[test]
    fn replay_result_display_formatting() {
        let mut result = ReplayResult::new();
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
