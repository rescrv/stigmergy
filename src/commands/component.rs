//! # Component Command Handler
//!
//! This module handles component instance-related CLI commands including
//! creation, listing, retrieval, updating, and deletion of component instances
//! for entities.

use crate::{
    Component, ComponentListItem, CreateComponentRequest, CreateComponentResponse, cli_utils,
    commands::shared::{dispatch_command, parse_entity_id_or_exit, validate_args_count_or_exit},
    component_utils, http_utils,
};
use serde_json::Value;

const COMPONENT_USAGE: &str = "Usage: stigctl component <create|list|get|update|delete> [args...] (Note: all component operations now require an entity-id)";

/// Handles all component-related commands.
///
/// # Arguments
/// * `args` - Command arguments (first element is the subcommand)
/// * `client` - HTTP client for API communication
/// * `output_format` - Output format for get/list commands
pub async fn handle_component_command(
    args: &[String],
    client: &http_utils::StigmergyClient,
    output_format: cli_utils::OutputFormat,
) {
    dispatch_command!("component", COMPONENT_USAGE, args, client, output_format, {
        "create" => handle_component_create,
        "list" => handle_component_list,
        "get" => handle_component_get,
        "update" => handle_component_update,
        "delete" => handle_component_delete,
    });
}

/// Handles component instance creation.
async fn handle_component_create(
    args: &[String],
    client: &http_utils::StigmergyClient,
    output_format: cli_utils::OutputFormat,
) {
    validate_args_count_or_exit(
        args,
        4,
        4,
        "create",
        r#"Usage: stigctl component create <entity> <component> <data-json>
Example: stigctl component create entity:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA MyComponent '{"value":"hello"}'"#,
    );

    let entity_id_str = &args[1];
    let component_name = &args[2];
    let data_str = &args[3];

    let data = component_utils::parse_json_data(data_str)
        .unwrap_or_else(|e| cli_utils::exit_with_error(&e));

    let entity_id = parse_entity_id_or_exit(entity_id_str);

    let component = Component::new(component_name).unwrap_or_else(|| {
        cli_utils::exit_with_error(&format!("Invalid component name: {}", component_name))
    });

    let request = CreateComponentRequest { component, data };
    let path = format!("entity/{}/component", entity_id.base64_part());

    let response = http_utils::execute_or_exit(
        || client.post::<CreateComponentRequest, CreateComponentResponse>(&path, &request),
        "Failed to create component",
    )
    .await;

    println!("Created component:");
    cli_utils::print_formatted_or_exit(&response, output_format, "component");
}

/// Handles component listing for an entity.
async fn handle_component_list(
    args: &[String],
    client: &http_utils::StigmergyClient,
    output_format: cli_utils::OutputFormat,
) {
    validate_args_count_or_exit(
        args,
        2,
        2,
        "list",
        "Usage: stigctl component list <entity-id>",
    );

    let entity_id = parse_entity_id_or_exit(&args[1]);
    let path = format!("entity/{}/component", entity_id.base64_part());

    let components = http_utils::execute_or_exit(
        || client.get::<Vec<ComponentListItem>>(&path),
        "Failed to list components",
    )
    .await;

    cli_utils::print_formatted_or_exit(&components, output_format, "components");
}

/// Handles component retrieval by entity ID and component ID.
async fn handle_component_get(
    args: &[String],
    client: &http_utils::StigmergyClient,
    output_format: cli_utils::OutputFormat,
) {
    validate_args_count_or_exit(
        args,
        3,
        3,
        "get",
        "Usage: stigctl component get <entity-id> <component-id>",
    );

    let entity_id = parse_entity_id_or_exit(&args[1]);
    let comp_id = &args[2];
    let path = format!("entity/{}/component/{}", entity_id.base64_part(), comp_id);
    let error_msg = format!(
        "Failed to get component {} for entity {}",
        comp_id, entity_id
    );

    let component = http_utils::execute_or_exit(|| client.get::<Value>(&path), &error_msg).await;

    cli_utils::print_formatted_or_exit(&component, output_format, "component");
}

/// Handles component update.
async fn handle_component_update(
    args: &[String],
    client: &http_utils::StigmergyClient,
    output_format: cli_utils::OutputFormat,
) {
    validate_args_count_or_exit(
        args,
        4,
        4,
        "update",
        "Usage: stigctl component update <entity-id> <component-id> <data-json>",
    );

    let entity_id = parse_entity_id_or_exit(&args[1]);
    let comp_id = &args[2];
    let data_str = &args[3];

    let data = component_utils::parse_json_data(data_str)
        .unwrap_or_else(|e| cli_utils::exit_with_error(&e));

    let path = format!("entity/{}/component/{}", entity_id.base64_part(), comp_id);
    let error_msg = format!(
        "Failed to update component {} for entity {}",
        comp_id, entity_id
    );

    let component =
        http_utils::execute_or_exit(|| client.put::<Value, Value>(&path, &data), &error_msg).await;

    println!("Updated component:");
    cli_utils::print_formatted_or_exit(&component, output_format, "component");
}

/// Handles component deletion.
async fn handle_component_delete(
    args: &[String],
    client: &http_utils::StigmergyClient,
    _output_format: cli_utils::OutputFormat,
) {
    validate_args_count_or_exit(
        args,
        3,
        3,
        "delete",
        "Usage: stigctl component delete <entity-id> <component-id>",
    );

    let entity_id = parse_entity_id_or_exit(&args[1]);
    let comp_id = &args[2];
    let path = format!("entity/{}/component/{}", entity_id.base64_part(), comp_id);
    let error_msg = format!(
        "Failed to delete component {} for entity {}",
        comp_id, entity_id
    );

    http_utils::execute_or_exit(|| client.delete(&path), &error_msg).await;

    println!("Deleted component {} from entity {}", comp_id, entity_id);
}
