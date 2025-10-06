//! # Invariant Management System
//!
//! This module provides the core invariant management capabilities for the stigmergy system.
//! Invariants represent conditions that must always be true within the system, enforcing
//! data integrity and system correctness.
//!
//! ## Key Features
//!
//! - **URL-Safe Identifiers**: Invariants use URL-safe base64 encoding with custom alphabet
//! - **Deterministic Parsing**: String representation is deterministic and reversible
//! - **Random Generation**: Support for cryptographically random invariant generation
//! - **Assertion Storage**: Each invariant stores its assertion condition
//!
//! ## InvariantID Format
//!
//! Invariants are represented as "invariant:{base64}" where the base64 portion is a URL-safe
//! encoding of 32 bytes without padding. This format is designed to be:
//! - Safe for use in URLs and file paths
//! - Human-readable for debugging
//! - Deterministic for consistent serialization
//!
//! ## Usage Examples
//!
//! ```rust
//! use stigmergy::InvariantID;
//!
//! // Create invariant from bytes
//! let invariant = InvariantID::new([1u8; 32]);
//! println!("{}", invariant); // "invariant:AQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQE"
//!
//! // Parse from string
//! let parsed: InvariantID = "invariant:AQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQE".parse().unwrap();
//! assert_eq!(invariant, parsed);
//!
//! // Access underlying bytes
//! let bytes = invariant.as_bytes();
//! assert_eq!(bytes, &[1u8; 32]);
//! ```

use std::fmt::{Display, Formatter, Result as FmtResult};
use std::fs::File;
use std::io::Read;
use std::str::FromStr;

use axum::Router;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::Json;
use axum::routing::get;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

////////////////////////////////////////////// Constants ///////////////////////////////////////////////

/// Length of the invariant prefix "invariant:"
const INVARIANT_PREFIX: &str = "invariant:";
const INVARIANT_PREFIX_LEN: usize = 10;

/// Expected length of base64 encoded 32 bytes (without padding)
const BASE64_ENCODED_LEN: usize = 43;

/// Maximum number of retries when generating invariants without special characters
const MAX_GENERATION_RETRIES: usize = 1000;

///////////////////////////////////////////// InvariantID //////////////////////////////////////////////

/// A 32-byte invariant identifier with URL-safe base64 string representation.
///
/// Invariants are displayed as "invariant:{base64}" where the base64 encoding uses URL-safe
/// characters (- and _ instead of + and /) and no padding. This format is suitable for
/// use in URLs and other contexts where standard base64 characters might cause issues.
///
/// # Examples
///
/// ```
/// # use stigmergy::InvariantID;
/// let invariant = InvariantID::new([1u8; 32]);
/// let invariant_string = invariant.to_string();
/// assert!(invariant_string.starts_with("invariant:"));
///
/// // Parse from string
/// let parsed: InvariantID = invariant_string.parse().unwrap();
/// assert_eq!(invariant, parsed);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct InvariantID([u8; 32]);

impl InvariantID {
    /// Creates a new InvariantID from a 32-byte array.
    ///
    /// # Arguments
    /// * `bytes` - A 32-byte array representing the invariant identifier
    ///
    /// # Examples
    /// ```
    /// # use stigmergy::InvariantID;
    /// let invariant = InvariantID::new([0u8; 32]);
    /// ```
    pub fn new(bytes: [u8; 32]) -> Self {
        InvariantID(bytes)
    }

    /// Returns a reference to the underlying 32-byte array.
    ///
    /// # Examples
    /// ```
    /// # use stigmergy::InvariantID;
    /// let invariant = InvariantID::new([1u8; 32]);
    /// assert_eq!(invariant.as_bytes(), &[1u8; 32]);
    /// ```
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Consumes the InvariantID and returns the underlying 32-byte array.
    ///
    /// # Examples
    /// ```
    /// # use stigmergy::InvariantID;
    /// let invariant = InvariantID::new([1u8; 32]);
    /// let bytes = invariant.into_bytes();
    /// assert_eq!(bytes, [1u8; 32]);
    /// ```
    pub fn into_bytes(self) -> [u8; 32] {
        self.0
    }

    /// Generates a random InvariantID using `/dev/urandom`.
    ///
    /// # Returns
    /// * `Ok(InvariantID)` - A randomly generated invariant on success
    /// * `Err(std::io::Error)` - An error if random number generation fails
    ///
    /// # Examples
    /// ```no_run
    /// # use stigmergy::InvariantID;
    /// let invariant = InvariantID::random().unwrap();
    /// ```
    pub fn random() -> std::io::Result<Self> {
        let mut random_bytes = [0u8; 32];
        let mut file = File::open("/dev/urandom")?;
        file.read_exact(&mut random_bytes)?;
        Ok(InvariantID::new(random_bytes))
    }

    /// Generates a random InvariantID that avoids URL-unsafe characters in its base64 representation.
    ///
    /// This method generates random invariants until it finds one whose base64 encoding
    /// doesn't contain `-` or `_` characters. After MAX_GENERATION_RETRIES attempts,
    /// it will accept any invariant and replace `-` with `9` and `_` with `6` to ensure
    /// URL-safe output.
    ///
    /// # Returns
    /// * `Ok(InvariantID)` - A randomly generated invariant without URL-unsafe characters
    /// * `Err(std::io::Error)` - An error if random number generation fails
    ///
    /// # Examples
    /// ```no_run
    /// # use stigmergy::InvariantID;
    /// let invariant = InvariantID::random_url_safe().unwrap();
    /// let invariant_string = invariant.to_string();
    /// let base64_part = &invariant_string[10..]; // Skip "invariant:"
    /// assert!(!base64_part.contains('-') && !base64_part.contains('_'));
    /// ```
    pub fn random_url_safe() -> std::io::Result<Self> {
        let mut attempts = 0;

        loop {
            attempts += 1;

            let invariant = Self::random()?;
            let invariant_string = invariant.to_string();

            if attempts > MAX_GENERATION_RETRIES {
                let cleaned_invariant_string = invariant_string.replace('-', "9").replace('_', "6");
                return Ok(cleaned_invariant_string.parse().unwrap());
            }

            let base64_part = &invariant_string[INVARIANT_PREFIX_LEN..];
            if !base64_part.contains('-') && !base64_part.contains('_') {
                return Ok(invariant);
            }
        }
    }

    /// Returns the base64 portion of the invariant identifier for URL construction.
    ///
    /// This method extracts just the base64-encoded part without the "invariant:" prefix,
    /// which is useful for constructing API URLs that expect only the identifier part.
    ///
    /// # Examples
    /// ```
    /// # use stigmergy::InvariantID;
    /// let invariant = InvariantID::new([1u8; 32]);
    /// let base64_part = invariant.base64_part();
    /// assert_eq!(base64_part.len(), 43); // Base64 encoding of 32 bytes is 43 chars
    /// assert!(!base64_part.contains("invariant:"));
    /// ```
    pub fn base64_part(&self) -> String {
        encode_base64_url_safe(&self.0)
    }
}

////////////////////////////////////// URL-Safe Base64 Encoding //////////////////////////////////////

/// URL-safe base64 character set (RFC 4648 Section 5)
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
        '=' => Ok(0),
        _ => Err("Invalid base64 character"),
    }
}

/////////////////////////////////////////// Display and FromStr ///////////////////////////////////////

impl Display for InvariantID {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let encoded = encode_base64_url_safe(&self.0);
        write!(f, "{}{}", INVARIANT_PREFIX, encoded)
    }
}

/// Errors that can occur when parsing an InvariantID from a string.
#[derive(Debug, PartialEq, Eq)]
pub enum InvariantIDParseError {
    /// The invariant string does not start with the required "invariant:" prefix
    InvalidPrefix,
    /// The invariant string format is invalid (e.g., wrong length after prefix)
    InvalidFormat,
    /// The base64 portion contains invalid characters or is malformed
    InvalidBase64,
    /// The decoded bytes are not exactly 32 bytes in length
    InvalidLength,
}

impl Display for InvariantIDParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            InvariantIDParseError::InvalidPrefix => write!(f, "Invalid invariant prefix"),
            InvariantIDParseError::InvalidFormat => write!(
                f,
                "Invalid invariant format - expected 43-character base64 string"
            ),
            InvariantIDParseError::InvalidBase64 => write!(f, "Invalid base64 encoding"),
            InvariantIDParseError::InvalidLength => {
                write!(f, "Invariant must be exactly 32 bytes")
            }
        }
    }
}

impl std::error::Error for InvariantIDParseError {}

impl FromStr for InvariantID {
    type Err = InvariantIDParseError;

    /// Parses an InvariantID from its string representation.
    ///
    /// Accepts either format:
    /// - "invariant:{base64}" - full format with prefix
    /// - "{base64}" - base64 only (43 characters of URL-safe base64)
    ///
    /// # Arguments
    /// * `s` - The string to parse
    ///
    /// # Returns
    /// * `Ok(InvariantID)` - The parsed invariant on success
    /// * `Err(InvariantIDParseError)` - The specific parsing error on failure
    ///
    /// # Examples
    /// ```
    /// # use stigmergy::InvariantID;
    /// let invariant1: InvariantID = "invariant:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA".parse().unwrap();
    /// let invariant2: InvariantID = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA".parse().unwrap();
    /// assert_eq!(invariant1, invariant2);
    /// ```
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let base64_part = if let Some(base64) = s.strip_prefix(INVARIANT_PREFIX) {
            base64
        } else if s.contains(':') {
            return Err(InvariantIDParseError::InvalidPrefix);
        } else {
            s
        };

        if base64_part.len() != BASE64_ENCODED_LEN {
            return Err(InvariantIDParseError::InvalidFormat);
        }

        let decoded = decode_base64_url_safe(base64_part)
            .map_err(|_| InvariantIDParseError::InvalidBase64)?;

        if decoded.len() != 32 {
            return Err(InvariantIDParseError::InvalidLength);
        }

        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(&decoded);
        Ok(InvariantID(bytes))
    }
}

impl Serialize for InvariantID {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let encoded = encode_base64_url_safe(&self.0);
        serializer.serialize_str(&encoded)
    }
}

impl<'de> Deserialize<'de> for InvariantID {
    fn deserialize<D>(deserializer: D) -> Result<InvariantID, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(InvariantIDVisitor)
    }
}

struct InvariantIDVisitor;

impl<'de> serde::de::Visitor<'de> for InvariantIDVisitor {
    type Value = InvariantID;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a base64 invariant ID string (43 characters)")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        let base64_part = if value.starts_with(INVARIANT_PREFIX) {
            &value[INVARIANT_PREFIX_LEN..]
        } else {
            value
        };

        if base64_part.len() != BASE64_ENCODED_LEN {
            return Err(serde::de::Error::custom(format!(
                "Invariant base64 must be exactly {} characters, got {}",
                BASE64_ENCODED_LEN,
                base64_part.len()
            )));
        }

        let decoded = decode_base64_url_safe(base64_part)
            .map_err(|e| serde::de::Error::custom(format!("Invalid base64: {}", e)))?;

        if decoded.len() != 32 {
            return Err(serde::de::Error::custom(format!(
                "Decoded invariant must be exactly 32 bytes, got {}",
                decoded.len()
            )));
        }

        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(&decoded);
        Ok(InvariantID(bytes))
    }
}

////////////////////////////////////////// HTTP Request/Response Types ////////////////////////////////////

/// Request structure for creating a new invariant.
#[derive(Debug, Deserialize, Serialize)]
pub struct CreateInvariantRequest {
    /// Optional invariant ID. If not provided, a random one will be generated.
    pub invariant_id: Option<InvariantID>,
    /// The assertion expression as a string.
    pub asserts: String,
}

/// Response structure for invariant creation.
#[derive(Debug, Deserialize, Serialize)]
pub struct CreateInvariantResponse {
    /// The created invariant's ID.
    pub invariant_id: InvariantID,
    /// The assertion expression.
    pub asserts: String,
}

/// Response structure for getting an invariant.
#[derive(Debug, Deserialize, Serialize)]
pub struct GetInvariantResponse {
    /// The invariant's ID.
    pub invariant_id: InvariantID,
    /// The assertion expression.
    pub asserts: String,
    /// When the invariant was created.
    pub created_at: DateTime<Utc>,
    /// When the invariant was last updated.
    pub updated_at: DateTime<Utc>,
}

/// Request structure for updating an invariant.
#[derive(Debug, Deserialize, Serialize)]
pub struct UpdateInvariantRequest {
    /// The new assertion expression.
    pub asserts: String,
}

////////////////////////////////////////////// HTTP Handlers //////////////////////////////////////////////

/// HTTP endpoint for creating a new invariant.
async fn create_invariant(
    State(pool): State<sqlx::PgPool>,
    Json(request): Json<CreateInvariantRequest>,
) -> Result<(StatusCode, Json<CreateInvariantResponse>), (StatusCode, &'static str)> {
    let invariant_id = request
        .invariant_id
        .map(Ok)
        .unwrap_or_else(InvariantID::random_url_safe)
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to generate invariant id",
            )
        })?;

    let mut tx = pool.begin().await.map_err(|_e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to begin transaction",
        )
    })?;

    match crate::sql::invariants::create(&mut tx, &invariant_id, &request.asserts).await {
        Ok(()) => {
            tx.commit().await.map_err(|_e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "failed to commit transaction",
                )
            })?;
            Ok((
                StatusCode::CREATED,
                Json(CreateInvariantResponse {
                    invariant_id,
                    asserts: request.asserts,
                }),
            ))
        }
        Err(crate::DataStoreError::AlreadyExists) => {
            Err((StatusCode::CONFLICT, "invariant already exists"))
        }
        Err(_) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to create invariant",
        )),
    }
}

/// HTTP endpoint for getting a specific invariant by ID.
async fn get_invariant(
    State(pool): State<sqlx::PgPool>,
    Path(invariant_base64): Path<String>,
) -> Result<Json<GetInvariantResponse>, (StatusCode, &'static str)> {
    let invariant_string = format!("{}{}", INVARIANT_PREFIX, invariant_base64);

    let invariant_id = InvariantID::from_str(&invariant_string)
        .map_err(|_| (StatusCode::BAD_REQUEST, "invalid invariant id"))?;

    let mut tx = pool.begin().await.map_err(|_e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to begin transaction",
        )
    })?;

    match crate::sql::invariants::get(&mut tx, &invariant_id).await {
        Ok(Some(record)) => {
            tx.commit().await.map_err(|_e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "failed to commit transaction",
                )
            })?;
            Ok(Json(GetInvariantResponse {
                invariant_id: record.invariant_id,
                asserts: record.asserts,
                created_at: record.created_at,
                updated_at: record.updated_at,
            }))
        }
        Ok(None) => Err((StatusCode::NOT_FOUND, "invariant not found")),
        Err(_) => Err((StatusCode::INTERNAL_SERVER_ERROR, "failed to get invariant")),
    }
}

/// HTTP endpoint for updating an existing invariant.
async fn update_invariant(
    State(pool): State<sqlx::PgPool>,
    Path(invariant_base64): Path<String>,
    Json(request): Json<UpdateInvariantRequest>,
) -> Result<Json<GetInvariantResponse>, (StatusCode, &'static str)> {
    let invariant_string = format!("{}{}", INVARIANT_PREFIX, invariant_base64);

    let invariant_id = InvariantID::from_str(&invariant_string)
        .map_err(|_| (StatusCode::BAD_REQUEST, "invalid invariant id"))?;

    let mut tx = pool.begin().await.map_err(|_e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to begin transaction",
        )
    })?;

    match crate::sql::invariants::update(&mut tx, &invariant_id, &request.asserts).await {
        Ok(true) => match crate::sql::invariants::get(&mut tx, &invariant_id).await {
            Ok(Some(record)) => {
                tx.commit().await.map_err(|_e| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "failed to commit transaction",
                    )
                })?;
                Ok(Json(GetInvariantResponse {
                    invariant_id: record.invariant_id,
                    asserts: record.asserts,
                    created_at: record.created_at,
                    updated_at: record.updated_at,
                }))
            }
            Ok(None) => Err((StatusCode::NOT_FOUND, "invariant not found")),
            Err(_) => Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to get updated invariant",
            )),
        },
        Ok(false) => Err((StatusCode::NOT_FOUND, "invariant not found")),
        Err(_) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to update invariant",
        )),
    }
}

/// HTTP endpoint for deleting an invariant by ID.
async fn delete_invariant(
    State(pool): State<sqlx::PgPool>,
    Path(invariant_base64): Path<String>,
) -> Result<StatusCode, (StatusCode, &'static str)> {
    let invariant_string = format!("{}{}", INVARIANT_PREFIX, invariant_base64);

    let invariant_id = InvariantID::from_str(&invariant_string)
        .map_err(|_| (StatusCode::BAD_REQUEST, "invalid invariant id"))?;

    let mut tx = pool.begin().await.map_err(|_e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to begin transaction",
        )
    })?;

    match crate::sql::invariants::delete(&mut tx, &invariant_id).await {
        Ok(true) => {
            tx.commit().await.map_err(|_e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "failed to commit transaction",
                )
            })?;
            Ok(StatusCode::NO_CONTENT)
        }
        Ok(false) => Err((StatusCode::NOT_FOUND, "invariant not found")),
        Err(_) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to delete invariant",
        )),
    }
}

/// HTTP endpoint for listing all invariants.
async fn list_invariants(
    State(pool): State<sqlx::PgPool>,
) -> Result<Json<Vec<GetInvariantResponse>>, (StatusCode, &'static str)> {
    let mut tx = pool.begin().await.map_err(|_e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to begin transaction",
        )
    })?;

    match crate::sql::invariants::list(&mut tx).await {
        Ok(records) => {
            tx.commit().await.map_err(|_e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "failed to commit transaction",
                )
            })?;
            let responses = records
                .into_iter()
                .map(|record| GetInvariantResponse {
                    invariant_id: record.invariant_id,
                    asserts: record.asserts,
                    created_at: record.created_at,
                    updated_at: record.updated_at,
                })
                .collect();
            Ok(Json(responses))
        }
        Err(_) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to list invariants",
        )),
    }
}

////////////////////////////////////////////// Router //////////////////////////////////////////////////

/// Creates an Axum router with invariant management endpoints.
///
/// # Arguments
/// * `pool` - PostgreSQL connection pool for invariant operations
///
/// # Routes
/// - `GET /invariant` - List all invariants
/// - `POST /invariant` - Create a new invariant
/// - `GET /invariant/{invariant_id}` - Get a specific invariant by ID
/// - `PUT /invariant/{invariant_id}` - Update an invariant by ID
/// - `DELETE /invariant/{invariant_id}` - Delete an invariant by ID
///
/// # Returns
/// An Axum `Router` configured with the invariant endpoints and state.
///
/// # Examples
/// ```no_run
/// # use stigmergy::create_invariant_router;
/// # use sqlx::PgPool;
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// # let database_url = "postgres://localhost/stigmergy";
/// let pool = PgPool::connect(database_url).await?;
/// let router = create_invariant_router(pool);
/// # Ok(())
/// # }
/// ```
pub fn create_invariant_router(pool: sqlx::PgPool) -> Router {
    Router::new()
        .route("/invariant", get(list_invariants).post(create_invariant))
        .route(
            "/invariant/:invariant_id",
            get(get_invariant)
                .put(update_invariant)
                .delete(delete_invariant),
        )
        .with_state(pool)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invariant_new_and_accessors() {
        let bytes = [1u8; 32];
        let invariant = InvariantID::new(bytes);
        assert_eq!(invariant.as_bytes(), &bytes);
        assert_eq!(invariant.into_bytes(), bytes);
    }

    #[test]
    fn invariant_display_format() {
        let invariant = InvariantID::new([0u8; 32]);
        let display = format!("{}", invariant);
        assert!(display.starts_with("invariant:"));
        assert_eq!(display.len(), INVARIANT_PREFIX_LEN + BASE64_ENCODED_LEN);
        assert_eq!(
            display,
            "invariant:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
        );
    }

    #[test]
    fn invariant_from_str_valid() {
        let invariant_str = "invariant:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
        let invariant = InvariantID::from_str(invariant_str).unwrap();
        assert_eq!(invariant.as_bytes(), &[0u8; 32]);
    }

    #[test]
    fn invariant_from_str_invalid_prefix() {
        let result = InvariantID::from_str("invalid:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA");
        assert_eq!(result, Err(InvariantIDParseError::InvalidPrefix));
    }

    #[test]
    fn invariant_from_str_invalid_length() {
        let result = InvariantID::from_str("invariant:ABC");
        assert_eq!(result, Err(InvariantIDParseError::InvalidFormat));
    }

    #[test]
    fn invariant_from_str_without_prefix() {
        let base64_str = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
        let invariant = InvariantID::from_str(base64_str).unwrap();
        assert_eq!(invariant.as_bytes(), &[0u8; 32]);
    }

    #[test]
    fn invariant_round_trip() {
        let original_bytes = [
            0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66,
            0x77, 0x88, 0x99, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0x00, 0x01, 0x02, 0x03, 0x04,
            0x05, 0x06, 0x07, 0x08,
        ];
        let invariant = InvariantID::new(original_bytes);

        let display_str = format!("{}", invariant);
        let parsed_invariant = InvariantID::from_str(&display_str).unwrap();
        assert_eq!(parsed_invariant.as_bytes(), &original_bytes);
        assert_eq!(invariant, parsed_invariant);
    }

    #[test]
    fn invariant_base64_part_method() {
        let invariant = InvariantID::new([1u8; 32]);
        let base64_part = invariant.base64_part();

        assert_eq!(base64_part.len(), 43);
        assert!(!base64_part.contains("invariant:"));

        let parsed = InvariantID::from_str(&base64_part).unwrap();
        assert_eq!(parsed, invariant);
    }
}
