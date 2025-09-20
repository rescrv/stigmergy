use std::fmt::{Display, Formatter, Result as FmtResult};
use std::fs::File;
use std::io::Read;
use std::str::FromStr;

use axum::Router;
use axum::extract::Path;
use axum::http::StatusCode;
use axum::response::Json;
use axum::routing::{delete, post};
use serde::{Deserialize, Serialize};

/////////////////////////////////////////////// Entity ////////////////////////////////////////////////

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Entity([u8; 32]);

impl Entity {
    pub fn new(bytes: [u8; 32]) -> Self {
        Entity(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    pub fn into_bytes(self) -> [u8; 32] {
        self.0
    }
}

////////////////////////////////////// URL-Safe Base64 Encoding //////////////////////////////////////

const BASE64_CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

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
        write!(f, "entity:{}", encoded)
    }
}

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

////////////////////////////////////////////// Routes //////////////////////////////////////////////////

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateEntityRequest {
    pub entity: Option<Entity>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateEntityResponse {
    pub entity: Entity,
    pub created: bool,
}

async fn create_entity(
    Json(request): Json<CreateEntityRequest>,
) -> Result<Json<CreateEntityResponse>, StatusCode> {
    let entity = match request.entity {
        Some(entity) => entity,
        None => {
            // Generate a random entity if none provided, avoiding - and _ characters
            loop {
                let mut random_bytes = [0u8; 32];
                match File::open("/dev/urandom") {
                    Ok(mut file) => {
                        if file.read_exact(&mut random_bytes).is_err() {
                            return Err(StatusCode::INTERNAL_SERVER_ERROR);
                        }
                    }
                    Err(_) => {
                        return Err(StatusCode::INTERNAL_SERVER_ERROR);
                    }
                }

                let entity = Entity::new(random_bytes);
                let entity_string = entity.to_string();

                // Check if the base64 part contains - or _
                let base64_part = &entity_string[7..]; // Skip "entity:"
                if !base64_part.contains('-') && !base64_part.contains('_') {
                    break entity;
                }
                // If it contains - or _, continue the loop to generate a new one
            }
        }
    };

    let response = CreateEntityResponse {
        entity,
        created: true,
    };

    Ok(Json(response))
}

async fn delete_entity(Path(entity_base64): Path<String>) -> Result<StatusCode, StatusCode> {
    // Construct full entity string from base64 part
    let entity_string = format!("entity:{}", entity_base64);

    // Parse the entity ID
    match Entity::from_str(&entity_string) {
        Ok(_entity) => {
            // Entity exists and was deleted successfully
            Ok(StatusCode::NO_CONTENT)
        }
        Err(_) => {
            // Invalid entity ID format
            Err(StatusCode::BAD_REQUEST)
        }
    }
}

////////////////////////////////////////////// Router //////////////////////////////////////////////////

pub fn create_entity_router() -> Router {
    Router::new()
        .route("/entity", post(create_entity))
        .route("/entity/:entity_id", delete(delete_entity))
}

impl FromStr for Entity {
    type Err = EntityParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if !s.starts_with("entity:") {
            return Err(EntityParseError::InvalidPrefix);
        }

        let base64_part = &s[7..]; // Skip "entity:"

        if base64_part.len() != 43 {
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
        assert_eq!(display.len(), 50); // "entity:" (7) + base64 (43)
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
        assert_eq!(display.len(), 50);
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
        println!("Display string: {}", display_str); // TODO(claude): cleanup this output

        let parsed_entity = Entity::from_str(&display_str).unwrap();
        assert_eq!(parsed_entity.as_bytes(), &original_bytes);
        assert_eq!(entity, parsed_entity);
    }

    #[test]
    fn entity_display_matches_expected_format() {
        let entity = Entity::new([0xFF; 32]);
        let display = format!("{}", entity);

        // Verify it matches the expected regex pattern entity:[0-9A-Za-z_-]{43}
        assert!(display.starts_with("entity:"));
        let base64_part = &display[7..];
        assert_eq!(base64_part.len(), 43);

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

        let response = create_entity(Json(request)).await.unwrap();

        assert_eq!(response.0.entity, test_entity);
        assert!(response.0.created);
    }

    #[tokio::test]
    async fn create_entity_generates_random_when_none() {
        let request = CreateEntityRequest { entity: None };

        let response = create_entity(Json(request)).await;

        // Must succeed - if /dev/urandom is not available, fail the test
        let response =
            response.expect("/dev/urandom should be available for random entity generation");
        assert!(response.0.created);
        // The entity should be randomly generated (not all zeros)
        assert_ne!(response.0.entity, Entity::new([0u8; 32]));
    }

    #[tokio::test]
    async fn create_entity_avoids_special_characters() {
        // Generate multiple random entities and verify none contain - or _
        for _ in 0..1000 {
            let request = CreateEntityRequest { entity: None };

            let response = create_entity(Json(request)).await;

            // Must succeed - if /dev/urandom is not available, fail the test
            let response =
                response.expect("/dev/urandom should be available for random entity generation");

            let entity_string = response.0.entity.to_string();
            let base64_part = &entity_string[7..]; // Skip "entity:"

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
    }

    #[tokio::test]
    async fn delete_entity_valid_id() {
        let entity = Entity::new([1u8; 32]);
        let entity_str = entity.to_string();
        // Extract just the base64 part after "entity:"
        let base64_part = entity_str.strip_prefix("entity:").unwrap();

        let result = delete_entity(Path(base64_part.to_string())).await;

        assert_eq!(result, Ok(StatusCode::NO_CONTENT));
    }

    #[tokio::test]
    async fn delete_entity_invalid_id() {
        let invalid_base64 = "invalid_entity_id".to_string();

        let result = delete_entity(Path(invalid_base64)).await;

        assert_eq!(result, Err(StatusCode::BAD_REQUEST));
    }
}
