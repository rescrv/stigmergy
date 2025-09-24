//! # Command Line Interface Utilities
//!
//! This module provides common utilities for command-line applications, including
//! error handling, program termination, and formatted output functions.
//!
//! ## Key Features
//!
//! - **Error Handling**: Standardized error reporting with optional usage information
//! - **JSON Output**: Formatted JSON serialization for CLI responses
//! - **Program Termination**: Clean exit functions with appropriate error codes
//!
//! ## Usage Examples
//!
//! ```rust
//! use stigmergy::cli_utils;
//!
//! // Exit with a simple error message
//! // cli_utils::exit_with_error("Configuration file not found");
//!
//! // Print formatted JSON output
//! let data = vec!["item1", "item2"];
//! cli_utils::print_json(&data).unwrap();
//! ```

use std::process;

/// Terminates the program with an error message and exit code 1.
///
/// This function prints the error message to stderr and exits the program
/// immediately. It should be used for unrecoverable errors where the
/// program cannot continue execution.
///
/// # Arguments
/// * `message` - The error message to display
///
/// # Examples
/// ```no_run
/// use stigmergy::cli_utils::exit_with_error;
/// exit_with_error("Database connection failed");
/// ```
pub fn exit_with_error(message: &str) -> ! {
    eprintln!("Error: {}", message);
    process::exit(1);
}

/// Terminates the program with an error message, usage information, and exit code 1.
///
/// This function is designed for command-line argument parsing errors where
/// the user needs both the error context and usage instructions to correct
/// their input.
///
/// # Arguments
/// * `message` - The error message describing what went wrong
/// * `usage` - Usage instructions or help text to guide the user
///
/// # Examples
/// ```no_run
/// use stigmergy::cli_utils::exit_with_usage_error;
/// exit_with_usage_error(
///     "Missing required argument",
///     "Usage: program <entity_id> [options]"
/// );
/// ```
pub fn exit_with_usage_error(message: &str, usage: &str) -> ! {
    eprintln!("Error: {}", message);
    eprintln!("{}", usage);
    process::exit(1);
}

/// Prints a success message to stdout.
///
/// Simple utility function for consistent success message formatting
/// across CLI applications.
///
/// # Arguments
/// * `message` - The success message to display
///
/// # Examples
/// ```
/// use stigmergy::cli_utils::print_success;
/// print_success("Entity created successfully");
/// ```
pub fn print_success(message: &str) {
    println!("{}", message);
}

/// Serializes a value to pretty-printed JSON and outputs to stdout.
///
/// This function provides consistent JSON formatting across CLI tools,
/// using proper indentation for human readability.
///
/// # Arguments
/// * `value` - Any serializable value to output as JSON
///
/// # Returns
/// * `Ok(())` - JSON was successfully printed
/// * `Err(serde_json::Error)` - Serialization failed
///
/// # Examples
/// ```
/// use stigmergy::cli_utils::print_json;
/// use serde_json::json;
///
/// let data = json!({"status": "success", "count": 42});
/// print_json(&data).unwrap();
/// ```
pub fn print_json<T>(value: &T) -> Result<(), serde_json::Error>
where
    T: serde::Serialize,
{
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

/// Serializes a value to JSON and outputs to stdout, exiting on error.
///
/// This is a convenience wrapper around `print_json` that handles errors
/// by terminating the program with a descriptive error message. Use this
/// when JSON serialization failure is unrecoverable.
///
/// # Arguments
/// * `value` - Any serializable value to output as JSON
/// * `context` - Context description for error messages (e.g., "response", "entity")
///
/// # Examples
/// ```no_run
/// use stigmergy::cli_utils::print_json_or_exit;
/// use serde_json::json;
///
/// let entity_data = json!({"id": "entity:ABC123"});
/// print_json_or_exit(&entity_data, "entity");
/// ```
pub fn print_json_or_exit<T>(value: &T, context: &str)
where
    T: serde::Serialize,
{
    if let Err(e) = print_json(value) {
        exit_with_error(&format!("Failed to format {} JSON: {}", context, e));
    }
}
