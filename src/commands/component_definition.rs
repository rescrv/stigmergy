//! # Component Definition Command Handler
//!
//! This module handles component definition-related CLI commands including
//! creation, listing, retrieval, updating, and deletion of component definitions.

use crate::{
    ComponentDefinition, cli_utils,
    commands::shared::{dispatch_command, validate_args_count_or_exit},
    component_utils, http_utils,
};
use serde_json::Value;

const COMPONENTDEFINITION_USAGE: &str =
    "Usage: stigctl componentdefinition <create|list|get|update|delete> [args...]";

/// Handles all component definition-related commands.
///
/// # Arguments
/// * `args` - Command arguments (first element is the subcommand)
/// * `client` - HTTP client for API communication
/// * `output_format` - Output format for get/list commands
pub async fn handle_componentdefinition_command(
    args: &[String],
    client: &http_utils::StigmergyClient,
    output_format: cli_utils::OutputFormat,
) {
    dispatch_command!("componentdefinition", COMPONENTDEFINITION_USAGE, args, client, output_format, {
        "create" => handle_componentdefinition_create,
        "list" => handle_componentdefinition_list,
        "get" => handle_componentdefinition_get,
        "update" => handle_componentdefinition_update,
        "delete" => handle_componentdefinition_delete,
    });
}

/// Handles component definition creation.
async fn handle_componentdefinition_create(
    args: &[String],
    client: &http_utils::StigmergyClient,
    output_format: cli_utils::OutputFormat,
) {
    validate_args_count_or_exit(
        args,
        3,
        3,
        "create",
        r#"Usage: stigctl componentdefinition create <name> <schema-json>
Example: stigctl componentdefinition create MyComponent '{"type":"object","properties":{"value":{"type":"string"}}}'"#,
    );

    let name = &args[1];
    let schema_str = &args[2];

    let schema = component_utils::parse_schema(schema_str)
        .unwrap_or_else(|e| cli_utils::exit_with_error(&e));

    let definition = component_utils::create_and_validate_definition(name, schema)
        .unwrap_or_else(|e| cli_utils::exit_with_error(&e));

    let created_definition = http_utils::execute_or_exit(
        || {
            client.post::<ComponentDefinition, ComponentDefinition>(
                "componentdefinition",
                &definition,
            )
        },
        "Failed to create component definition",
    )
    .await;

    println!("Created component definition:");
    cli_utils::print_formatted_or_exit(&created_definition, output_format, "component definition");
}

/// Handles component definition listing.
async fn handle_componentdefinition_list(
    args: &[String],
    client: &http_utils::StigmergyClient,
    output_format: cli_utils::OutputFormat,
) {
    validate_args_count_or_exit(
        args,
        1,
        1,
        "list",
        "Usage: stigctl componentdefinition list",
    );

    let definitions = http_utils::execute_or_exit(
        || client.get::<Vec<ComponentDefinition>>("componentdefinition"),
        "Failed to list component definitions",
    )
    .await;

    cli_utils::print_formatted_or_exit(&definitions, output_format, "component definitions");
}

/// Handles component definition retrieval by ID.
async fn handle_componentdefinition_get(
    args: &[String],
    client: &http_utils::StigmergyClient,
    output_format: cli_utils::OutputFormat,
) {
    validate_args_count_or_exit(
        args,
        2,
        2,
        "get",
        "Usage: stigctl componentdefinition get <id>",
    );

    let def_id = &args[1];
    let path = format!("componentdefinition/{}", def_id);
    let error_msg = format!("Failed to get component definition {}", def_id);

    let definition =
        http_utils::execute_or_exit(|| client.get::<ComponentDefinition>(&path), &error_msg).await;

    cli_utils::print_formatted_or_exit(&definition, output_format, "component definition");
}

/// Handles component definition update.
async fn handle_componentdefinition_update(
    args: &[String],
    client: &http_utils::StigmergyClient,
    output_format: cli_utils::OutputFormat,
) {
    validate_args_count_or_exit(
        args,
        3,
        3,
        "update",
        "Usage: stigctl componentdefinition update <id> <schema-json>",
    );

    let def_id = &args[1];
    let schema_str = &args[2];

    let schema = component_utils::parse_schema(schema_str)
        .unwrap_or_else(|e| cli_utils::exit_with_error(&e));

    component_utils::validate_schema_for_component(def_id, &schema)
        .unwrap_or_else(|e| cli_utils::exit_with_error(&e));

    let path = format!("componentdefinition/{}", def_id);
    let error_msg = format!("Failed to update component definition {}", def_id);

    let definition = http_utils::execute_or_exit(
        || client.put::<Value, ComponentDefinition>(&path, &schema),
        &error_msg,
    )
    .await;

    println!("Updated component definition:");
    cli_utils::print_formatted_or_exit(&definition, output_format, "component definition");
}

/// Handles component definition deletion.
async fn handle_componentdefinition_delete(
    args: &[String],
    client: &http_utils::StigmergyClient,
    _output_format: cli_utils::OutputFormat,
) {
    validate_args_count_or_exit(
        args,
        2,
        2,
        "delete",
        "Usage: stigctl componentdefinition delete <id>",
    );

    let def_id = &args[1];
    let path = format!("componentdefinition/{}", def_id);
    let error_msg = format!("Failed to delete component definition {}", def_id);

    http_utils::execute_or_exit(|| client.delete(&path), &error_msg).await;

    println!("Deleted component definition: {}", def_id);
}
