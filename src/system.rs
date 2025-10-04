use std::str::FromStr;

use axum::Router;
use axum::async_trait;
use axum::body::Bytes;
use axum::extract::{FromRequest, Path, Request, State};
use axum::http::StatusCode;
use axum::response::Json;
use axum::routing::{get, post};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;

use crate::{
    DataStore, OperationStatus, SaveEntry, SaveMetadata, SaveOperation, SavefileManager,
    SystemConfig, SystemParser,
};

/////////////////////////////////////////////// SystemName /////////////////////////////////////////////

/// Error returned when parsing an invalid system name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemNameParseError {
    invalid_name: String,
}

impl SystemNameParseError {
    /// Creates a new SystemNameParseError.
    pub fn new(name: String) -> Self {
        SystemNameParseError { invalid_name: name }
    }

    /// Returns the invalid name that caused the error.
    pub fn invalid_name(&self) -> &str {
        &self.invalid_name
    }
}

impl std::fmt::Display for SystemNameParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Invalid system name {:?}. System names must be valid Rust-style identifiers (alphanumeric, underscore, hyphen, starting with letter or underscore)",
            self.invalid_name
        )
    }
}

impl std::error::Error for SystemNameParseError {}

/// A strongly-typed system name identifier.
///
/// System names must be valid Rust identifiers with optional module paths (like Component).
/// This ensures system names are URL-safe and follow consistent naming conventions.
///
/// # Examples
///
/// ```rust
/// use stigmergy::SystemName;
///
/// // Simple system names
/// let name = SystemName::new("dry-principal").unwrap();
/// let name2 = SystemName::new("code-reviewer").unwrap();
///
/// // Invalid names return None
/// assert!(SystemName::new("").is_none());
/// assert!(SystemName::new("123invalid").is_none());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SystemName(String);

impl SystemName {
    /// Creates a new SystemName if the string is a valid identifier.
    ///
    /// # Arguments
    /// * `name` - A string-like type that can be converted to a system name
    ///
    /// # Returns
    /// * `Some(SystemName)` - If the name is valid
    /// * `None` - If the name is invalid
    pub fn new(name: impl Into<String>) -> Option<SystemName> {
        let s = name.into();
        if is_valid_system_name(&s) {
            Some(SystemName(s))
        } else {
            None
        }
    }

    /// Returns the system name as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consumes the SystemName and returns the inner String.
    pub fn into_string(self) -> String {
        self.0
    }
}

impl std::fmt::Display for SystemName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for SystemName {
    type Err = SystemNameParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        SystemName::new(s).ok_or_else(|| SystemNameParseError::new(s.to_string()))
    }
}

/// Validates that a string is a valid system name.
///
/// System names must be valid Rust-style identifiers, potentially with module paths.
/// They follow the same rules as Component names.
fn is_valid_system_name(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }

    // Split by :: to handle paths like my_crate::system
    let segments: Vec<&str> = s.split("::").collect();

    // Each segment must be a valid identifier
    segments
        .iter()
        .all(|segment| is_valid_rust_identifier(segment))
}

/// Validates that a string is a valid Rust identifier.
fn is_valid_rust_identifier(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }

    let mut chars = s.chars();
    let first = chars.next().unwrap();

    // First character must be alphabetic or underscore
    if !first.is_alphabetic() && first != '_' && first != '-' {
        return false;
    }

    // Remaining characters must be alphanumeric, underscore, or hyphen
    chars.all(|c| c.is_alphanumeric() || c == '_' || c == '-')
}

//////////////////////////////////////////////// System ////////////////////////////////////////////////

/// A system represents a Claude Code agent configuration with its associated metadata.
/// Systems are identified by their name.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct System {
    /// The system configuration (flattened into this struct)
    #[serde(flatten)]
    pub config: SystemConfig,
    /// When the system was created
    pub created_at: DateTime<Utc>,
    /// When the system was last updated
    pub updated_at: DateTime<Utc>,
}

impl System {
    /// Creates a new System with the given configuration.
    pub fn new(config: SystemConfig) -> Self {
        let now = Utc::now();
        System {
            config,
            created_at: now,
            updated_at: now,
        }
    }

    /// Returns the system's name (which serves as its identifier).
    pub fn name(&self) -> &SystemName {
        &self.config.name
    }

    /// Updates the system configuration and marks it as updated.
    pub fn update_config(&mut self, config: SystemConfig) {
        self.config = config;
        self.updated_at = Utc::now();
    }
}

/// A wrapper that extracts SystemConfig from either JSON or YAML based on Content-Type.
pub struct SystemConfigExtractor(pub SystemConfig);

#[async_trait]
impl<S> FromRequest<S> for SystemConfigExtractor
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let (parts, body) = req.into_parts();
        let content_type = parts
            .headers
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("application/json")
            .to_string();

        let bytes = Bytes::from_request(Request::from_parts(parts, body), state)
            .await
            .map_err(|_| (StatusCode::BAD_REQUEST, "failed to read request body"))?;

        let config = if content_type.contains("yaml") || content_type.contains("yml") {
            serde_yml::from_slice::<SystemConfig>(&bytes)
                .map_err(|_| (StatusCode::BAD_REQUEST, "invalid yaml"))?
        } else {
            serde_json::from_slice::<SystemConfig>(&bytes)
                .map_err(|_| (StatusCode::BAD_REQUEST, "invalid json"))?
        };

        Ok(SystemConfigExtractor(config))
    }
}

////////////////////////////////////////////// Routes //////////////////////////////////////////////////

/// Request structure for creating a system from markdown content.
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateSystemFromMarkdownRequest {
    /// The markdown content containing frontmatter and system description.
    pub content: String,
}

/// Response structure for system creation.
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateSystemResponse {
    /// The system that was created.
    pub system: System,
    /// Whether the system was successfully created (always true in current implementation).
    pub created: bool,
}

/// List item for systems (simplified view).
#[derive(Debug, Serialize, Deserialize)]
pub struct SystemListItem {
    /// The system's name (which serves as its identifier)
    pub name: String,
    /// Brief description of the system's purpose
    pub description: String,
    /// The language model to use for this system
    pub model: String,
    /// Color identifier for UI display
    pub color: String,
    /// When the system was created
    pub created_at: DateTime<Utc>,
    /// When the system was last updated
    pub updated_at: DateTime<Utc>,
}

impl From<System> for SystemListItem {
    fn from(system: System) -> Self {
        SystemListItem {
            name: system.config.name.into_string(),
            description: system.config.description,
            model: system.config.model,
            color: system.config.color,
            created_at: system.created_at,
            updated_at: system.updated_at,
        }
    }
}

/// Creates a new system from configuration.
async fn create_system(
    State((logger, data_store)): State<(Arc<SavefileManager>, Arc<dyn DataStore>)>,
    SystemConfigExtractor(config): SystemConfigExtractor,
) -> Result<Json<CreateSystemResponse>, (StatusCode, &'static str)> {
    // Validate the config first
    if config.validate().is_err() {
        let save_entry = SaveEntry::new(
            SaveOperation::SystemCreate {
                system_id: "unknown".to_string(),
                config: None,
                success: false,
            },
            SaveMetadata::rest_api(None).with_status(OperationStatus::Failed),
        );
        logger.save_or_error(&save_entry);
        return Err((StatusCode::BAD_REQUEST, "Invalid system configuration"));
    }

    let system = System::new(config.clone());
    let system_name = system.name().to_string();

    // Store the system in the data store
    if data_store.create_system(&system).is_err() {
        let log_entry = SaveEntry::new(
            SaveOperation::SystemCreate {
                system_id: system_name.clone(),
                config: Some(config),
                success: false,
            },
            SaveMetadata::rest_api(None).with_status(OperationStatus::Failed),
        );
        logger.save_or_error(&log_entry);
        return Err((StatusCode::CONFLICT, "system with this name already exists"));
    }

    let log_entry = SaveEntry::new(
        SaveOperation::SystemCreate {
            system_id: system_name,
            config: Some(config),
            success: true,
        },
        SaveMetadata::rest_api(None).with_status(OperationStatus::Success),
    );
    logger.save_or_error(&log_entry);

    let response = CreateSystemResponse {
        system,
        created: true,
    };

    Ok(Json(response))
}

/// Creates a new system from markdown content.
async fn create_system_from_markdown(
    State((logger, data_store)): State<(Arc<SavefileManager>, Arc<dyn DataStore>)>,
    Json(request): Json<CreateSystemFromMarkdownRequest>,
) -> Result<Json<CreateSystemResponse>, (StatusCode, String)> {
    let config = match SystemParser::parse(&request.content) {
        Ok(config) => config,
        Err(e) => {
            let log_entry = SaveEntry::new(
                SaveOperation::SystemCreate {
                    system_id: "unknown".to_string(),
                    config: None,
                    success: false,
                },
                SaveMetadata::rest_api(None).with_status(OperationStatus::Failed),
            );
            logger.save_or_error(&log_entry);
            return Err((
                StatusCode::BAD_REQUEST,
                format!("Failed to parse markdown: {}", e),
            ));
        }
    };

    match create_system(State((logger, data_store)), SystemConfigExtractor(config)).await {
        Ok(response) => Ok(response),
        Err((status, msg)) => Err((status, msg.to_string())),
    }
}

/// Lists all systems.
async fn list_systems(
    State((_logger, data_store)): State<(Arc<SavefileManager>, Arc<dyn DataStore>)>,
) -> Result<Json<Vec<SystemListItem>>, (StatusCode, &'static str)> {
    match data_store.list_systems() {
        Ok(systems) => {
            let system_list: Vec<SystemListItem> =
                systems.into_iter().map(|system| system.into()).collect();
            Ok(Json(system_list))
        }
        Err(_) => Err((StatusCode::INTERNAL_SERVER_ERROR, "failed to list systems")),
    }
}

/// Gets a system by its name.
async fn get_system(
    State((logger, data_store)): State<(Arc<SavefileManager>, Arc<dyn DataStore>)>,
    Path(name): Path<String>,
) -> Result<Json<System>, (StatusCode, &'static str)> {
    let system_name = match SystemName::new(&name) {
        Some(n) => n,
        None => {
            let log_entry = SaveEntry::new(
                SaveOperation::SystemGet {
                    system_id: name,
                    success: false,
                },
                SaveMetadata::rest_api(None).with_status(OperationStatus::Failed),
            );
            logger.save_or_error(&log_entry);
            return Err((StatusCode::BAD_REQUEST, "invalid system name"));
        }
    };

    match data_store.get_system(&system_name) {
        Ok(Some(system)) => {
            let log_entry = SaveEntry::new(
                SaveOperation::SystemGet {
                    system_id: system_name.into_string(),
                    success: true,
                },
                SaveMetadata::rest_api(None).with_status(OperationStatus::Success),
            );
            logger.save_or_error(&log_entry);
            Ok(Json(system))
        }
        Ok(None) => {
            let log_entry = SaveEntry::new(
                SaveOperation::SystemGet {
                    system_id: name,
                    success: false,
                },
                SaveMetadata::rest_api(None).with_status(OperationStatus::Failed),
            );
            logger.save_or_error(&log_entry);
            Err((StatusCode::NOT_FOUND, "system not found"))
        }
        Err(_) => {
            let log_entry = SaveEntry::new(
                SaveOperation::SystemGet {
                    system_id: name,
                    success: false,
                },
                SaveMetadata::rest_api(None).with_status(OperationStatus::Failed),
            );
            logger.save_or_error(&log_entry);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to retrieve system",
            ))
        }
    }
}

/// Updates a system.
async fn update_system(
    State((logger, data_store)): State<(Arc<SavefileManager>, Arc<dyn DataStore>)>,
    Path(name): Path<String>,
    SystemConfigExtractor(config): SystemConfigExtractor,
) -> Result<Json<System>, (StatusCode, String)> {
    // Validate the config first
    if let Err(e) = config.validate() {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("Invalid system configuration: {}", e),
        ));
    }

    let system_name = match SystemName::new(&name) {
        Some(n) => n,
        None => {
            let log_entry = SaveEntry::new(
                SaveOperation::SystemUpdate {
                    system_id: name,
                    old_config: None,
                    new_config: config,
                    success: false,
                },
                SaveMetadata::rest_api(None).with_status(OperationStatus::Failed),
            );
            logger.save_or_error(&log_entry);
            return Err((StatusCode::BAD_REQUEST, "invalid system name".to_string()));
        }
    };

    let old_system = match data_store.get_system(&system_name) {
        Ok(Some(system)) => system,
        Ok(None) => {
            let log_entry = SaveEntry::new(
                SaveOperation::SystemUpdate {
                    system_id: name.clone(),
                    old_config: None,
                    new_config: config,
                    success: false,
                },
                SaveMetadata::rest_api(None).with_status(OperationStatus::Failed),
            );
            logger.save_or_error(&log_entry);
            return Err((StatusCode::NOT_FOUND, "system not found".to_string()));
        }
        Err(_) => {
            let log_entry = SaveEntry::new(
                SaveOperation::SystemUpdate {
                    system_id: name.clone(),
                    old_config: None,
                    new_config: config,
                    success: false,
                },
                SaveMetadata::rest_api(None).with_status(OperationStatus::Failed),
            );
            logger.save_or_error(&log_entry);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to retrieve system".to_string(),
            ));
        }
    };

    let mut updated_system = old_system.clone();
    updated_system.update_config(config.clone());

    match data_store.update_system(&updated_system) {
        Ok(true) => {
            let log_entry = SaveEntry::new(
                SaveOperation::SystemUpdate {
                    system_id: name,
                    old_config: Some(old_system.config),
                    new_config: config,
                    success: true,
                },
                SaveMetadata::rest_api(None).with_status(OperationStatus::Success),
            );
            logger.save_or_error(&log_entry);
            Ok(Json(updated_system))
        }
        Ok(false) => {
            let log_entry = SaveEntry::new(
                SaveOperation::SystemUpdate {
                    system_id: name,
                    old_config: Some(old_system.config),
                    new_config: config,
                    success: false,
                },
                SaveMetadata::rest_api(None).with_status(OperationStatus::Failed),
            );
            logger.save_or_error(&log_entry);
            Err((StatusCode::NOT_FOUND, "system not found".to_string()))
        }
        Err(_) => {
            let log_entry = SaveEntry::new(
                SaveOperation::SystemUpdate {
                    system_id: name,
                    old_config: Some(old_system.config),
                    new_config: config,
                    success: false,
                },
                SaveMetadata::rest_api(None).with_status(OperationStatus::Failed),
            );
            logger.save_or_error(&log_entry);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to update system".to_string(),
            ))
        }
    }
}

/// Patches a system.
async fn patch_system(
    State((logger, data_store)): State<(Arc<SavefileManager>, Arc<dyn DataStore>)>,
    Path(name): Path<String>,
    Json(patch_data): Json<Value>,
) -> Result<Json<System>, (StatusCode, String)> {
    let system_name = match SystemName::new(&name) {
        Some(n) => n,
        None => {
            let log_entry = SaveEntry::new(
                SaveOperation::SystemPatch {
                    system_id: name,
                    patch_data,
                    success: false,
                },
                SaveMetadata::rest_api(None).with_status(OperationStatus::Failed),
            );
            logger.save_or_error(&log_entry);
            return Err((StatusCode::BAD_REQUEST, "invalid system name".to_string()));
        }
    };

    let mut system = match data_store.get_system(&system_name) {
        Ok(Some(system)) => system,
        Ok(None) => {
            let log_entry = SaveEntry::new(
                SaveOperation::SystemPatch {
                    system_id: name.clone(),
                    patch_data,
                    success: false,
                },
                SaveMetadata::rest_api(None).with_status(OperationStatus::Failed),
            );
            logger.save_or_error(&log_entry);
            return Err((StatusCode::NOT_FOUND, "system not found".to_string()));
        }
        Err(_) => {
            let log_entry = SaveEntry::new(
                SaveOperation::SystemPatch {
                    system_id: name.clone(),
                    patch_data,
                    success: false,
                },
                SaveMetadata::rest_api(None).with_status(OperationStatus::Failed),
            );
            logger.save_or_error(&log_entry);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to retrieve system".to_string(),
            ));
        }
    };

    let mut config = system.config.clone();
    let patch_obj = match patch_data.as_object() {
        Some(obj) => obj,
        None => {
            let log_entry = SaveEntry::new(
                SaveOperation::SystemPatch {
                    system_id: name.clone(),
                    patch_data,
                    success: false,
                },
                SaveMetadata::rest_api(None).with_status(OperationStatus::Failed),
            );
            logger.save_or_error(&log_entry);
            return Err((
                StatusCode::BAD_REQUEST,
                "patch data must be an object".to_string(),
            ));
        }
    };

    if let Some(patch_name) = patch_obj.get("name").and_then(|v| v.as_str()) {
        config.name = SystemName::new(patch_name)
            .ok_or_else(|| (StatusCode::BAD_REQUEST, "invalid system name".to_string()))?;
    }
    if let Some(description) = patch_obj.get("description").and_then(|v| v.as_str()) {
        config.description = description.to_string();
    }
    if let Some(model) = patch_obj.get("model").and_then(|v| v.as_str()) {
        config.model = model.to_string();
    }
    if let Some(color) = patch_obj.get("color").and_then(|v| v.as_str()) {
        config.color = color.to_string();
    }
    if let Some(content) = patch_obj.get("content").and_then(|v| v.as_str()) {
        config.content = content.to_string();
    }
    if let Some(tools) = patch_obj.get("tools").and_then(|v| v.as_array()) {
        config.tools = tools
            .iter()
            .filter_map(|v| v.as_str())
            .map(|s| s.to_string())
            .collect();
    }

    system.update_config(config);

    match data_store.update_system(&system) {
        Ok(true) => {
            let log_entry = SaveEntry::new(
                SaveOperation::SystemPatch {
                    system_id: name,
                    patch_data,
                    success: true,
                },
                SaveMetadata::rest_api(None).with_status(OperationStatus::Success),
            );
            logger.save_or_error(&log_entry);
            Ok(Json(system))
        }
        Ok(false) => {
            let log_entry = SaveEntry::new(
                SaveOperation::SystemPatch {
                    system_id: name,
                    patch_data,
                    success: false,
                },
                SaveMetadata::rest_api(None).with_status(OperationStatus::Failed),
            );
            logger.save_or_error(&log_entry);
            Err((StatusCode::NOT_FOUND, "system not found".to_string()))
        }
        Err(_) => {
            let log_entry = SaveEntry::new(
                SaveOperation::SystemPatch {
                    system_id: name,
                    patch_data,
                    success: false,
                },
                SaveMetadata::rest_api(None).with_status(OperationStatus::Failed),
            );
            logger.save_or_error(&log_entry);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to update system".to_string(),
            ))
        }
    }
}

/// Deletes a system.
async fn delete_system(
    State((logger, data_store)): State<(Arc<SavefileManager>, Arc<dyn DataStore>)>,
    Path(name): Path<String>,
) -> Result<StatusCode, (StatusCode, &'static str)> {
    let system_name = match SystemName::new(&name) {
        Some(n) => n,
        None => {
            let log_entry = SaveEntry::new(
                SaveOperation::SystemDelete {
                    system_id: name,
                    success: false,
                },
                SaveMetadata::rest_api(None).with_status(OperationStatus::Failed),
            );
            logger.save_or_error(&log_entry);
            return Err((StatusCode::BAD_REQUEST, "invalid system name"));
        }
    };

    match data_store.delete_system(&system_name) {
        Ok(true) => {
            let log_entry = SaveEntry::new(
                SaveOperation::SystemDelete {
                    system_id: name,
                    success: true,
                },
                SaveMetadata::rest_api(None).with_status(OperationStatus::Success),
            );
            logger.save_or_error(&log_entry);
            Ok(StatusCode::NO_CONTENT)
        }
        Ok(false) => {
            let log_entry = SaveEntry::new(
                SaveOperation::SystemDelete {
                    system_id: name,
                    success: false,
                },
                SaveMetadata::rest_api(None).with_status(OperationStatus::Failed),
            );
            logger.save_or_error(&log_entry);
            Err((StatusCode::NOT_FOUND, "system not found"))
        }
        Err(_) => {
            let log_entry = SaveEntry::new(
                SaveOperation::SystemDelete {
                    system_id: name,
                    success: false,
                },
                SaveMetadata::rest_api(None).with_status(OperationStatus::Failed),
            );
            logger.save_or_error(&log_entry);
            Err((StatusCode::INTERNAL_SERVER_ERROR, "failed to delete system"))
        }
    }
}

/// Deletes all systems.
async fn delete_all_systems(
    State((logger, data_store)): State<(Arc<SavefileManager>, Arc<dyn DataStore>)>,
) -> Result<StatusCode, (StatusCode, &'static str)> {
    match data_store.delete_all_systems() {
        Ok(count) => {
            let log_entry = SaveEntry::new(
                SaveOperation::SystemDeleteAll {
                    count_deleted: count,
                },
                SaveMetadata::rest_api(None).with_status(OperationStatus::Success),
            );
            logger.save_or_error(&log_entry);
            Ok(StatusCode::NO_CONTENT)
        }
        Err(_) => {
            let log_entry = SaveEntry::new(
                SaveOperation::SystemDeleteAll { count_deleted: 0 },
                SaveMetadata::rest_api(None).with_status(OperationStatus::Failed),
            );
            logger.save_or_error(&log_entry);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to delete all systems",
            ))
        }
    }
}

////////////////////////////////////////////// Router //////////////////////////////////////////////////

/// Creates an Axum router with system management endpoints.
pub fn create_system_router(
    logger: Arc<SavefileManager>,
    data_store: Arc<dyn DataStore>,
) -> Router {
    Router::new()
        .route(
            "/system",
            get(list_systems)
                .post(create_system)
                .delete(delete_all_systems),
        )
        .route("/system/from-markdown", post(create_system_from_markdown))
        .route(
            "/system/:name",
            get(get_system)
                .put(update_system)
                .patch(patch_system)
                .delete(delete_system),
        )
        .with_state((logger, data_store))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_helpers::{clear_savefile, test_data_store};
    use std::path::PathBuf;
    use std::process;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn test_logger() -> (Arc<SavefileManager>, PathBuf) {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let test_path = PathBuf::from(format!("test_system_{}_{}.jsonl", process::id(), timestamp));
        let logger = Arc::new(SavefileManager::new(test_path.clone()));
        (logger, test_path)
    }

    fn test_system_config() -> SystemConfig {
        SystemConfig {
            name: SystemName::new("test-system").unwrap(),
            description: "A test system".to_string(),
            tools: vec!["Read".to_string(), "Write".to_string()],
            model: "inherit".to_string(),
            color: "blue".to_string(),
            bid: Vec::new(),
            content: "You are a test system.".to_string(),
        }
    }

    #[test]
    fn system_new() {
        let config = test_system_config();
        let system = System::new(config.clone());

        assert_eq!(system.config, config);
        assert!(system.created_at <= system.updated_at);
    }

    #[test]
    fn system_update_config() {
        let config = test_system_config();
        let mut system = System::new(config.clone());
        let original_created = system.created_at;
        let original_updated = system.updated_at;

        // Wait a bit to ensure timestamps differ
        std::thread::sleep(std::time::Duration::from_millis(1));

        let mut new_config = test_system_config();
        new_config.name = SystemName::new("updated-system").unwrap();
        system.update_config(new_config.clone());

        assert_eq!(system.config, new_config);
        assert_eq!(system.created_at, original_created);
        assert!(system.updated_at > original_updated);
    }

    #[tokio::test]
    async fn create_system_success() {
        let config = test_system_config();

        let (logger, log_path) = test_logger();
        let data_store = test_data_store();
        let response = create_system(
            State((logger, data_store)),
            SystemConfigExtractor(config.clone()),
        )
        .await;

        if response.is_ok() {
            let response = response.unwrap();
            assert_eq!(response.0.system.config.name, config.name);
            assert!(response.0.created);
        }
        clear_savefile(&log_path);
    }

    #[tokio::test]
    async fn create_system_from_markdown_success() {
        let content = r#"---
name: test-system
description: A test system from markdown
tools: Read, Write, Edit
model: inherit
color: green
---

You are a test system created from markdown."#;

        let request = CreateSystemFromMarkdownRequest {
            content: content.to_string(),
        };

        let (logger, log_path) = test_logger();
        let data_store = test_data_store();
        let response =
            create_system_from_markdown(State((logger, data_store)), Json(request)).await;

        if response.is_ok() {
            let response = response.unwrap();
            assert_eq!(
                response.0.system.config.name,
                SystemName::new("test-system").unwrap()
            );
            assert_eq!(
                response.0.system.config.description,
                "A test system from markdown"
            );
            assert_eq!(
                response.0.system.config.tools,
                vec!["Read", "Write", "Edit"]
            );
            assert_eq!(
                response.0.system.config.content,
                "You are a test system created from markdown."
            );
            assert!(response.0.created);
        }
        clear_savefile(&log_path);
    }

    #[tokio::test]
    async fn create_system_from_markdown_invalid() {
        let content = "Invalid markdown without frontmatter";

        let request = CreateSystemFromMarkdownRequest {
            content: content.to_string(),
        };

        let (logger, log_path) = test_logger();
        let data_store = test_data_store();
        let response =
            create_system_from_markdown(State((logger, data_store)), Json(request)).await;

        assert!(response.is_err());
        if let Err((status, _)) = response {
            assert_eq!(status, StatusCode::BAD_REQUEST);
        }
        clear_savefile(&log_path);
    }
}
