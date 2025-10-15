//! # Shared Command Utilities
//!
//! This module provides shared validation, parsing, and utility functions
//! used across multiple command handlers to reduce code duplication.

use crate::commands::errors::UserError;
use crate::{Entity, SystemName, cli_utils, http_utils};
use handled::Handle;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// Generic ID parsing function that works with any ID type that implements FromStr
/// and whose error type implements Handle<UserError>.
fn parse_id_or_exit_generic<T, E>(id_str: &str, id_type_name: &str) -> T
where
    T: FromStr<Err = E>,
    E: Handle<UserError> + std::fmt::Display,
{
    id_str.parse().unwrap_or_else(|e: E| {
        if let Some(user_error) = e.handle() {
            if let Some(ref hint) = user_error.usage_hint {
                cli_utils::exit_with_usage_error(&user_error.message, hint);
            } else {
                cli_utils::exit_with_error(&user_error.message);
            }
        } else {
            cli_utils::exit_with_error(&format!("Invalid {}: {}", id_type_name, e));
        }
    })
}

/// Validates and parses an entity ID from a string with enhanced error handling.
///
/// # Arguments
/// * `entity_id_str` - The string representation of the entity ID
///
/// # Returns
/// The parsed Entity ID, or exits the program with an enhanced error message
pub fn parse_entity_id_or_exit(entity_id_str: &str) -> Entity {
    parse_id_or_exit_generic(entity_id_str, "entity ID")
}

/// Validates and parses a system name from a string.
///
/// # Arguments
/// * `name_str` - The string representation of the system name
///
/// # Returns
/// The parsed SystemName, or exits the program with an error message
pub fn parse_system_name_or_exit(name_str: &str) -> SystemName {
    name_str.parse().unwrap_or_else(|e| {
        cli_utils::exit_with_error(&format!("{}", e));
    })
}

/// Validates required arguments count and exits with usage error if insufficient.
///
/// # Arguments
/// * `args` - The command arguments array
/// * `required_count` - The minimum number of arguments required
/// * `command` - The command name for error message
/// * `usage` - The usage string to display
pub fn require_args_or_exit(args: &[String], required_count: usize, command: &str, usage: &str) {
    if args.len() < required_count {
        cli_utils::exit_with_usage_error(
            &format!("{} command requires more arguments", command),
            usage,
        );
    }
}

/// Validates both minimum and maximum argument counts.
///
/// # Arguments
/// * `args` - The command arguments array
/// * `min_count` - The minimum number of arguments required (including subcommand)
/// * `max_count` - The maximum number of arguments allowed (including subcommand)
/// * `command` - The command name for error message
/// * `usage` - The usage string to display
pub fn validate_args_count_or_exit(
    args: &[String],
    min_count: usize,
    max_count: usize,
    command: &str,
    usage: &str,
) {
    if args.len() < min_count {
        cli_utils::exit_with_usage_error(
            &format!("{} command requires more arguments", command),
            usage,
        );
    }
    if args.len() > max_count {
        cli_utils::exit_with_usage_error(
            &format!("{} command has too many arguments", command),
            usage,
        );
    }
}

/// Macro to generate command dispatcher boilerplate.
macro_rules! dispatch_command {
    ($command_name:expr, $usage:expr, $args:expr, $client:expr, $output_format:expr, {
        $($subcommand:expr => $handler:expr),* $(,)?
    }) => {
        if $args.is_empty() {
            crate::cli_utils::exit_with_usage_error(
                &format!("{} command requires a subcommand", $command_name),
                $usage,
            );
        }

        match $args[0].as_str() {
            $(
                $subcommand => $handler($args, $client, $output_format).await,
            )*
            _ => {
                let available_subcommands = vec![$($subcommand),*];
                crate::cli_utils::exit_with_error(&format!(
                    "Unknown {} subcommand '{}'. Available subcommands: {}",
                    $command_name,
                    $args[0],
                    available_subcommands.join(", ")
                ));
            }
        }
    };
}

pub(crate) use dispatch_command;

/// HTTP operation helper utilities to reduce duplicate request patterns.
pub struct HttpOperations;

impl HttpOperations {
    /// Generic POST operation with error handling.
    pub async fn post<Req, Resp>(
        client: &http_utils::StigmergyClient,
        path: &str,
        request: &Req,
        context: &str,
    ) -> Resp
    where
        Req: Serialize,
        Resp: for<'de> Deserialize<'de>,
    {
        http_utils::execute_or_exit(
            || client.post::<Req, Resp>(path, request),
            &format!("Failed to {}", context),
        )
        .await
    }

    /// Generic GET operation with error handling.
    pub async fn get<Resp>(client: &http_utils::StigmergyClient, path: &str, context: &str) -> Resp
    where
        Resp: for<'de> Deserialize<'de>,
    {
        http_utils::execute_or_exit(
            || client.get::<Resp>(path),
            &format!("Failed to {}", context),
        )
        .await
    }

    /// Generic PUT operation with error handling.
    pub async fn put<Req, Resp>(
        client: &http_utils::StigmergyClient,
        path: &str,
        request: &Req,
        context: &str,
    ) -> Resp
    where
        Req: Serialize,
        Resp: for<'de> Deserialize<'de>,
    {
        http_utils::execute_or_exit(
            || client.put::<Req, Resp>(path, request),
            &format!("Failed to {}", context),
        )
        .await
    }

    /// Generic DELETE operation with error handling.
    pub async fn delete(client: &http_utils::StigmergyClient, path: &str, context: &str) {
        http_utils::execute_or_exit(|| client.delete(path), &format!("Failed to {}", context))
            .await;
    }
}

/// URL builder utilities for API endpoints to eliminate duplicate path construction.
pub struct ApiUrlBuilder;

impl ApiUrlBuilder {
    const API_V1_PREFIX: &'static str = "/api/v1";

    /// Build entity API URL.
    pub fn entity(id: Option<&str>) -> String {
        match id {
            Some(id) => format!("{}/entity/{}", Self::API_V1_PREFIX, id),
            None => format!("{}/entity", Self::API_V1_PREFIX),
        }
    }

    /// Build system API URL.
    pub fn system(id: Option<&str>) -> String {
        match id {
            Some(id) => format!("{}/system/{}", Self::API_V1_PREFIX, id),
            None => format!("{}/system", Self::API_V1_PREFIX),
        }
    }

    /// Build component definition API URL.
    pub fn component_definition(id: Option<&str>) -> String {
        match id {
            Some(id) => format!("{}/componentdefinition/{}", Self::API_V1_PREFIX, id),
            None => format!("{}/componentdefinition", Self::API_V1_PREFIX),
        }
    }

    /// Build component API URL.
    pub fn component(entity_id: Option<&str>) -> String {
        match entity_id {
            Some(id) => format!("{}/entity/{}/component", Self::API_V1_PREFIX, id),
            None => format!("{}/component", Self::API_V1_PREFIX),
        }
    }

    /// Build system from markdown API URL.
    pub fn system_from_markdown() -> String {
        format!("{}/system/from-markdown", Self::API_V1_PREFIX)
    }

    /// Build full URL with base URL.
    pub fn build_url(base_url: &str, path: &str) -> String {
        format!("{}{}", base_url, path)
    }
}
