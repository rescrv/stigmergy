//! # Command Error Handling
//!
//! This module provides error handling utilities for stigctl CLI commands
//! using the handled crate for consistent error property extraction.

use handled::Handle;

/// User-friendly error information that can be extracted from various error types
#[derive(Debug, Clone)]
pub struct UserError {
    /// The main error message to display to the user
    pub message: String,
    /// Optional usage hint to help the user correct the error
    pub usage_hint: Option<String>,
}

impl std::fmt::Display for UserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

/// Implements Handle<UserError> for itself to allow extraction
impl Handle<UserError> for UserError {
    fn handle(&self) -> Option<UserError> {
        Some(self.clone())
    }
}

/// Entity parsing errors that provide user-friendly messages
#[derive(Debug)]
pub struct EntityParseError {
    /// The input string that failed to parse
    pub input: String,
    /// The reason why parsing failed
    pub reason: String,
}

impl std::fmt::Display for EntityParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Invalid entity ID '{}': {}", self.input, self.reason)
    }
}

impl std::error::Error for EntityParseError {}

impl Handle<UserError> for EntityParseError {
    fn handle(&self) -> Option<UserError> {
        Some(UserError {
            message: format!("Invalid entity ID '{}': {}", self.input, self.reason),
            usage_hint: Some(
                "Entity IDs should be in format 'entity:BASE64_STRING' or just 'BASE64_STRING'"
                    .to_string(),
            ),
        })
    }
}

/// HTTP operation errors that provide user-friendly messages
#[derive(Debug)]
pub struct HttpOperationError {
    /// The name of the operation that failed
    pub operation: String,
    /// The HTTP status code if available
    pub status: Option<u16>,
    /// Detailed error information
    pub details: String,
}

impl std::fmt::Display for HttpOperationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(status) = self.status {
            write!(
                f,
                "{} failed (HTTP {}): {}",
                self.operation, status, self.details
            )
        } else {
            write!(f, "{} failed: {}", self.operation, self.details)
        }
    }
}

impl std::error::Error for HttpOperationError {}

impl Handle<UserError> for HttpOperationError {
    fn handle(&self) -> Option<UserError> {
        let message = if let Some(status) = self.status {
            format!(
                "{} failed (HTTP {}): {}",
                self.operation, status, self.details
            )
        } else {
            format!("{} failed: {}", self.operation, self.details)
        };

        let usage_hint = match self.status {
            Some(404) => Some(
                "The requested resource was not found. Check the ID and try again.".to_string(),
            ),
            Some(400) => Some("Invalid request. Check your input data and try again.".to_string()),
            Some(401) => Some("Authentication required. Check your credentials.".to_string()),
            Some(403) => Some(
                "Access forbidden. You may not have permission for this operation.".to_string(),
            ),
            Some(429) => Some("Too many requests. Wait a moment and try again.".to_string()),
            Some(500..=599) => {
                Some("Server error. The service may be temporarily unavailable.".to_string())
            }
            _ => None,
        };

        Some(UserError {
            message,
            usage_hint,
        })
    }
}

impl HttpOperationError {
    /// Creates an HttpOperationError from a reqwest Response
    pub async fn from_response(response: reqwest::Response, operation: &str) -> Self {
        let status = response.status().as_u16();
        let details = response
            .text()
            .await
            .unwrap_or_else(|_| "No error details".to_string());

        Self {
            operation: operation.to_string(),
            status: Some(status),
            details: if details.is_empty() {
                "No error details".to_string()
            } else {
                details
            },
        }
    }

    /// Creates an HttpOperationError with a custom message
    pub fn new(operation: &str, details: &str) -> Self {
        Self {
            operation: operation.to_string(),
            status: None,
            details: details.to_string(),
        }
    }
}

/// Validation error for command arguments
#[derive(Debug)]
pub struct ValidationError {
    /// The field name that failed validation
    pub field: String,
    /// The value that was invalid
    pub value: String,
    /// The reason why validation failed
    pub reason: String,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Invalid {}: '{}' - {}",
            self.field, self.value, self.reason
        )
    }
}

impl std::error::Error for ValidationError {}

impl Handle<UserError> for ValidationError {
    fn handle(&self) -> Option<UserError> {
        Some(UserError {
            message: format!("Invalid {}: '{}' - {}", self.field, self.value, self.reason),
            usage_hint: Some("Check the documentation for valid input formats".to_string()),
        })
    }
}

/// Helper function to extract user-friendly error messages
pub fn extract_user_error<E>(error: &E) -> Option<UserError>
where
    E: Handle<UserError>,
{
    error.handle()
}

/// Enhanced error formatting for CLI output
pub fn format_cli_error<E>(error: &E) -> String
where
    E: Handle<UserError> + std::fmt::Display,
{
    if let Some(user_error) = error.handle() {
        let mut output = format!("Error: {}", user_error.message);
        if let Some(hint) = user_error.usage_hint {
            output.push_str(&format!("\nHint: {}", hint));
        }
        output
    } else {
        format!("Error: {}", error)
    }
}
