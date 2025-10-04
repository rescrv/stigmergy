//! # Error Extensions for Handled
//!
//! This module extends existing error types in the codebase to implement
//! Handle<UserError> for consistent error property extraction.

use super::errors::UserError;
use crate::EntityParseError;
use handled::Handle;

/// Implement Handle<UserError> for EntityParseError
impl Handle<UserError> for EntityParseError {
    fn handle(&self) -> Option<UserError> {
        let (message, hint) = match self {
            EntityParseError::InvalidPrefix => (
                "Entity ID must start with 'entity:' prefix or be a valid base64 string"
                    .to_string(),
                Some(
                    "Use format 'entity:BASE64_STRING' or just 'BASE64_STRING' (43 characters)"
                        .to_string(),
                ),
            ),
            EntityParseError::InvalidFormat => (
                "Entity ID format is invalid - expected 43-character base64 string".to_string(),
                Some("Entity IDs must be exactly 43 characters of URL-safe base64".to_string()),
            ),
            EntityParseError::InvalidBase64 => (
                "Entity ID contains invalid base64 characters".to_string(),
                Some("Use only URL-safe base64 characters (A-Z, a-z, 0-9, -, _)".to_string()),
            ),
            EntityParseError::InvalidLength => (
                "Entity ID must decode to exactly 32 bytes".to_string(),
                Some("Entity IDs must be exactly 43 characters when base64 encoded".to_string()),
            ),
        };

        Some(UserError {
            message,
            usage_hint: hint,
        })
    }
}

/// Implement Handle<UserError> for serde_json::Error
impl Handle<UserError> for serde_json::Error {
    fn handle(&self) -> Option<UserError> {
        Some(UserError {
            message: format!("JSON parsing error: {}", self),
            usage_hint: Some(
                "Ensure the JSON is properly formatted and contains all required fields"
                    .to_string(),
            ),
        })
    }
}

/// Implement Handle<UserError> for std::io::Error
impl Handle<UserError> for std::io::Error {
    fn handle(&self) -> Option<UserError> {
        let hint = match self.kind() {
            std::io::ErrorKind::NotFound => {
                Some("The specified file was not found. Check the file path.".to_string())
            }
            std::io::ErrorKind::PermissionDenied => {
                Some("Permission denied. Check file permissions.".to_string())
            }
            std::io::ErrorKind::InvalidData => Some("The file contains invalid data.".to_string()),
            _ => None,
        };

        Some(UserError {
            message: format!("File operation error: {}", self),
            usage_hint: hint,
        })
    }
}
