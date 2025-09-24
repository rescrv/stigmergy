use std::fmt::{Display, Formatter, Result as FmtResult};
use std::fs::File;
use std::io::Read;
use std::str::FromStr;

////////////////////////////////////////////// Constants ///////////////////////////////////////////////

/// Length of the entity prefix "entity:"
const ENTITY_PREFIX: &str = "entity:";
const ENTITY_PREFIX_LEN: usize = 7;

/// Expected length of base64 encoded 32 bytes (without padding)
const BASE64_ENCODED_LEN: usize = 43;

/// Maximum number of retries when generating entities without special characters
const MAX_GENERATION_RETRIES: usize = 1000;

use axum::Router;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::Json;
use axum::routing::{delete, get};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::{
    DataStore, DataStoreOperations, OperationStatus, SaveEntry, SaveMetadata, SaveOperation,
    SavefileManager,
};

/////////////////////////////////////////////// Entity ////////////////////////////////////////////////

/// A 32-byte entity identifier with URL-safe base64 string representation.
///
/// Entities are displayed as "entity:{base64}" where the base64 encoding uses URL-safe
/// characters (- and _ instead of + and /) and no padding. This format is suitable for
/// use in URLs and other contexts where standard base64 characters might cause issues.
///
/// # Examples
///
/// ```
/// # use stigmergy::Entity;
/// let entity = Entity::new([1u8; 32]);
/// let entity_string = entity.to_string();
/// assert!(entity_string.starts_with("entity:"));
///
/// // Parse from string
/// let parsed: Entity = entity_string.parse().unwrap();
/// assert_eq!(entity, parsed);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Entity([u8; 32]);

impl Entity {
    /// Creates a new Entity from a 32-byte array.
    ///
    /// # Arguments
    /// * `bytes` - A 32-byte array representing the entity identifier
    ///
    /// # Examples
    /// ```
    /// # use stigmergy::Entity;
    /// let entity = Entity::new([0u8; 32]);
    /// ```
    pub fn new(bytes: [u8; 32]) -> Self {
        Entity(bytes)
    }

    /// Returns a reference to the underlying 32-byte array.
    ///
    /// # Examples
    /// ```
    /// # use stigmergy::Entity;
    /// let entity = Entity::new([1u8; 32]);
    /// assert_eq!(entity.as_bytes(), &[1u8; 32]);
    /// ```
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Consumes the Entity and returns the underlying 32-byte array.
    ///
    /// # Examples
    /// ```
    /// # use stigmergy::Entity;
    /// let entity = Entity::new([1u8; 32]);
    /// let bytes = entity.into_bytes();
    /// assert_eq!(bytes, [1u8; 32]);
    /// ```
    pub fn into_bytes(self) -> [u8; 32] {
        self.0
    }

    /// Generates a random Entity using `/dev/urandom`.
    ///
    /// # Returns
    /// * `Ok(Entity)` - A randomly generated entity on success
    /// * `Err(std::io::Error)` - An error if random number generation fails
    ///
    /// # Examples
    /// ```no_run
    /// # use stigmergy::Entity;
    /// let entity = Entity::random().unwrap();
    /// ```
    pub fn random() -> std::io::Result<Self> {
        let mut random_bytes = [0u8; 32];
        let mut file = File::open("/dev/urandom")?;
        file.read_exact(&mut random_bytes)?;
        Ok(Entity::new(random_bytes))
    }

    /// Generates a random Entity that avoids URL-unsafe characters in its base64 representation.
    ///
    /// This method generates random entities until it finds one whose base64 encoding
    /// doesn't contain `-` or `_` characters. After MAX_GENERATION_RETRIES attempts,
    /// it will accept any entity and replace `-` with `9` and `_` with `6` to ensure
    /// URL-safe output.
    ///
    /// # Returns
    /// * `Ok(Entity)` - A randomly generated entity without URL-unsafe characters
    /// * `Err(std::io::Error)` - An error if random number generation fails
    ///
    /// # Examples
    /// ```no_run
    /// # use stigmergy::Entity;
    /// let entity = Entity::random_url_safe().unwrap();
    /// let entity_string = entity.to_string();
    /// let base64_part = &entity_string[7..]; // Skip "entity:"
    /// assert!(!base64_part.contains('-') && !base64_part.contains('_'));
    /// ```
    pub fn random_url_safe() -> std::io::Result<Self> {
        let mut attempts = 0;

        loop {
            attempts += 1;

            let entity = Self::random()?;
            let entity_string = entity.to_string();

            if attempts > MAX_GENERATION_RETRIES {
                // Fallback: accept any valid entity after max retries and clean it up
                let cleaned_entity_string = entity_string.replace('-', "9").replace('_', "6");
                return Ok(cleaned_entity_string.parse().unwrap());
            }

            // Check if the base64 part contains - or _
            let base64_part = &entity_string[ENTITY_PREFIX_LEN..];
            if !base64_part.contains('-') && !base64_part.contains('_') {
                return Ok(entity);
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
///
/// This implementation uses the URL-safe alphabet (RFC 4648 Section 5) and
/// omits padding characters for a cleaner representation.
///
/// # Arguments
/// * `input` - The bytes to encode
///
/// # Returns
/// A URL-safe base64 encoded string without padding
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
///
/// # Arguments
/// * `input` - The base64 string to decode
///
/// # Returns
/// * `Ok(Vec<u8>)` - The decoded bytes on success
/// * `Err(&'static str)` - An error message on failure
///
/// # Errors
/// Returns an error if the input contains invalid base64 characters.
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
///
/// # Arguments
/// * `c` - The character to convert
///
/// # Returns
/// * `Ok(u32)` - The base64 value (0-63) on success
/// * `Err(&'static str)` - An error message for invalid characters
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

impl Display for Entity {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let encoded = encode_base64_url_safe(&self.0);
        write!(f, "{}{}", ENTITY_PREFIX, encoded)
    }
}

/// Errors that can occur when parsing an Entity from a string.
#[derive(Debug, PartialEq, Eq)]
pub enum EntityParseError {
    InvalidPrefix,
    InvalidFormat,
    InvalidBase64,
    InvalidLength,
}

impl Display for EntityParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            EntityParseError::InvalidPrefix => write!(f, "Entity string must start with 'entity:'"),
            EntityParseError::InvalidFormat => write!(f, "Invalid entity format"),
            EntityParseError::InvalidBase64 => write!(f, "Invalid base64 encoding"),
            EntityParseError::InvalidLength => write!(f, "Entity must be exactly 32 bytes"),
        }
    }
}

impl std::error::Error for EntityParseError {}

impl FromStr for Entity {
    type Err = EntityParseError;

    /// Parses an Entity from its string representation.
    ///
    /// Expected format: "entity:{base64}" where base64 is 43 characters of URL-safe base64.
    ///
    /// # Arguments
    /// * `s` - The string to parse
    ///
    /// # Returns
    /// * `Ok(Entity)` - The parsed entity on success
    /// * `Err(EntityParseError)` - The specific parsing error on failure
    ///
    /// # Examples
    /// ```
    /// # use stigmergy::Entity;
    /// let entity: Entity = "entity:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA".parse().unwrap();
    /// ```
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if !s.starts_with(ENTITY_PREFIX) {
            return Err(EntityParseError::InvalidPrefix);
        }

        let base64_part = &s[ENTITY_PREFIX_LEN..]; // Skip "entity:"

        if base64_part.len() != BASE64_ENCODED_LEN {
            return Err(EntityParseError::InvalidFormat);
        }

        let decoded =
            decode_base64_url_safe(base64_part).map_err(|_| EntityParseError::InvalidBase64)?;

        if decoded.len() != 32 {
            return Err(EntityParseError::InvalidLength);
        }

        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(&decoded);
        Ok(Entity(bytes))
    }
}

impl Serialize for Entity {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let encoded = encode_base64_url_safe(&self.0);
        serializer.serialize_str(&encoded)
    }
}

impl<'de> Deserialize<'de> for Entity {
    fn deserialize<D>(deserializer: D) -> Result<Entity, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(EntityVisitor)
    }
}

struct EntityVisitor;

impl<'de> serde::de::Visitor<'de> for EntityVisitor {
    type Value = Entity;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a base64 entity ID string (43 characters)")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        // Handle both formats: base64-only and entity:base64
        let base64_part = if value.starts_with(ENTITY_PREFIX) {
            &value[ENTITY_PREFIX_LEN..]
        } else {
            value
        };

        // Validate length
        if base64_part.len() != BASE64_ENCODED_LEN {
            return Err(serde::de::Error::custom(format!(
                "Entity base64 must be exactly {} characters, got {}",
                BASE64_ENCODED_LEN,
                base64_part.len()
            )));
        }

        // Decode base64
        let decoded = decode_base64_url_safe(base64_part)
            .map_err(|e| serde::de::Error::custom(format!("Invalid base64: {}", e)))?;

        if decoded.len() != 32 {
            return Err(serde::de::Error::custom(format!(
                "Decoded entity must be exactly 32 bytes, got {}",
                decoded.len()
            )));
        }

        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(&decoded);
        Ok(Entity(bytes))
    }
}

////////////////////////////////////////////// Routes //////////////////////////////////////////////////

/// Request structure for creating a new entity.
///
/// If `entity` is `None`, a random entity will be generated that avoids
/// the URL-unsafe characters `-` and `_` in its base64 representation.
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateEntityRequest {
    /// Optional entity to create. If None, a random entity will be generated.
    pub entity: Option<Entity>,
}

/// Response structure for entity creation.
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateEntityResponse {
    /// The entity that was created or provided.
    pub entity: Entity,
    /// Whether the entity was successfully created (always true in current implementation).
    pub created: bool,
}

/// Creates a new entity.
///
/// If no entity is provided in the request, generates a random entity that avoids
/// URL-unsafe characters (-_) in its base64 representation. Uses a bounded retry
/// approach to prevent infinite loops.
///
/// # Errors
/// Returns `StatusCode::INTERNAL_SERVER_ERROR` if random number generation fails.
async fn create_entity(
    State((logger, data_store)): State<(Arc<SavefileManager>, Arc<dyn DataStore>)>,
    Json(request): Json<CreateEntityRequest>,
) -> Result<Json<CreateEntityResponse>, (StatusCode, &'static str)> {
    let was_random = request.entity.is_none();
    let entity = match request.entity {
        Some(entity) => entity,
        None => Entity::random_url_safe().map_err(|_e| {
            let save_entry = SaveEntry::new(
                SaveOperation::EntityCreate {
                    entity: Entity::new([0u8; 32]),
                    was_random,
                },
                SaveMetadata::rest_api(None).with_status(OperationStatus::Failed),
            );
            logger.save_or_error(&save_entry);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to generate random entity",
            )
        })?,
    };

    // Store the entity in the data store using the standardized operation
    let result = DataStoreOperations::create_entity(&*data_store, &entity);

    let save_entry = SaveEntry::new(
        SaveOperation::EntityCreate { entity, was_random },
        SaveMetadata::rest_api(None).with_status(if result.success {
            OperationStatus::Success
        } else {
            OperationStatus::Failed
        }),
    );
    logger.save_or_error(&save_entry);

    if !result.success {
        return Err(match result.into_error() {
            crate::DataStoreError::AlreadyExists => (StatusCode::CONFLICT, "entity already exists"),
            _ => (StatusCode::INTERNAL_SERVER_ERROR, "failed to create entity"),
        });
    }

    let response = CreateEntityResponse {
        entity,
        created: true,
    };

    Ok(Json(response))
}

/// Deletes an entity by its base64 identifier.
///
/// # Arguments
/// * `entity_base64` - The base64 part of the entity ID (without "entity:" prefix)
///
/// # Returns
/// * `Ok(StatusCode::NO_CONTENT)` - Entity was found and deleted
/// * `Err(StatusCode::BAD_REQUEST)` - Invalid entity ID format
///
/// # Note
/// Currently this is a mock implementation that only validates the entity format
/// but doesn't perform actual deletion from a data store.
async fn delete_entity(
    State((logger, data_store)): State<(Arc<SavefileManager>, Arc<dyn DataStore>)>,
    Path(entity_base64): Path<String>,
) -> Result<StatusCode, (StatusCode, &'static str)> {
    // Construct full entity string from base64 part
    let entity_string = format!("{}{}", ENTITY_PREFIX, entity_base64);

    // Parse the entity ID
    match Entity::from_str(&entity_string) {
        Ok(_entity) => {
            // Attempt to delete from data store using the standardized operation
            let result = DataStoreOperations::delete_entity(&*data_store, &entity_string);
            let success = result.success && result.data.unwrap_or(false);

            let save_entry = SaveEntry::new(
                SaveOperation::EntityDelete {
                    entity_id: entity_string,
                    success,
                },
                SaveMetadata::rest_api(None).with_status(if success {
                    OperationStatus::Success
                } else {
                    OperationStatus::Failed
                }),
            );
            logger.save_or_error(&save_entry);

            if success {
                Ok(StatusCode::NO_CONTENT)
            } else {
                Err((StatusCode::NOT_FOUND, "entity not found"))
            }
        }
        Err(_parse_error) => {
            // Invalid entity ID format - log the specific error for debugging
            let save_entry = SaveEntry::new(
                SaveOperation::EntityDelete {
                    entity_id: entity_string,
                    success: false,
                },
                SaveMetadata::rest_api(None).with_status(OperationStatus::Failed),
            );
            logger.save_or_error(&save_entry);
            Err((StatusCode::BAD_REQUEST, "invalid entity id"))
        }
    }
}

/// Lists all entities stored in the data store.
///
/// # Arguments
/// * `State((logger, data_store))` - The application state containing logger and data store
///
/// # Returns
/// * `Ok(Json<Vec<Entity>>)` - List of all entities on success
/// * `Err(StatusCode::INTERNAL_SERVER_ERROR)` - If data store operation fails
async fn list_entities(
    State((_logger, data_store)): State<(Arc<SavefileManager>, Arc<dyn DataStore>)>,
) -> Result<Json<Vec<Entity>>, (StatusCode, &'static str)> {
    let entities = match data_store.list_entities() {
        Ok(entities) => entities,
        Err(_) => {
            return Err((StatusCode::INTERNAL_SERVER_ERROR, "failed to list entities"));
        }
    };

    Ok(Json(entities))
}

////////////////////////////////////////////// Router //////////////////////////////////////////////////

/// Creates an Axum router with entity management endpoints.
///
/// # Arguments
/// * `logger` - The durable logger instance to use for logging operations
/// * `data_store` - The data store implementation to use for persistence
///
/// # Routes
/// - `GET /entity` - List all entities
/// - `POST /entity` - Create a new entity (optionally random)
/// - `DELETE /entity/{entity_id}` - Delete an entity by ID
///
/// # Returns
/// An Axum `Router` configured with the entity endpoints and state.
///
/// # Examples
/// ```
/// # use stigmergy::{create_entity_router, SavefileManager, InMemoryDataStore};
/// # use std::sync::Arc;
/// # use std::path::PathBuf;
/// let logger = Arc::new(SavefileManager::new(PathBuf::from("test.jsonl")));
/// let data_store = Arc::new(InMemoryDataStore::new());
/// let router = create_entity_router(logger, data_store);
/// ```
pub fn create_entity_router(
    logger: Arc<SavefileManager>,
    data_store: Arc<dyn DataStore>,
) -> Router {
    Router::new()
        .route("/entity", get(list_entities).post(create_entity))
        .route("/entity/:entity_id", delete(delete_entity))
        .with_state((logger, data_store))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_helpers::{
        clear_savefile, create_test_savefile_manager_with_path, load_entries, test_data_store,
    };
    use axum::extract::State;
    use std::path::PathBuf;

    fn test_savefile_manager() -> (Arc<SavefileManager>, PathBuf) {
        use std::process;
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let test_path = PathBuf::from(format!("test_entity_{}_{}.jsonl", process::id(), timestamp));
        let logger = Arc::new(SavefileManager::new(test_path.clone()));
        (logger, test_path)
    }

    #[test]
    fn entity_new_and_accessors() {
        let bytes = [1u8; 32];
        let entity = Entity::new(bytes);
        assert_eq!(entity.as_bytes(), &bytes);
        assert_eq!(entity.into_bytes(), bytes);
    }

    #[test]
    fn base64_encode_decode_round_trip() {
        let input = b"hello world test data for base64";
        let encoded = encode_base64_url_safe(input);
        let decoded = decode_base64_url_safe(&encoded).unwrap();
        assert_eq!(decoded, input);
    }

    #[test]
    fn base64_encode_32_bytes() {
        let input = [0u8; 32];
        let encoded = encode_base64_url_safe(&input);
        assert_eq!(encoded.len(), 43); // 32 bytes -> 43 base64 chars (no padding)
        assert_eq!(encoded, "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA");
    }

    #[test]
    fn base64_decode_no_padding() {
        let input = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
        let decoded = decode_base64_url_safe(input).unwrap();
        assert_eq!(decoded.len(), 32);
        assert_eq!(decoded, vec![0u8; 32]);
    }

    #[test]
    fn entity_display_format() {
        let entity = Entity::new([0u8; 32]);
        let display = format!("{}", entity);
        assert!(display.starts_with("entity:"));
        assert_eq!(display.len(), ENTITY_PREFIX_LEN + BASE64_ENCODED_LEN);
        assert_eq!(
            display,
            "entity:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
        );
    }

    #[test]
    fn entity_display_non_zero() {
        let mut bytes = [0u8; 32];
        bytes[0] = 0xFF;
        bytes[31] = 0x42;
        let entity = Entity::new(bytes);
        let display = format!("{}", entity);
        assert!(display.starts_with("entity:"));
        assert_eq!(display.len(), ENTITY_PREFIX_LEN + BASE64_ENCODED_LEN);
    }

    #[test]
    fn entity_from_str_valid() {
        let entity_str = "entity:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
        let entity = Entity::from_str(entity_str).unwrap();
        assert_eq!(entity.as_bytes(), &[0u8; 32]);
    }

    #[test]
    fn entity_from_str_invalid_prefix() {
        let result = Entity::from_str("invalid:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA");
        assert_eq!(result, Err(EntityParseError::InvalidPrefix));
    }

    #[test]
    fn entity_from_str_invalid_length() {
        let result = Entity::from_str("entity:ABC");
        assert_eq!(result, Err(EntityParseError::InvalidFormat));
    }

    #[test]
    fn entity_from_str_invalid_base64() {
        // Test with correct length but invalid base64 characters
        let result = Entity::from_str("entity:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA!AAAAAAAA");
        assert_eq!(result, Err(EntityParseError::InvalidBase64));
    }

    #[test]
    fn entity_from_str_wrong_length_triggers_format_error() {
        // Test that wrong length triggers format error before base64 validation
        let result = Entity::from_str("entity:SHORTSTRING");
        assert_eq!(result, Err(EntityParseError::InvalidFormat));
    }

    #[test]
    fn entity_round_trip_display_fromstr() {
        let original_bytes = [
            0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66,
            0x77, 0x88, 0x99, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0x00, 0x01, 0x02, 0x03, 0x04,
            0x05, 0x06, 0x07, 0x08,
        ];
        let entity = Entity::new(original_bytes);

        let display_str = format!("{}", entity);

        let parsed_entity = Entity::from_str(&display_str).unwrap();
        assert_eq!(parsed_entity.as_bytes(), &original_bytes);
        assert_eq!(entity, parsed_entity);
    }

    #[test]
    fn entity_display_matches_expected_format() {
        let entity = Entity::new([0xFF; 32]);
        let display = format!("{}", entity);

        // Verify it matches the expected regex pattern entity:[0-9A-Za-z_-]{43}
        assert!(display.starts_with(ENTITY_PREFIX));
        let base64_part = &display[7..];
        assert_eq!(base64_part.len(), BASE64_ENCODED_LEN);

        // Check that all characters are valid URL-safe base64 chars
        for c in base64_part.chars() {
            assert!(c.is_ascii_alphanumeric() || c == '-' || c == '_');
        }
    }

    #[test]
    fn multiple_entity_round_trips() {
        for i in 0..=255u8 {
            let mut bytes = [0u8; 32];
            bytes[0] = i;
            bytes[31] = 255 - i;

            let entity = Entity::new(bytes);
            let display_str = format!("{}", entity);
            let parsed_entity = Entity::from_str(&display_str).unwrap();

            assert_eq!(entity, parsed_entity);
        }
    }

    #[test]
    fn url_safe_characters_used() {
        // Test that URL-safe characters are used instead of standard base64
        let mut bytes = [0u8; 32];
        bytes[0] = 0xFC; // Should produce '+' in standard base64, '-' in URL-safe
        bytes[1] = 0xFF; // Should produce '/' in standard base64, '_' in URL-safe

        let entity = Entity::new(bytes);
        let display = format!("{}", entity);

        // Should contain URL-safe characters
        assert!(display.contains('-') || display.contains('_'));
        // Should not contain standard base64 special characters
        assert!(!display.contains('+'));
        assert!(!display.contains('/'));
    }

    #[tokio::test]
    async fn create_entity_with_provided_entity() {
        let test_entity = Entity::new([1u8; 32]);
        let request = CreateEntityRequest {
            entity: Some(test_entity),
        };

        let (logger, log_path) = test_savefile_manager();
        let data_store = test_data_store();
        let response = create_entity(State((logger, data_store)), Json(request))
            .await
            .unwrap();

        assert_eq!(response.0.entity, test_entity);
        assert!(response.0.created);
        clear_savefile(&log_path);
    }

    #[tokio::test]
    async fn create_entity_generates_random_when_none() {
        let request = CreateEntityRequest { entity: None };

        let (logger, log_path) = test_savefile_manager();
        let data_store = test_data_store();
        let response = create_entity(State((logger, data_store)), Json(request)).await;

        // Must succeed - if /dev/urandom is not available, fail the test
        let response =
            response.expect("/dev/urandom should be available for random entity generation");
        assert!(response.0.created);
        // The entity should be randomly generated (not all zeros)
        assert_ne!(response.0.entity, Entity::new([0u8; 32]));
        clear_savefile(&log_path);
    }

    #[tokio::test]
    async fn create_entity_avoids_special_characters() {
        // Generate multiple random entities and verify none contain - or _
        let (logger, log_path) = test_savefile_manager();
        let data_store = test_data_store();

        for _ in 0..1000 {
            let request = CreateEntityRequest { entity: None };

            let response =
                create_entity(State((logger.clone(), data_store.clone())), Json(request)).await;

            // Must succeed - if /dev/urandom is not available, fail the test
            let response =
                response.expect("/dev/urandom should be available for random entity generation");

            let entity_string = response.0.entity.to_string();
            let base64_part = &entity_string[ENTITY_PREFIX_LEN..]; // Skip "entity:"

            // Ensure no - or _ characters in the base64 part
            assert!(
                !base64_part.contains('-'),
                "Generated entity ID contains '-': {}",
                entity_string
            );
            assert!(
                !base64_part.contains('_'),
                "Generated entity ID contains '_': {}",
                entity_string
            );

            // Should only contain alphanumeric characters
            for c in base64_part.chars() {
                assert!(
                    c.is_ascii_alphanumeric(),
                    "Generated entity ID contains non-alphanumeric character '{}': {}",
                    c,
                    entity_string
                );
            }
        }
        clear_savefile(&log_path);
    }

    #[tokio::test]
    async fn delete_entity_valid_id() {
        let entity = Entity::new([1u8; 32]);
        let entity_str = entity.to_string();
        // Extract just the base64 part after "entity:"
        let base64_part = entity_str.strip_prefix(ENTITY_PREFIX).unwrap();

        let (logger, log_path) = test_savefile_manager();
        let data_store = test_data_store();

        // First create the entity in the data store
        data_store.create_entity(&entity).unwrap();

        let result =
            delete_entity(State((logger, data_store)), Path(base64_part.to_string())).await;

        assert_eq!(result, Ok(StatusCode::NO_CONTENT));
        clear_savefile(&log_path);
    }

    #[tokio::test]
    async fn delete_entity_invalid_id() {
        let invalid_base64 = "invalid_entity_id".to_string();

        let (logger, log_path) = test_savefile_manager();
        let data_store = test_data_store();
        let result = delete_entity(
            State((logger, data_store)),
            Path(invalid_base64.to_string()),
        )
        .await;

        assert_eq!(result, Err((StatusCode::BAD_REQUEST, "invalid entity id")));
        clear_savefile(&log_path);
    }

    #[tokio::test]
    async fn create_entity_success_logs_correctly() {
        let (logger, log_path) = create_test_savefile_manager_with_path("entity", "create_success");
        clear_savefile(&log_path);

        let test_entity = Entity::new([42u8; 32]);
        let request = CreateEntityRequest {
            entity: Some(test_entity),
        };

        // Check log file before operation (should be empty)
        let logs_before = load_entries(&log_path);
        assert!(
            logs_before.is_empty(),
            "Log file should be empty before operation"
        );

        // Execute HTTP operation
        let data_store = test_data_store();
        let response = create_entity(State((logger, data_store)), Json(request)).await;
        assert!(response.is_ok(), "Create entity should succeed");
        let response = response.unwrap();
        assert_eq!(response.0.entity, test_entity);
        assert!(response.0.created);

        // Check log file after operation
        let logs_after = load_entries(&log_path);
        assert_eq!(logs_after.len(), 1, "Should have exactly one log entry");

        let save_entry = &logs_after[0];
        assert_eq!(save_entry.operation_type(), "EntityCreate");
        assert!(save_entry.is_success());
        assert_eq!(save_entry.entity_id(), Some(test_entity.to_string()));

        // Validate JSON structure and content
        match &save_entry.operation {
            SaveOperation::EntityCreate { entity, was_random } => {
                assert_eq!(*entity, test_entity);
                assert!(!was_random, "Should not be random when entity is provided");
            }
            _ => panic!("Expected EntityCreate operation"),
        }

        assert_eq!(save_entry.metadata.source, "REST API");
        assert!(matches!(
            save_entry.metadata.status,
            OperationStatus::Success
        ));

        // Cleanup
        clear_savefile(&log_path);
    }

    #[tokio::test]
    async fn create_entity_random_success_logs_correctly() {
        let (logger, log_path) = create_test_savefile_manager_with_path("entity", "create_random");
        clear_savefile(&log_path);

        let request = CreateEntityRequest { entity: None };

        // Check log file before operation
        let logs_before = load_entries(&log_path);
        assert!(logs_before.is_empty());

        // Execute HTTP operation
        let data_store = test_data_store();
        let response = create_entity(State((logger, data_store)), Json(request)).await;

        // This test requires /dev/urandom to be available
        if response.is_err() {
            println!("Skipping random entity test - /dev/urandom not available");
            clear_savefile(&log_path);
            return;
        }

        let response = response.unwrap();
        assert!(response.0.created);

        // Check log file after operation
        let logs_after = load_entries(&log_path);
        assert_eq!(logs_after.len(), 1, "Should have exactly one log entry");

        let save_entry = &logs_after[0];
        assert_eq!(save_entry.operation_type(), "EntityCreate");
        assert!(save_entry.is_success());

        // Validate JSON structure and content
        match &save_entry.operation {
            SaveOperation::EntityCreate {
                entity: _,
                was_random,
            } => {
                assert!(
                    *was_random,
                    "Should be marked as random when no entity provided"
                );
            }
            _ => panic!("Expected EntityCreate operation"),
        }

        // Cleanup
        clear_savefile(&log_path);
    }

    #[tokio::test]
    async fn delete_entity_success_logs_correctly() {
        let (logger, log_path) = create_test_savefile_manager_with_path("entity", "delete_success");
        clear_savefile(&log_path);

        let entity = Entity::new([1u8; 32]);
        let entity_str = entity.to_string();
        let base64_part = entity_str.strip_prefix(ENTITY_PREFIX).unwrap();

        // Check log file before operation
        let logs_before = load_entries(&log_path);
        assert!(logs_before.is_empty());

        // Execute HTTP operation
        let data_store = test_data_store();

        // First create the entity in the data store
        data_store.create_entity(&entity).unwrap();

        let result =
            delete_entity(State((logger, data_store)), Path(base64_part.to_string())).await;
        assert_eq!(result, Ok(StatusCode::NO_CONTENT));

        // Check log file after operation
        let logs_after = load_entries(&log_path);
        assert_eq!(logs_after.len(), 1, "Should have exactly one log entry");

        let save_entry = &logs_after[0];
        assert_eq!(save_entry.operation_type(), "EntityDelete");
        assert!(save_entry.is_success());
        assert_eq!(save_entry.entity_id(), Some(entity_str.clone()));

        // Validate JSON structure and content
        match &save_entry.operation {
            SaveOperation::EntityDelete { entity_id, success } => {
                assert_eq!(*entity_id, entity_str);
                assert!(*success, "Delete should be successful");
            }
            _ => panic!("Expected EntityDelete operation"),
        }

        assert_eq!(save_entry.metadata.source, "REST API");
        assert!(matches!(
            save_entry.metadata.status,
            OperationStatus::Success
        ));

        // Cleanup
        clear_savefile(&log_path);
    }

    #[tokio::test]
    async fn delete_entity_failure_logs_correctly() {
        let (logger, log_path) = create_test_savefile_manager_with_path("entity", "delete_failure");
        clear_savefile(&log_path);

        let invalid_base64 = "invalid_entity_id";

        // Check log file before operation
        let logs_before = load_entries(&log_path);
        assert!(logs_before.is_empty());

        // Execute HTTP operation
        let data_store = test_data_store();
        let result = delete_entity(
            State((logger, data_store)),
            Path(invalid_base64.to_string()),
        )
        .await;
        assert_eq!(result, Err((StatusCode::BAD_REQUEST, "invalid entity id")));

        // Check log file after operation
        let logs_after = load_entries(&log_path);
        assert_eq!(logs_after.len(), 1, "Should have exactly one log entry");

        let save_entry = &logs_after[0];
        assert_eq!(save_entry.operation_type(), "EntityDelete");
        assert!(save_entry.is_failure());

        // Validate JSON structure and content
        match &save_entry.operation {
            SaveOperation::EntityDelete { entity_id, success } => {
                assert_eq!(*entity_id, format!("{}{}", ENTITY_PREFIX, invalid_base64));
                assert!(!success, "Delete should be unsuccessful for invalid entity");
            }
            _ => panic!("Expected EntityDelete operation"),
        }

        assert_eq!(save_entry.metadata.source, "REST API");
        assert!(matches!(
            save_entry.metadata.status,
            OperationStatus::Failed
        ));

        // Cleanup
        clear_savefile(&log_path);
    }
}
