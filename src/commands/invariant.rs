//! # Invariant Command Handler
//!
//! This module handles invariant-related CLI commands including creation, listing,
//! retrieval, updating, and deletion of invariants.

use crate::{
    GetInvariantResponse, InvariantID, cli_utils,
    commands::shared::{dispatch_command, validate_args_count_or_exit},
    http_utils,
};

const INVARIANT_USAGE: &str = "Usage: stigctl invariant <create|list|get|update|delete> [args...]";

/// Handles all invariant-related commands.
///
/// # Arguments
/// * `args` - Command arguments (first element is the subcommand)
/// * `client` - HTTP client for API communication
/// * `output_format` - Output format for get/list commands
pub async fn handle_invariant_command(
    args: &[String],
    client: &http_utils::StigmergyClient,
    output_format: cli_utils::OutputFormat,
) {
    dispatch_command!("invariant", INVARIANT_USAGE, args, client, output_format, {
        "create" => handle_invariant_create,
        "list" => handle_invariant_list,
        "get" => handle_invariant_get,
        "update" => handle_invariant_update,
        "delete" => handle_invariant_delete,
    });
}

/// Handles invariant creation.
async fn handle_invariant_create(
    args: &[String],
    client: &http_utils::StigmergyClient,
    _output_format: cli_utils::OutputFormat,
) {
    validate_args_count_or_exit(
        args,
        2,
        3,
        "create",
        r#"Usage: stigctl invariant create <assertion> [invariant-id]
Example: stigctl invariant create "x > 0 && y < 100"
Example: stigctl invariant create "x > 0" invariant:AAAA..."#,
    );

    let asserts = &args[1];
    let invariant_id = if args.len() >= 3 {
        Some(
            args[2]
                .parse::<InvariantID>()
                .unwrap_or_else(|_| cli_utils::exit_with_error("Invalid invariant ID")),
        )
    } else {
        None
    };

    let request = crate::CreateInvariantRequest {
        invariant_id,
        asserts: asserts.to_string(),
    };

    let created = http_utils::execute_or_exit(
        || {
            client.post::<crate::CreateInvariantRequest, crate::CreateInvariantResponse>(
                "invariant",
                &request,
            )
        },
        "Failed to create invariant",
    )
    .await;

    println!("Created invariant: {}", created.invariant_id);
    println!("Assertion: {}", created.asserts);
}

/// Handles invariant listing.
async fn handle_invariant_list(
    args: &[String],
    client: &http_utils::StigmergyClient,
    output_format: cli_utils::OutputFormat,
) {
    validate_args_count_or_exit(args, 1, 1, "list", "Usage: stigctl invariant list");

    let invariants = http_utils::execute_or_exit(
        || client.get::<Vec<GetInvariantResponse>>("invariant"),
        "Failed to list invariants",
    )
    .await;

    if invariants.is_empty() {
        println!("No invariants found");
    } else if output_format == cli_utils::OutputFormat::Json
        || output_format == cli_utils::OutputFormat::Yaml
    {
        cli_utils::print_formatted_or_exit(&invariants, output_format, "invariants");
    } else {
        for inv in invariants {
            println!("ID:        {}", inv.invariant_id);
            println!("Asserts:   {}", inv.asserts);
            println!("Created:   {}", inv.created_at);
            println!("Updated:   {}", inv.updated_at);
            println!();
        }
    }
}

/// Handles invariant retrieval by ID.
async fn handle_invariant_get(
    args: &[String],
    client: &http_utils::StigmergyClient,
    output_format: cli_utils::OutputFormat,
) {
    validate_args_count_or_exit(
        args,
        2,
        2,
        "get",
        "Usage: stigctl invariant get <invariant-id>",
    );

    let invariant_id_str = &args[1];
    let invariant_id = invariant_id_str
        .parse::<InvariantID>()
        .unwrap_or_else(|_| cli_utils::exit_with_error("Invalid invariant ID"));

    let path = format!("invariant/{}", invariant_id.base64_part());
    let error_msg = format!("Failed to get invariant {}", invariant_id);

    let invariant =
        http_utils::execute_or_exit(|| client.get::<GetInvariantResponse>(&path), &error_msg).await;

    if output_format == cli_utils::OutputFormat::Json
        || output_format == cli_utils::OutputFormat::Yaml
    {
        cli_utils::print_formatted_or_exit(&invariant, output_format, "invariant");
    } else {
        println!("ID:        {}", invariant.invariant_id);
        println!("Asserts:   {}", invariant.asserts);
        println!("Created:   {}", invariant.created_at);
        println!("Updated:   {}", invariant.updated_at);
    }
}

/// Handles invariant update.
async fn handle_invariant_update(
    args: &[String],
    client: &http_utils::StigmergyClient,
    output_format: cli_utils::OutputFormat,
) {
    validate_args_count_or_exit(
        args,
        3,
        3,
        "update",
        "Usage: stigctl invariant update <invariant-id> <assertion>",
    );

    let invariant_id_str = &args[1];
    let invariant_id = invariant_id_str
        .parse::<InvariantID>()
        .unwrap_or_else(|_| cli_utils::exit_with_error("Invalid invariant ID"));

    let asserts = &args[2];

    let request = crate::UpdateInvariantRequest {
        asserts: asserts.to_string(),
    };

    let path = format!("invariant/{}", invariant_id.base64_part());
    let error_msg = format!("Failed to update invariant {}", invariant_id);

    let invariant = http_utils::execute_or_exit(
        || client.put::<crate::UpdateInvariantRequest, GetInvariantResponse>(&path, &request),
        &error_msg,
    )
    .await;

    if output_format == cli_utils::OutputFormat::Json
        || output_format == cli_utils::OutputFormat::Yaml
    {
        println!("Updated invariant:");
        cli_utils::print_formatted_or_exit(&invariant, output_format, "invariant");
    } else {
        println!("Updated invariant:");
        println!("ID:        {}", invariant.invariant_id);
        println!("Asserts:   {}", invariant.asserts);
        println!("Created:   {}", invariant.created_at);
        println!("Updated:   {}", invariant.updated_at);
    }
}

/// Handles invariant deletion.
async fn handle_invariant_delete(
    args: &[String],
    client: &http_utils::StigmergyClient,
    _output_format: cli_utils::OutputFormat,
) {
    validate_args_count_or_exit(
        args,
        2,
        2,
        "delete",
        "Usage: stigctl invariant delete <invariant-id>",
    );

    let invariant_id_str = &args[1];
    let invariant_id = invariant_id_str
        .parse::<InvariantID>()
        .unwrap_or_else(|_| cli_utils::exit_with_error("Invalid invariant ID"));

    let path = format!("invariant/{}", invariant_id.base64_part());
    let error_msg = format!("Failed to delete invariant {}", invariant_id);

    http_utils::execute_or_exit(|| client.delete(&path), &error_msg).await;

    println!("Deleted invariant: {}", invariant_id);
}
