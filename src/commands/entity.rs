//! # Entity Command Handler
//!
//! This module handles entity-related CLI commands including creation, listing,
//! and deletion of entities.

use crate::{
    CreateEntityRequest, CreateEntityResponse, Entity, cli_utils,
    commands::shared::{dispatch_command, parse_entity_id_or_exit, validate_args_count_or_exit},
    http_utils,
};

const ENTITY_USAGE: &str = "Usage: stigctl entity <create|list|delete> [args...]";

/// Handles all entity-related commands.
///
/// # Arguments
/// * `args` - Command arguments (first element is the subcommand)
/// * `client` - HTTP client for API communication
/// * `output_format` - Output format for get/list commands
pub async fn handle_entity_command(
    args: &[String],
    client: &http_utils::StigmergyClient,
    output_format: cli_utils::OutputFormat,
) {
    dispatch_command!("entity", ENTITY_USAGE, args, client, output_format, {
        "create" => handle_entity_create,
        "list" => handle_entity_list,
        "delete" => handle_entity_delete,
    });
}

/// Handles entity creation command.
async fn handle_entity_create(
    args: &[String],
    client: &http_utils::StigmergyClient,
    _output_format: cli_utils::OutputFormat,
) {
    validate_args_count_or_exit(args, 1, 1, "create", "Usage: stigctl entity create");
    let request = CreateEntityRequest { entity: None };

    let response = http_utils::execute_or_exit(
        || client.post::<CreateEntityRequest, CreateEntityResponse>("entity", &request),
        "Failed to create entity",
    )
    .await;

    println!("Created entity: {}", response.entity);
}

/// Handles entity listing command.
async fn handle_entity_list(
    args: &[String],
    client: &http_utils::StigmergyClient,
    output_format: cli_utils::OutputFormat,
) {
    validate_args_count_or_exit(args, 1, 1, "list", "Usage: stigctl entity list");
    let entities = http_utils::execute_or_exit(
        || client.get::<Vec<Entity>>("entity"),
        "Failed to list entities",
    )
    .await;

    if entities.is_empty() {
        println!("No entities found");
    } else if output_format == cli_utils::OutputFormat::Json
        || output_format == cli_utils::OutputFormat::Yaml
    {
        cli_utils::print_formatted_or_exit(&entities, output_format, "entities");
    } else {
        println!("Entities:");
        for entity in entities {
            println!("  {}", entity);
        }
    }
}

/// Handles entity deletion command.
async fn handle_entity_delete(
    args: &[String],
    client: &http_utils::StigmergyClient,
    _output_format: cli_utils::OutputFormat,
) {
    validate_args_count_or_exit(
        args,
        2,
        2,
        "delete",
        "Usage: stigctl entity delete <entity-id>",
    );

    let entity_id = parse_entity_id_or_exit(&args[1]);
    let path = format!("entity/{}", entity_id.base64_part());

    http_utils::execute_or_exit(|| client.delete(&path), "Failed to delete entity").await;

    println!("Deleted entity: {}", entity_id);
}
