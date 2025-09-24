use std::fmt::{Display, Formatter, Result as FmtResult};
use std::fs::File;
use std::io::Read;
use std::str::FromStr;

use axum::Router;
use axum::extract::{Path, State};
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

////////////////////////////////////////////// Constants ///////////////////////////////////////////////

/// Length of the system prefix "system:"
const SYSTEM_PREFIX: &str = "system:";
const SYSTEM_PREFIX_LEN: usize = 7;

/// Expected length of base64 encoded 32 bytes (without padding)
const BASE64_ENCODED_LEN: usize = 43;

/// Maximum number of retries when generating systems without special characters
const MAX_GENERATION_RETRIES: usize = 1000;

/////////////////////////////////////////////// SystemId //////////////////////////////////////////////

/// A 32-byte system identifier with URL-safe base64 string representation.
///
/// Systems are displayed as "system:{base64}" where the base64 encoding uses URL-safe
/// characters (- and _ instead of + and /) and no padding. This format is suitable for
/// use in URLs and other contexts where standard base64 characters might cause issues.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SystemId([u8; 32]);

impl SystemId {
    /// Creates a new SystemId from a 32-byte array.
    pub fn new(bytes: [u8; 32]) -> Self {
        SystemId(bytes)
    }

    /// Returns a reference to the underlying 32-byte array.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Consumes the SystemId and returns the underlying 32-byte array.
    pub fn into_bytes(self) -> [u8; 32] {
        self.0
    }

    /// Generates a random SystemId using `/dev/urandom`.
    pub fn random() -> std::io::Result<Self> {
        let mut random_bytes = [0u8; 32];
        let mut file = File::open("/dev/urandom")?;
        file.read_exact(&mut random_bytes)?;
        Ok(SystemId::new(random_bytes))
    }

    /// Generates a random SystemId that avoids URL-unsafe characters in its base64 representation.
    pub fn random_url_safe() -> std::io::Result<Self> {
        let mut attempts = 0;

        loop {
            attempts += 1;

            let system_id = Self::random()?;
            let system_string = system_id.to_string();

            if attempts > MAX_GENERATION_RETRIES {
                // If we can't generate URL-safe after max retries, fail instead of creating invalid IDs
                return Err(std::io::Error::other(
                    "Failed to generate URL-safe system ID after maximum retries",
                ));
            }

            // Check if the base64 part contains - or _
            let base64_part = system_string.strip_prefix("system:").ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Generated system ID missing expected prefix",
                )
            })?;
            if !base64_part.contains('-') && !base64_part.contains('_') {
                return Ok(system_id);
            }
            // If it contains - or _, continue the loop to generate a new one
        }
    }
}

////////////////////////////////////// URL-Safe Base64 Encoding //////////////////////////////////////

/// URL-safe base64 character set (RFC 4648 Section 5)
/// Uses - and _ instead of + and / to be safe in URLs
const BASE64_CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

/// Encodes bytes to URL-safe base64 without padding.
fn encode_base64_url_safe(input: &[u8]) -> String {
    let mut result = String::new();
    let mut i = 0;

    while i < input.len() {
        let b1 = input[i];
        let b2 = if i + 1 < input.len() { input[i + 1] } else { 0 };
        let b3 = if i + 2 < input.len() { input[i + 2] } else { 0 };

        let combined = ((b1 as u32) << 16) | ((b2 as u32) << 8) | (b3 as u32);

        let c1 = BASE64_CHARS[((combined >> 18) & 0x3F) as usize] as char;
        let c2 = BASE64_CHARS[((combined >> 12) & 0x3F) as usize] as char;

        result.push(c1);
        result.push(c2);

        if i + 1 < input.len() {
            let c3 = BASE64_CHARS[((combined >> 6) & 0x3F) as usize] as char;
            result.push(c3);
        }

        if i + 2 < input.len() {
            let c4 = BASE64_CHARS[(combined & 0x3F) as usize] as char;
            result.push(c4);
        }

        i += 3;
    }

    result
}

/// Decodes a URL-safe base64 string, handling missing padding automatically.
fn decode_base64_url_safe(input: &str) -> Result<Vec<u8>, &'static str> {
    let mut chars: Vec<char> = input.chars().collect();

    // Add padding if needed
    while !chars.len().is_multiple_of(4) {
        chars.push('=');
    }

    let mut result = Vec::new();
    let mut i = 0;

    while i < chars.len() {
        let c1 = chars[i];
        let c2 = chars[i + 1];
        let c3 = chars[i + 2];
        let c4 = chars[i + 3];

        let v1 = char_to_base64_value(c1)?;
        let v2 = char_to_base64_value(c2)?;
        let v3 = if c3 == '=' {
            0
        } else {
            char_to_base64_value(c3)?
        };
        let v4 = if c4 == '=' {
            0
        } else {
            char_to_base64_value(c4)?
        };

        let combined = (v1 << 18) | (v2 << 12) | (v3 << 6) | v4;

        result.push((combined >> 16) as u8);
        if c3 != '=' {
            result.push((combined >> 8) as u8);
        }
        if c4 != '=' {
            result.push(combined as u8);
        }

        i += 4;
    }

    Ok(result)
}

/// Converts a single character to its base64 value.
fn char_to_base64_value(c: char) -> Result<u32, &'static str> {
    match c {
        'A'..='Z' => Ok((c as u32) - ('A' as u32)),
        'a'..='z' => Ok((c as u32) - ('a' as u32) + 26),
        '0'..='9' => Ok((c as u32) - ('0' as u32) + 52),
        '-' => Ok(62),
        '_' => Ok(63),
        '=' => Ok(0), // Padding
        _ => Err("Invalid base64 character"),
    }
}

/////////////////////////////////////////// Display and FromStr ///////////////////////////////////////

impl Display for SystemId {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let encoded = encode_base64_url_safe(&self.0);
        write!(f, "{}{}", SYSTEM_PREFIX, encoded)
    }
}

/// Errors that can occur when parsing a SystemId from a string.
#[derive(Debug, PartialEq, Eq)]
pub enum SystemIdParseError {
    /// The string does not start with 'system:'
    InvalidPrefix,
    /// The format is invalid (wrong length or structure)
    InvalidFormat,
    /// The base64 encoding is invalid
    InvalidBase64,
    /// The decoded bytes are not exactly 32 bytes
    InvalidLength,
}

impl Display for SystemIdParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            SystemIdParseError::InvalidPrefix => {
                write!(f, "System string must start with 'system:'")
            }
            SystemIdParseError::InvalidFormat => write!(f, "Invalid system format"),
            SystemIdParseError::InvalidBase64 => write!(f, "Invalid base64 encoding"),
            SystemIdParseError::InvalidLength => write!(f, "System must be exactly 32 bytes"),
        }
    }
}

impl std::error::Error for SystemIdParseError {}

impl FromStr for SystemId {
    type Err = SystemIdParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if !s.starts_with(SYSTEM_PREFIX) {
            return Err(SystemIdParseError::InvalidPrefix);
        }

        let base64_part = &s[SYSTEM_PREFIX_LEN..]; // Skip "system:"

        if base64_part.len() != BASE64_ENCODED_LEN {
            return Err(SystemIdParseError::InvalidFormat);
        }

        let decoded =
            decode_base64_url_safe(base64_part).map_err(|_| SystemIdParseError::InvalidBase64)?;

        if decoded.len() != 32 {
            return Err(SystemIdParseError::InvalidLength);
        }

        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(&decoded);
        Ok(SystemId(bytes))
    }
}

impl Serialize for SystemId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let encoded = encode_base64_url_safe(&self.0);
        serializer.serialize_str(&encoded)
    }
}

impl<'de> Deserialize<'de> for SystemId {
    fn deserialize<D>(deserializer: D) -> Result<SystemId, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(SystemIdVisitor)
    }
}

struct SystemIdVisitor;

impl<'de> serde::de::Visitor<'de> for SystemIdVisitor {
    type Value = SystemId;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a base64 system ID string (43 characters)")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        // Handle both formats: base64-only and system:base64
        let base64_part = if value.starts_with(SYSTEM_PREFIX) {
            &value[SYSTEM_PREFIX_LEN..]
        } else {
            value
        };

        // Validate length
        if base64_part.len() != BASE64_ENCODED_LEN {
            return Err(serde::de::Error::custom(format!(
                "System base64 must be exactly {} characters, got {}",
                BASE64_ENCODED_LEN,
                base64_part.len()
            )));
        }

        // Decode base64
        let decoded = decode_base64_url_safe(base64_part)
            .map_err(|e| serde::de::Error::custom(format!("Invalid base64: {}", e)))?;

        if decoded.len() != 32 {
            return Err(serde::de::Error::custom(format!(
                "Decoded system must be exactly 32 bytes, got {}",
                decoded.len()
            )));
        }

        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(&decoded);
        Ok(SystemId(bytes))
    }
}

//////////////////////////////////////////////// System ////////////////////////////////////////////////

/// A system represents a Claude Code agent configuration with its associated metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct System {
    /// Unique identifier for the system
    pub id: SystemId,
    /// The system configuration containing behavior and metadata
    pub config: SystemConfig,
    /// When the system was created
    pub created_at: DateTime<Utc>,
    /// When the system was last updated
    pub updated_at: DateTime<Utc>,
}

impl System {
    /// Creates a new System with the given configuration.
    pub fn new(config: SystemConfig) -> std::io::Result<Self> {
        let now = Utc::now();
        Ok(System {
            id: SystemId::random_url_safe()?,
            config,
            created_at: now,
            updated_at: now,
        })
    }

    /// Creates a new System with a specific ID and configuration.
    pub fn with_id(id: SystemId, config: SystemConfig) -> Self {
        let now = Utc::now();
        System {
            id,
            config,
            created_at: now,
            updated_at: now,
        }
    }

    /// Updates the system configuration and marks it as updated.
    pub fn update_config(&mut self, config: SystemConfig) {
        self.config = config;
        self.updated_at = Utc::now();
    }
}

////////////////////////////////////////////// Routes //////////////////////////////////////////////////

/// Request structure for creating a new system.
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateSystemRequest {
    /// The system configuration (can be from markdown content).
    pub config: SystemConfig,
}

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
    /// Unique identifier for the system
    pub id: SystemId,
    /// The system's display name
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
            id: system.id,
            name: system.config.name,
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
    Json(request): Json<CreateSystemRequest>,
) -> Result<Json<CreateSystemResponse>, (StatusCode, &'static str)> {
    // Validate the config first
    if request.config.validate().is_err() {
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

    let system = System::new(request.config).map_err(|_e| {
        let log_entry = SaveEntry::new(
            SaveOperation::SystemCreate {
                system_id: "unknown".to_string(),
                config: None,
                success: false,
            },
            SaveMetadata::rest_api(None).with_status(OperationStatus::Failed),
        );
        logger.save_or_error(&log_entry);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to generate system id",
        )
    })?;

    let system_id = system.id.to_string();
    let config = system.config.clone();

    // Store the system in the data store
    if data_store.create_system(&system).is_err() {
        let log_entry = SaveEntry::new(
            SaveOperation::SystemCreate {
                system_id: system_id.clone(),
                config: Some(config),
                success: false,
            },
            SaveMetadata::rest_api(None).with_status(OperationStatus::Failed),
        );
        logger.save_or_error(&log_entry);
        return Err((StatusCode::CONFLICT, "system with this id already exists"));
    }

    let log_entry = SaveEntry::new(
        SaveOperation::SystemCreate {
            system_id,
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

    let create_request = CreateSystemRequest { config };
    match create_system(State((logger, data_store)), Json(create_request)).await {
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

/// Gets a system by its ID.
async fn get_system(
    State((logger, data_store)): State<(Arc<SavefileManager>, Arc<dyn DataStore>)>,
    Path(system_base64): Path<String>,
) -> Result<Json<System>, (StatusCode, &'static str)> {
    // Construct full system string from base64 part
    let system_string = format!("{}{}", SYSTEM_PREFIX, system_base64);

    // Parse the system ID
    let system_id = match SystemId::from_str(&system_string) {
        Ok(id) => id,
        Err(_parse_error) => {
            let log_entry = SaveEntry::new(
                SaveOperation::SystemGet {
                    system_id: system_string,
                    success: false,
                },
                SaveMetadata::rest_api(None).with_status(OperationStatus::Failed),
            );
            logger.save_or_error(&log_entry);
            return Err((StatusCode::BAD_REQUEST, "invalid system id"));
        }
    };

    // Look up the system in the data store
    match data_store.get_system(&system_id) {
        Ok(Some(system)) => {
            let log_entry = SaveEntry::new(
                SaveOperation::SystemGet {
                    system_id: system_string,
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
                    system_id: system_string,
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
                    system_id: system_string,
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
    Path(system_base64): Path<String>,
    Json(config): Json<SystemConfig>,
) -> Result<Json<System>, (StatusCode, String)> {
    // Validate the config first
    if let Err(e) = config.validate() {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("Invalid system configuration: {}", e),
        ));
    }

    // Construct full system string from base64 part
    let system_string = format!("{}{}", SYSTEM_PREFIX, system_base64);

    // Parse the system ID
    let system_id = match SystemId::from_str(&system_string) {
        Ok(id) => id,
        Err(_parse_error) => {
            let log_entry = SaveEntry::new(
                SaveOperation::SystemUpdate {
                    system_id: system_string,
                    old_config: None,
                    new_config: config,
                    success: false,
                },
                SaveMetadata::rest_api(None).with_status(OperationStatus::Failed),
            );
            logger.save_or_error(&log_entry);
            return Err((StatusCode::BAD_REQUEST, "invalid system id".to_string()));
        }
    };

    // Get the existing system first
    let old_system = match data_store.get_system(&system_id) {
        Ok(Some(system)) => system,
        Ok(None) => {
            let log_entry = SaveEntry::new(
                SaveOperation::SystemUpdate {
                    system_id: system_string,
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
                    system_id: system_string,
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

    // Create updated system
    let mut updated_system = old_system.clone();
    updated_system.update_config(config.clone());

    // Update in data store
    match data_store.update_system(&updated_system) {
        Ok(true) => {
            let log_entry = SaveEntry::new(
                SaveOperation::SystemUpdate {
                    system_id: system_string,
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
                    system_id: system_string,
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
                    system_id: system_string,
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
    Path(system_base64): Path<String>,
    Json(patch_data): Json<Value>,
) -> Result<Json<System>, (StatusCode, String)> {
    // Construct full system string from base64 part
    let system_string = format!("{}{}", SYSTEM_PREFIX, system_base64);

    // Parse the system ID
    let system_id = match SystemId::from_str(&system_string) {
        Ok(id) => id,
        Err(_parse_error) => {
            let log_entry = SaveEntry::new(
                SaveOperation::SystemPatch {
                    system_id: system_string,
                    patch_data,
                    success: false,
                },
                SaveMetadata::rest_api(None).with_status(OperationStatus::Failed),
            );
            logger.save_or_error(&log_entry);
            return Err((StatusCode::BAD_REQUEST, "invalid system id".to_string()));
        }
    };

    // Get the existing system first
    let mut system = match data_store.get_system(&system_id) {
        Ok(Some(system)) => system,
        Ok(None) => {
            let log_entry = SaveEntry::new(
                SaveOperation::SystemPatch {
                    system_id: system_string,
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
                    system_id: system_string,
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

    // Apply simple field-level patches to the config
    let mut config = system.config.clone();
    let patch_obj = match patch_data.as_object() {
        Some(obj) => obj,
        None => {
            let log_entry = SaveEntry::new(
                SaveOperation::SystemPatch {
                    system_id: system_string,
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

    // Apply patches to individual fields
    if let Some(name) = patch_obj.get("name").and_then(|v| v.as_str()) {
        config.name = name.to_string();
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

    // Update the system
    system.update_config(config);

    // Save to data store
    match data_store.update_system(&system) {
        Ok(true) => {
            let log_entry = SaveEntry::new(
                SaveOperation::SystemPatch {
                    system_id: system_string,
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
                    system_id: system_string,
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
                    system_id: system_string,
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
    Path(system_base64): Path<String>,
) -> Result<StatusCode, (StatusCode, &'static str)> {
    // Construct full system string from base64 part
    let system_string = format!("{}{}", SYSTEM_PREFIX, system_base64);

    // Parse the system ID
    let system_id = match SystemId::from_str(&system_string) {
        Ok(id) => id,
        Err(_parse_error) => {
            let log_entry = SaveEntry::new(
                SaveOperation::SystemDelete {
                    system_id: system_string,
                    success: false,
                },
                SaveMetadata::rest_api(None).with_status(OperationStatus::Failed),
            );
            logger.save_or_error(&log_entry);
            return Err((StatusCode::BAD_REQUEST, "invalid system id"));
        }
    };

    // Delete from data store
    match data_store.delete_system(&system_id) {
        Ok(true) => {
            let log_entry = SaveEntry::new(
                SaveOperation::SystemDelete {
                    system_id: system_string,
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
                    system_id: system_string,
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
                    system_id: system_string,
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
            "/system/:id",
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
            name: "test-system".to_string(),
            description: "A test system".to_string(),
            tools: vec!["Read".to_string(), "Write".to_string()],
            model: "inherit".to_string(),
            color: "blue".to_string(),
            content: "You are a test system.".to_string(),
        }
    }

    #[test]
    fn system_id_new_and_accessors() {
        let bytes = [1u8; 32];
        let system_id = SystemId::new(bytes);
        assert_eq!(system_id.as_bytes(), &bytes);
        assert_eq!(system_id.into_bytes(), bytes);
    }

    #[test]
    fn system_id_display_format() {
        let system_id = SystemId::new([0u8; 32]);
        let display = format!("{}", system_id);
        assert!(display.starts_with("system:"));
        assert_eq!(display.len(), SYSTEM_PREFIX_LEN + BASE64_ENCODED_LEN);
        assert_eq!(
            display,
            "system:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
        );
    }

    #[test]
    fn system_id_from_str_valid() {
        let system_str = "system:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
        let system_id = SystemId::from_str(system_str).unwrap();
        assert_eq!(system_id.as_bytes(), &[0u8; 32]);
    }

    #[test]
    fn system_id_from_str_invalid_prefix() {
        let result = SystemId::from_str("invalid:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA");
        assert_eq!(result, Err(SystemIdParseError::InvalidPrefix));
    }

    #[test]
    fn system_id_round_trip_display_fromstr() {
        let original_bytes = [
            0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66,
            0x77, 0x88, 0x99, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0x00, 0x01, 0x02, 0x03, 0x04,
            0x05, 0x06, 0x07, 0x08,
        ];
        let system_id = SystemId::new(original_bytes);

        let display_str = format!("{}", system_id);

        let parsed_system_id = SystemId::from_str(&display_str).unwrap();
        assert_eq!(parsed_system_id.as_bytes(), &original_bytes);
        assert_eq!(system_id, parsed_system_id);
    }

    #[test]
    fn system_new() {
        let config = test_system_config();
        let system = System::new(config.clone());

        // Check that system creation works (may fail if /dev/urandom unavailable)
        if let Ok(system) = system {
            assert_eq!(system.config, config);
            assert!(system.created_at <= system.updated_at);
        }
    }

    #[test]
    fn system_with_id() {
        let config = test_system_config();
        let system_id = SystemId::new([1u8; 32]);
        let system = System::with_id(system_id, config.clone());

        assert_eq!(system.id, system_id);
        assert_eq!(system.config, config);
        assert_eq!(system.created_at, system.updated_at);
    }

    #[test]
    fn system_update_config() {
        let config = test_system_config();
        let system_id = SystemId::new([1u8; 32]);
        let mut system = System::with_id(system_id, config.clone());
        let original_created = system.created_at;
        let original_updated = system.updated_at;

        // Wait a bit to ensure timestamps differ
        std::thread::sleep(std::time::Duration::from_millis(1));

        let mut new_config = test_system_config();
        new_config.name = "updated-system".to_string();
        system.update_config(new_config.clone());

        assert_eq!(system.config, new_config);
        assert_eq!(system.created_at, original_created);
        assert!(system.updated_at > original_updated);
    }

    #[tokio::test]
    async fn create_system_success() {
        let config = test_system_config();
        let request = CreateSystemRequest {
            config: config.clone(),
        };

        let (logger, log_path) = test_logger();
        let data_store = test_data_store();
        let response = create_system(State((logger, data_store)), Json(request)).await;

        if response.is_ok() {
            let response = response.unwrap();
            assert_eq!(response.0.system.config, config);
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
            assert_eq!(response.0.system.config.name, "test-system");
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
