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

use crate::{SystemConfig, SystemParser};

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
    State(pool): State<sqlx::PgPool>,
    SystemConfigExtractor(config): SystemConfigExtractor,
) -> Result<Json<CreateSystemResponse>, (StatusCode, &'static str)> {
    if config.validate().is_err() {
        return Err((StatusCode::BAD_REQUEST, "Invalid system configuration"));
    }

    let system = System::new(config);

    let mut tx = pool.begin().await.map_err(|_e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to begin transaction",
        )
    })?;

    match crate::sql::system::create(&mut tx, &system).await {
        Ok(()) => {
            tx.commit().await.map_err(|_e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "failed to commit transaction",
                )
            })?;
            let response = CreateSystemResponse {
                system,
                created: true,
            };
            Ok(Json(response))
        }
        Err(crate::DataStoreError::AlreadyExists) => {
            Err((StatusCode::CONFLICT, "system with this name already exists"))
        }
        Err(_) => Err((StatusCode::INTERNAL_SERVER_ERROR, "failed to create system")),
    }
}

/// Creates a new system from markdown content.
async fn create_system_from_markdown(
    State(pool): State<sqlx::PgPool>,
    Json(request): Json<CreateSystemFromMarkdownRequest>,
) -> Result<Json<CreateSystemResponse>, (StatusCode, String)> {
    let config = match SystemParser::parse(&request.content) {
        Ok(config) => config,
        Err(e) => {
            return Err((
                StatusCode::BAD_REQUEST,
                format!("Failed to parse markdown: {}", e),
            ));
        }
    };

    match create_system(State(pool), SystemConfigExtractor(config)).await {
        Ok(response) => Ok(response),
        Err((status, msg)) => Err((status, msg.to_string())),
    }
}

/// Lists all systems.
async fn list_systems(
    State(pool): State<sqlx::PgPool>,
) -> Result<Json<Vec<SystemListItem>>, (StatusCode, &'static str)> {
    let mut tx = pool.begin().await.map_err(|_e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to begin transaction",
        )
    })?;

    match crate::sql::system::list(&mut tx).await {
        Ok(systems) => {
            tx.commit().await.map_err(|_e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "failed to commit transaction",
                )
            })?;
            let system_list: Vec<SystemListItem> =
                systems.into_iter().map(|system| system.into()).collect();
            Ok(Json(system_list))
        }
        Err(_) => Err((StatusCode::INTERNAL_SERVER_ERROR, "failed to list systems")),
    }
}

/// Gets a system by its name.
async fn get_system(
    State(pool): State<sqlx::PgPool>,
    Path(name): Path<String>,
) -> Result<Json<System>, (StatusCode, &'static str)> {
    let system_name = match SystemName::new(&name) {
        Some(n) => n,
        None => {
            return Err((StatusCode::BAD_REQUEST, "invalid system name"));
        }
    };

    let mut tx = pool.begin().await.map_err(|_e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to begin transaction",
        )
    })?;

    match crate::sql::system::get(&mut tx, &system_name).await {
        Ok(Some(system)) => {
            tx.commit().await.map_err(|_e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "failed to commit transaction",
                )
            })?;
            Ok(Json(system))
        }
        Ok(None) => Err((StatusCode::NOT_FOUND, "system not found")),
        Err(_) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to retrieve system",
        )),
    }
}

/// Updates a system.
async fn update_system(
    State(pool): State<sqlx::PgPool>,
    Path(name): Path<String>,
    SystemConfigExtractor(config): SystemConfigExtractor,
) -> Result<Json<System>, (StatusCode, String)> {
    if let Err(e) = config.validate() {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("Invalid system configuration: {}", e),
        ));
    }

    let system_name = match SystemName::new(&name) {
        Some(n) => n,
        None => {
            return Err((StatusCode::BAD_REQUEST, "invalid system name".to_string()));
        }
    };

    let mut tx = pool.begin().await.map_err(|_e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to begin transaction".to_string(),
        )
    })?;

    let old_system = match crate::sql::system::get(&mut tx, &system_name).await {
        Ok(Some(system)) => system,
        Ok(None) => {
            return Err((StatusCode::NOT_FOUND, "system not found".to_string()));
        }
        Err(_) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to retrieve system".to_string(),
            ));
        }
    };

    let mut updated_system = old_system;
    updated_system.update_config(config);

    match crate::sql::system::update(&mut tx, &updated_system).await {
        Ok(true) => {
            tx.commit().await.map_err(|_e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "failed to commit transaction".to_string(),
                )
            })?;
            Ok(Json(updated_system))
        }
        Ok(false) => Err((StatusCode::NOT_FOUND, "system not found".to_string())),
        Err(_) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to update system".to_string(),
        )),
    }
}

/// Patches a system.
async fn patch_system(
    State(pool): State<sqlx::PgPool>,
    Path(name): Path<String>,
    Json(patch_data): Json<Value>,
) -> Result<Json<System>, (StatusCode, String)> {
    let system_name = match SystemName::new(&name) {
        Some(n) => n,
        None => {
            return Err((StatusCode::BAD_REQUEST, "invalid system name".to_string()));
        }
    };

    let mut tx = pool.begin().await.map_err(|_e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to begin transaction".to_string(),
        )
    })?;

    let mut system = match crate::sql::system::get(&mut tx, &system_name).await {
        Ok(Some(system)) => system,
        Ok(None) => {
            return Err((StatusCode::NOT_FOUND, "system not found".to_string()));
        }
        Err(_) => {
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

    system.update_config(config);

    match crate::sql::system::update(&mut tx, &system).await {
        Ok(true) => {
            tx.commit().await.map_err(|_e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "failed to commit transaction".to_string(),
                )
            })?;
            Ok(Json(system))
        }
        Ok(false) => Err((StatusCode::NOT_FOUND, "system not found".to_string())),
        Err(_) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to update system".to_string(),
        )),
    }
}

/// Deletes a system.
async fn delete_system(
    State(pool): State<sqlx::PgPool>,
    Path(name): Path<String>,
) -> Result<StatusCode, (StatusCode, &'static str)> {
    let system_name = match SystemName::new(&name) {
        Some(n) => n,
        None => {
            return Err((StatusCode::BAD_REQUEST, "invalid system name"));
        }
    };

    let mut tx = pool.begin().await.map_err(|_e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to begin transaction",
        )
    })?;

    match crate::sql::system::delete(&mut tx, &system_name).await {
        Ok(true) => {
            tx.commit().await.map_err(|_e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "failed to commit transaction",
                )
            })?;
            Ok(StatusCode::NO_CONTENT)
        }
        Ok(false) => Err((StatusCode::NOT_FOUND, "system not found")),
        Err(_) => Err((StatusCode::INTERNAL_SERVER_ERROR, "failed to delete system")),
    }
}

/// Deletes all systems.
async fn delete_all_systems(
    State(pool): State<sqlx::PgPool>,
) -> Result<StatusCode, (StatusCode, &'static str)> {
    let mut tx = pool.begin().await.map_err(|_e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to begin transaction",
        )
    })?;

    match crate::sql::system::delete_all(&mut tx).await {
        Ok(_) => {
            tx.commit().await.map_err(|_e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "failed to commit transaction",
                )
            })?;
            Ok(StatusCode::NO_CONTENT)
        }
        Err(_) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to delete all systems",
        )),
    }
}

////////////////////////////////////////////// Router //////////////////////////////////////////////////

/// Creates an Axum router with system management endpoints.
pub fn create_system_router(pool: sqlx::PgPool) -> Router {
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
        .with_state(pool)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_system_config() -> SystemConfig {
        SystemConfig {
            name: SystemName::new("test-system").unwrap(),
            description: "A test system".to_string(),
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

        std::thread::sleep(std::time::Duration::from_millis(1));

        let mut new_config = test_system_config();
        new_config.name = SystemName::new("updated-system").unwrap();
        system.update_config(new_config.clone());

        assert_eq!(system.config, new_config);
        assert_eq!(system.created_at, original_created);
        assert!(system.updated_at > original_updated);
    }
}
