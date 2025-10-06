//! # Entity Management System
//!
//! This module provides the core entity management capabilities for the stigmergy system.
//! Entities serve as unique identifiers that can have components attached to them, following
//! Entity-Component-System (ECS) architectural patterns.
//!
//! ## Key Features
//!
//! - **URL-Safe Identifiers**: Entities use URL-safe base64 encoding with custom alphabet
//! - **Deterministic Parsing**: String representation is deterministic and reversible
//! - **Random Generation**: Support for cryptographically random entity generation
//! - **HTTP Integration**: Built-in HTTP endpoints for entity lifecycle management
//! - **Audit Logging**: All operations are logged for persistence and replay
//!
//! ## Entity Format
//!
//! Entities are represented as "entity:{base64}" where the base64 portion is a URL-safe
//! encoding of 32 bytes without padding. This format is designed to be:
//! - Safe for use in URLs and file paths
//! - Human-readable for debugging
//! - Deterministic for consistent serialization
//!
//! ## Usage Examples
//!
//! ```rust
//! use stigmergy::Entity;
//!
//! // Create entity from bytes
//! let entity = Entity::new([1u8; 32]);
//! println!("{}", entity); // "entity:AQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQE"
//!
//! // Parse from string
//! let parsed: Entity = "entity:AQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQE".parse().unwrap();
//! assert_eq!(entity, parsed);
//!
//! // Access underlying bytes
//! let bytes = entity.as_bytes();
//! assert_eq!(bytes, &[1u8; 32]);
//! ```

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

    /// Returns the base64 portion of the entity identifier for URL construction.
    ///
    /// This method extracts just the base64-encoded part without the "entity:" prefix,
    /// which is useful for constructing API URLs that expect only the identifier part.
    ///
    /// # Examples
    /// ```
    /// # use stigmergy::Entity;
    /// let entity = Entity::new([1u8; 32]);
    /// let base64_part = entity.base64_part();
    /// assert_eq!(base64_part.len(), 43); // Base64 encoding of 32 bytes is 43 chars
    /// assert!(!base64_part.contains("entity:"));
    /// ```
    pub fn base64_part(&self) -> String {
        encode_base64_url_safe(&self.0)
    }
}

////////////////////////////////////// URL-Safe Base64 Encoding //////////////////////////////////////

/// URL-safe base64 character set (RFC 4648 Section 5)
/// Uses - and _ instead of + and / to be safe in URLs
const BASE64_CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

/// Encodes bytes to URL-safe base64 without padding.
///
/// This implementation uses the URL-safe alphabet (RFC 4648 Section 5) and
/// omits padding characters for a cleaner representation. The encoding process
/// converts each group of 3 bytes into 4 base64 characters, handling partial
/// groups at the end without adding padding.
///
/// # Arguments
/// * `input` - The bytes to encode
///
/// # Returns
/// A URL-safe base64 encoded string without padding
///
/// # Examples
/// ```
/// # use stigmergy::Entity;
/// let entity = Entity::new([0u8; 32]);
/// let encoded = entity.to_string();
/// assert_eq!(encoded, "entity:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA");
/// ```
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
///
/// This enum represents the different ways that entity string parsing can fail,
/// providing specific error types for different categories of parsing problems.
#[derive(Debug, PartialEq, Eq)]
pub enum EntityParseError {
    /// The entity string does not start with the required "entity:" prefix
    InvalidPrefix,
    /// The entity string format is invalid (e.g., wrong length after prefix)
    InvalidFormat,
    /// The base64 portion contains invalid characters or is malformed
    InvalidBase64,
    /// The decoded bytes are not exactly 32 bytes in length
    InvalidLength,
}

impl Display for EntityParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            EntityParseError::InvalidPrefix => write!(f, "Invalid entity prefix"),
            EntityParseError::InvalidFormat => write!(
                f,
                "Invalid entity format - expected 43-character base64 string"
            ),
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
    /// Accepts either format:
    /// - "entity:{base64}" - full format with prefix
    /// - "{base64}" - base64 only (43 characters of URL-safe base64)
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
    /// let entity1: Entity = "entity:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA".parse().unwrap();
    /// let entity2: Entity = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA".parse().unwrap();
    /// assert_eq!(entity1, entity2);
    /// ```
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let base64_part = if let Some(base64) = s.strip_prefix(ENTITY_PREFIX) {
            // Has "entity:" prefix, use the part after it
            base64
        } else if s.contains(':') {
            // Has some other prefix - this is invalid
            return Err(EntityParseError::InvalidPrefix);
        } else {
            // No prefix, assume it's already the base64 part
            s
        };

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

/// HTTP endpoint for creating a new entity.
///
/// This endpoint accepts a POST request with an optional entity ID. If no entity
/// is provided in the request, generates a random entity that avoids URL-unsafe
/// characters (-_) in its base64 representation using a bounded retry approach
/// to prevent infinite loops.
///
/// # Request Format
/// ```json
/// {
///   "entity": null  // Optional: specific entity to create, or null for random
/// }
/// ```
///
/// # Response Format
/// ```json
/// {
///   "entity": "entity:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
///   "created": true
/// }
/// ```
///
/// # Errors
/// Returns `StatusCode::INTERNAL_SERVER_ERROR` if random number generation fails.
/// Returns `StatusCode::CONFLICT` if the entity already exists in the data store.
async fn create_entity(
    State(pool): State<sqlx::PgPool>,
    Json(request): Json<CreateEntityRequest>,
) -> Result<Json<CreateEntityResponse>, (StatusCode, &'static str)> {
    let entity = match request.entity {
        Some(entity) => entity,
        None => Entity::random_url_safe().map_err(|_e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to generate random entity",
            )
        })?,
    };

    let mut tx = pool.begin().await.map_err(|_e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to begin transaction",
        )
    })?;

    match crate::sql::entity::create(&mut tx, &entity).await {
        Ok(()) => {
            tx.commit().await.map_err(|_e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "failed to commit transaction",
                )
            })?;
            let response = CreateEntityResponse {
                entity,
                created: true,
            };
            Ok(Json(response))
        }
        Err(crate::DataStoreError::AlreadyExists) => {
            Err((StatusCode::CONFLICT, "entity already exists"))
        }
        Err(_) => Err((StatusCode::INTERNAL_SERVER_ERROR, "failed to create entity")),
    }
}

/// HTTP endpoint for deleting an entity by its base64 identifier.
///
/// This endpoint accepts a DELETE request with the entity's base64 identifier
/// (without the "entity:" prefix) in the URL path. The entity is removed from
/// the data store and all associated components are cascade deleted.
///
/// # URL Parameters
/// * `entity_base64` - The base64 part of the entity ID (without "entity:" prefix)
///
/// # Returns
/// * `StatusCode::NO_CONTENT` - Entity was found and successfully deleted
/// * `StatusCode::BAD_REQUEST` - Invalid entity ID format
/// * `StatusCode::NOT_FOUND` - Entity does not exist in the data store
///
/// # Examples
/// ```
/// // DELETE /entity/AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA
/// // -> 204 No Content (if entity exists)
/// // -> 400 Bad Request (if malformed)
/// // -> 404 Not Found (if entity doesn't exist)
/// ```
async fn delete_entity(
    State(pool): State<sqlx::PgPool>,
    Path(entity_base64): Path<String>,
) -> Result<StatusCode, (StatusCode, &'static str)> {
    let entity_string = format!("{}{}", ENTITY_PREFIX, entity_base64);

    let entity = Entity::from_str(&entity_string)
        .map_err(|_parse_error| (StatusCode::BAD_REQUEST, "invalid entity id"))?;

    let mut tx = pool.begin().await.map_err(|_e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to begin transaction",
        )
    })?;

    match crate::sql::entity::delete(&mut tx, &entity).await {
        Ok(true) => {
            tx.commit().await.map_err(|_e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "failed to commit transaction",
                )
            })?;
            Ok(StatusCode::NO_CONTENT)
        }
        Ok(false) => Err((StatusCode::NOT_FOUND, "entity not found")),
        Err(_) => Err((StatusCode::INTERNAL_SERVER_ERROR, "failed to delete entity")),
    }
}

/// HTTP endpoint for listing all entities stored in the data store.
///
/// This endpoint returns a JSON array of all entities currently stored in the
/// system. Each entity is represented in its full "entity:{base64}" format.
///
/// # Returns
/// * `Ok(Json<Vec<Entity>>)` - JSON array of all entities on success
/// * `Err(StatusCode::INTERNAL_SERVER_ERROR)` - If data store operation fails
///
/// # Response Format
/// ```json
/// [
///   "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
///   "BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB"
/// ]
/// ```
///
/// # Examples
/// ```
/// // GET /entity
/// // -> 200 OK with array of entity base64 strings
/// ```
async fn list_entities(
    State(pool): State<sqlx::PgPool>,
) -> Result<Json<Vec<Entity>>, (StatusCode, &'static str)> {
    let mut tx = pool.begin().await.map_err(|_e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to begin transaction",
        )
    })?;

    match crate::sql::entity::list(&mut tx).await {
        Ok(entities) => {
            tx.commit().await.map_err(|_e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "failed to commit transaction",
                )
            })?;
            Ok(Json(entities))
        }
        Err(_) => Err((StatusCode::INTERNAL_SERVER_ERROR, "failed to list entities")),
    }
}

////////////////////////////////////////////// Router //////////////////////////////////////////////////

/// Creates an Axum router with entity management endpoints.
///
/// # Arguments
/// * `pool` - PostgreSQL connection pool for entity operations
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
/// ```no_run
/// # use stigmergy::create_entity_router;
/// # use sqlx::PgPool;
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// # let database_url = "postgres://localhost/stigmergy";
/// let pool = PgPool::connect(database_url).await?;
/// let router = create_entity_router(pool);
/// # Ok(())
/// # }
/// ```
pub fn create_entity_router(pool: sqlx::PgPool) -> Router {
    Router::new()
        .route("/entity", get(list_entities).post(create_entity))
        .route("/entity/:entity_id", delete(delete_entity))
        .with_state(pool)
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn entity_from_str_without_prefix() {
        let base64_str = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
        let entity = Entity::from_str(base64_str).unwrap();
        assert_eq!(entity.as_bytes(), &[0u8; 32]);
    }

    #[test]
    fn entity_from_str_with_and_without_prefix_equivalent() {
        let base64_str = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
        let with_prefix_str = "entity:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";

        let entity1 = Entity::from_str(base64_str).unwrap();
        let entity2 = Entity::from_str(with_prefix_str).unwrap();

        assert_eq!(entity1, entity2);
        assert_eq!(entity1.as_bytes(), entity2.as_bytes());
    }

    #[test]
    fn entity_base64_part_method() {
        let entity = Entity::new([1u8; 32]);
        let base64_part = entity.base64_part();

        // Should be 43 characters (base64 encoding of 32 bytes without padding)
        assert_eq!(base64_part.len(), 43);

        // Should not contain the prefix
        assert!(!base64_part.contains("entity:"));

        // Should be able to parse back using the base64 part only
        let parsed = Entity::from_str(&base64_part).unwrap();
        assert_eq!(parsed, entity);
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

    fn unique_entity(test_name: &str) -> Entity {
        use std::time::{SystemTime, UNIX_EPOCH};
        let pid = std::process::id();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_micros() as u64;

        let mut bytes = [0u8; 32];
        bytes[0..4].copy_from_slice(&pid.to_le_bytes());
        bytes[4..12].copy_from_slice(&now.to_le_bytes());

        let test_bytes = test_name.as_bytes();
        let copy_len = test_bytes.len().min(20);
        bytes[12..12 + copy_len].copy_from_slice(&test_bytes[..copy_len]);

        Entity::new(bytes)
    }

    #[tokio::test]
    async fn create_entity_with_provided_entity() {
        let pool = crate::sql::tests::setup_test_db().await;
        let entity = unique_entity("create_entity_with_provided_entity");

        let request = CreateEntityRequest {
            entity: Some(entity),
        };

        let result = create_entity(State(pool.clone()), Json(request)).await;
        assert!(result.is_ok());

        let response = result.unwrap().0;
        assert_eq!(response.entity, entity);
        assert!(response.created);

        let mut tx = pool.begin().await.unwrap();
        let stored = crate::sql::entity::get(&mut tx, &entity).await.unwrap();
        tx.commit().await.unwrap();
        assert!(stored.is_some());
    }

    #[tokio::test]
    async fn create_entity_generates_random_when_none() {
        let pool = crate::sql::tests::setup_test_db().await;

        let request = CreateEntityRequest { entity: None };

        let result = create_entity(State(pool.clone()), Json(request)).await;
        assert!(result.is_ok());

        let response = result.unwrap().0;
        assert!(response.created);

        let mut tx = pool.begin().await.unwrap();
        let stored = crate::sql::entity::get(&mut tx, &response.entity)
            .await
            .unwrap();
        tx.commit().await.unwrap();
        assert!(stored.is_some());
    }

    #[tokio::test]
    async fn create_entity_avoids_special_characters() {
        let pool = crate::sql::tests::setup_test_db().await;

        for _ in 0..10 {
            let request = CreateEntityRequest { entity: None };

            let result = create_entity(State(pool.clone()), Json(request)).await;
            assert!(result.is_ok());

            let response = result.unwrap().0;
            let entity_str = response.entity.to_string();
            let base64_part = &entity_str[ENTITY_PREFIX_LEN..];

            assert!(
                !base64_part.contains('-') && !base64_part.contains('_'),
                "Generated entity contains special characters: {}",
                base64_part
            );
        }
    }

    #[tokio::test]
    async fn delete_entity_valid_id() {
        let pool = crate::sql::tests::setup_test_db().await;
        let entity = unique_entity("delete_entity_valid_id");

        let mut tx = pool.begin().await.unwrap();
        crate::sql::entity::create(&mut tx, &entity).await.unwrap();
        tx.commit().await.unwrap();

        let base64_part = entity.base64_part();
        let result = delete_entity(State(pool.clone()), Path(base64_part)).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), StatusCode::NO_CONTENT);

        let mut tx = pool.begin().await.unwrap();
        let stored = crate::sql::entity::get(&mut tx, &entity).await.unwrap();
        tx.commit().await.unwrap();
        assert!(stored.is_none());
    }

    #[tokio::test]
    async fn delete_entity_invalid_id() {
        let pool = crate::sql::tests::setup_test_db().await;

        let result = delete_entity(State(pool.clone()), Path("invalid-id".to_string())).await;

        assert!(result.is_err());
        let (status, _message) = result.unwrap_err();
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn create_entity_duplicate_fails() {
        let pool = crate::sql::tests::setup_test_db().await;
        let entity = unique_entity("create_entity_duplicate_fails");

        let request1 = CreateEntityRequest {
            entity: Some(entity),
        };

        let result1 = create_entity(State(pool.clone()), Json(request1)).await;
        assert!(result1.is_ok());

        let request2 = CreateEntityRequest {
            entity: Some(entity),
        };

        let result2 = create_entity(State(pool.clone()), Json(request2)).await;
        assert!(result2.is_err());
        let (status, _message) = result2.unwrap_err();
        assert_eq!(status, StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn delete_entity_nonexistent() {
        let pool = crate::sql::tests::setup_test_db().await;
        let entity = unique_entity("delete_entity_nonexistent");

        let base64_part = entity.base64_part();
        let result = delete_entity(State(pool.clone()), Path(base64_part)).await;

        assert!(result.is_err());
        let (status, _message) = result.unwrap_err();
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn list_entities_includes_created() {
        let pool = crate::sql::tests::setup_test_db().await;
        let entity = unique_entity("list_entities_includes_created");

        let mut tx = pool.begin().await.unwrap();
        crate::sql::entity::create(&mut tx, &entity).await.unwrap();
        tx.commit().await.unwrap();

        let result = list_entities(State(pool.clone())).await;
        assert!(result.is_ok());

        let entities = result.unwrap().0;
        assert!(entities.contains(&entity));
    }
}
