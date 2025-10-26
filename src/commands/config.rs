//! # Config Command Handler
//!
//! This module handles config-related CLI commands including retrieving
//! and setting the system configuration.

use crate::{
    Config, GetConfigResponse, PostConfigRequest, PostConfigResponse, cli_utils,
    commands::shared::{dispatch_command, validate_args_count_or_exit},
    http_utils,
};

const CONFIG_USAGE: &str = "Usage: stigctl config <get|set> [args...]";

/// Handles all config-related commands.
///
/// # Arguments
/// * `args` - Command arguments (first element is the subcommand)
/// * `client` - HTTP client for API communication
/// * `output_format` - Output format for get/list commands
pub async fn handle_config_command(
    args: &[String],
    client: &http_utils::StigmergyClient,
    output_format: cli_utils::OutputFormat,
) {
    dispatch_command!("config", CONFIG_USAGE, args, client, output_format, {
        "get" => handle_config_get,
        "set" => handle_config_set,
    });
}

/// Handles config retrieval.
async fn handle_config_get(
    args: &[String],
    client: &http_utils::StigmergyClient,
    output_format: cli_utils::OutputFormat,
) {
    validate_args_count_or_exit(args, 1, 1, "get", "Usage: stigctl config get");

    let response = http_utils::execute_or_exit(
        || client.get::<GetConfigResponse>("config"),
        "Failed to get config",
    )
    .await;

    cli_utils::print_formatted_or_exit(&response.config, output_format, "config");
}

/// Handles config setting from a file.
async fn handle_config_set(
    args: &[String],
    client: &http_utils::StigmergyClient,
    output_format: cli_utils::OutputFormat,
) {
    validate_args_count_or_exit(
        args,
        2,
        2,
        "set",
        "Usage: stigctl config set <file.json|file.yaml>",
    );

    let file_path = &args[1];
    let content = std::fs::read_to_string(file_path).unwrap_or_else(|e| {
        cli_utils::exit_with_error(&format!("Failed to read file {}: {}", file_path, e))
    });

    let config = parse_config_from_content(file_path, &content);

    let request = PostConfigRequest { config };

    let response = http_utils::execute_or_exit(
        || client.post::<PostConfigRequest, PostConfigResponse>("config", &request),
        "Failed to set config",
    )
    .await;

    println!(
        "Config updated successfully (version: {})",
        response.version
    );
    cli_utils::print_formatted_or_exit(&response.config, output_format, "config");
}

/// Parses config from file content, attempting both JSON and YAML formats.
fn parse_config_from_content(file_path: &str, content: &str) -> Config {
    if file_path.ends_with(".json") {
        serde_json::from_str(content).unwrap_or_else(|e| {
            cli_utils::exit_with_error(&format!("Failed to parse JSON config: {}", e))
        })
    } else if file_path.ends_with(".yaml") || file_path.ends_with(".yml") {
        serde_yml::from_str(content).unwrap_or_else(|e| {
            cli_utils::exit_with_error(&format!("Failed to parse YAML config: {}", e))
        })
    } else if let Ok(config) = serde_json::from_str::<Config>(content) {
        config
    } else if let Ok(config) = serde_yml::from_str::<Config>(content) {
        config
    } else {
        cli_utils::exit_with_error("Failed to parse config file. Ensure it is valid JSON or YAML.")
    }
}
