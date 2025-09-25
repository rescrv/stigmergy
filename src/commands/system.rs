//! # System Command Handler
//!
//! This module handles system-related CLI commands including creation,
//! listing, retrieval, updating, and deletion of systems.

use crate::{
    CreateSystemFromMarkdownRequest, CreateSystemRequest, CreateSystemResponse, System,
    SystemConfig, SystemListItem, cli_utils,
    commands::shared::{dispatch_command, parse_system_id_or_exit, validate_args_count_or_exit},
    http_utils,
};

const SYSTEM_USAGE: &str =
    "Usage: stigctl system <create|create-from-md|list|get|update|delete> [args...]";

/// Handles all system-related commands.
///
/// # Arguments
/// * `args` - Command arguments (first element is the subcommand)
/// * `client` - HTTP client for API communication
pub async fn handle_system_command(args: &[String], client: &http_utils::StigmergyClient) {
    dispatch_command!("system", SYSTEM_USAGE, args, client, {
        "create" => handle_system_create,
        "create-from-md" => handle_system_create_from_md,
        "list" => handle_system_list,
        "get" => handle_system_get,
        "update" => handle_system_update,
        "delete" => handle_system_delete,
    });
}

/// Handles system creation from JSON config.
async fn handle_system_create(args: &[String], client: &http_utils::StigmergyClient) {
    validate_args_count_or_exit(
        args,
        2,
        2,
        "create",
        r#"Usage: stigctl system create <config-json>
Example: stigctl system create '{"name":"test","description":"A test system","tools":["Read","Write"],"model":"inherit","color":"blue","content":"You are a test system."}}'"#,
    );

    let config_str = &args[1];
    let config: SystemConfig = serde_json::from_str(config_str)
        .unwrap_or_else(|e| cli_utils::exit_with_error(&format!("Invalid config JSON: {}", e)));

    let request = CreateSystemRequest { config };

    let response = http_utils::execute_or_exit(
        || client.post::<CreateSystemRequest, CreateSystemResponse>("system", &request),
        "Failed to create system",
    )
    .await;

    println!("Created system:");
    cli_utils::print_json_or_exit(&response.system, "system");
}

/// Handles system creation from markdown file.
async fn handle_system_create_from_md(args: &[String], client: &http_utils::StigmergyClient) {
    validate_args_count_or_exit(
        args,
        2,
        2,
        "create-from-md",
        "Usage: stigctl system create-from-md <file.md>",
    );

    let file_path = &args[1];
    let content = std::fs::read_to_string(file_path).unwrap_or_else(|e| {
        cli_utils::exit_with_error(&format!("Failed to read file {}: {}", file_path, e))
    });

    let request = CreateSystemFromMarkdownRequest { content };

    let response = http_utils::execute_or_exit(
        || {
            client.post::<CreateSystemFromMarkdownRequest, CreateSystemResponse>(
                "system/from-markdown",
                &request,
            )
        },
        "Failed to create system from markdown",
    )
    .await;

    println!("Created system from markdown:");
    cli_utils::print_json_or_exit(&response.system, "system");
}

/// Handles system listing.
async fn handle_system_list(args: &[String], client: &http_utils::StigmergyClient) {
    validate_args_count_or_exit(args, 1, 1, "list", "Usage: stigctl system list");

    let systems = http_utils::execute_or_exit(
        || client.get::<Vec<SystemListItem>>("system"),
        "Failed to list systems",
    )
    .await;

    cli_utils::print_json_or_exit(&systems, "systems");
}

/// Handles system retrieval by ID.
async fn handle_system_get(args: &[String], client: &http_utils::StigmergyClient) {
    validate_args_count_or_exit(args, 2, 2, "get", "Usage: stigctl system get <system-id>");

    let system_id = parse_system_id_or_exit(&args[1]);
    let path = format!("system/{}", system_id.base64_part());

    let system = http_utils::execute_or_exit(
        || client.get::<System>(&path),
        &format!("Failed to get system {}", system_id),
    )
    .await;

    cli_utils::print_json_or_exit(&system, "system");
}

/// Handles system update.
async fn handle_system_update(args: &[String], client: &http_utils::StigmergyClient) {
    validate_args_count_or_exit(
        args,
        3,
        3,
        "update",
        "Usage: stigctl system update <system-id> <config-json>",
    );

    let system_id = parse_system_id_or_exit(&args[1]);
    let config_str = &args[2];

    let config: SystemConfig = serde_json::from_str(config_str)
        .unwrap_or_else(|e| cli_utils::exit_with_error(&format!("Invalid config JSON: {}", e)));

    let path = format!("system/{}", system_id.base64_part());
    let system = http_utils::execute_or_exit(
        || client.put::<SystemConfig, System>(&path, &config),
        &format!("Failed to update system {}", system_id),
    )
    .await;

    println!("Updated system:");
    cli_utils::print_json_or_exit(&system, "system");
}

/// Handles system deletion.
async fn handle_system_delete(args: &[String], client: &http_utils::StigmergyClient) {
    validate_args_count_or_exit(
        args,
        2,
        2,
        "delete",
        "Usage: stigctl system delete <system-id>",
    );

    let system_id = parse_system_id_or_exit(&args[1]);
    let path = format!("system/{}", system_id.base64_part());

    http_utils::execute_or_exit(
        || client.delete(&path),
        &format!("Failed to delete system {}", system_id),
    )
    .await;

    println!("Deleted system: {}", system_id);
}
