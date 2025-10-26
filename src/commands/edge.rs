//! # Edge Command Handler
//!
//! This module handles edge-related CLI commands including creation, listing,
//! and deletion of edges in the graph.

use crate::{
    CreateEdgeRequest, CreateEdgeResponse, Edge, cli_utils,
    commands::shared::{dispatch_command, parse_entity_id_or_exit, validate_args_count_or_exit},
    http_utils,
};

const EDGE_USAGE: &str = "Usage: stigctl edge <create|list|get|delete> [args...]";

/// Handles all edge-related commands.
pub async fn handle_edge_command(
    args: &[String],
    client: &http_utils::StigmergyClient,
    output_format: cli_utils::OutputFormat,
) {
    dispatch_command!("edge", EDGE_USAGE, args, client, output_format, {
        "create" => handle_edge_create,
        "list" => handle_edge_list,
        "get" => handle_edge_get,
        "delete" => handle_edge_delete,
    });
}

/// Handles edge creation command.
async fn handle_edge_create(
    args: &[String],
    client: &http_utils::StigmergyClient,
    _output_format: cli_utils::OutputFormat,
) {
    validate_args_count_or_exit(
        args,
        4,
        4,
        "create",
        "Usage: stigctl edge create <src> <dst> <label>",
    );

    let src = parse_entity_id_or_exit(&args[1]);
    let dst = parse_entity_id_or_exit(&args[2]);
    let label = parse_entity_id_or_exit(&args[3]);

    let request = CreateEdgeRequest { src, dst, label };

    let response = http_utils::execute_or_exit(
        || client.post::<CreateEdgeRequest, CreateEdgeResponse>("edge", &request),
        "Failed to create edge",
    )
    .await;

    println!(
        "Created edge: src={}, dst={}, label={}",
        response.edge.src, response.edge.dst, response.edge.label
    );
}

/// Handles edge listing command.
async fn handle_edge_list(
    args: &[String],
    client: &http_utils::StigmergyClient,
    output_format: cli_utils::OutputFormat,
) {
    let path = if args.len() == 1 {
        "edge".to_string()
    } else if args.len() == 3 && args[1] == "--from" {
        let src = parse_entity_id_or_exit(&args[2]);
        format!("edge/from/{}", src)
    } else if args.len() == 3 && args[1] == "--to" {
        let dst = parse_entity_id_or_exit(&args[2]);
        format!("edge/to/{}", dst)
    } else if args.len() == 3 && args[1] == "--labeled" {
        let label = parse_entity_id_or_exit(&args[2]);
        format!("edge/labeled/{}", label)
    } else {
        cli_utils::exit_with_error(
            "Usage: stigctl edge list [--from <src> | --to <dst> | --labeled <label>]",
        );
    };

    let edges =
        http_utils::execute_or_exit(|| client.get::<Vec<Edge>>(&path), "Failed to list edges")
            .await;

    if edges.is_empty() {
        println!("No edges found");
    } else if output_format == cli_utils::OutputFormat::Json
        || output_format == cli_utils::OutputFormat::Yaml
    {
        cli_utils::print_formatted_or_exit(&edges, output_format, "edges");
    } else {
        println!("Edges:");
        for edge in edges {
            println!(
                "  src: {}, dst: {}, label: {}",
                edge.src, edge.dst, edge.label
            );
        }
    }
}

/// Handles edge get command.
async fn handle_edge_get(
    args: &[String],
    client: &http_utils::StigmergyClient,
    output_format: cli_utils::OutputFormat,
) {
    validate_args_count_or_exit(
        args,
        4,
        4,
        "get",
        "Usage: stigctl edge get <src> <dst> <label>",
    );

    let src = parse_entity_id_or_exit(&args[1]);
    let dst = parse_entity_id_or_exit(&args[2]);
    let label = parse_entity_id_or_exit(&args[3]);

    let path = format!("edge/from/{}/to/{}/labeled/{}", src, dst, label);

    let edge =
        http_utils::execute_or_exit(|| client.get::<Edge>(&path), "Failed to get edge").await;

    if output_format == cli_utils::OutputFormat::Json
        || output_format == cli_utils::OutputFormat::Yaml
    {
        cli_utils::print_formatted_or_exit(&edge, output_format, "edge");
    } else {
        println!(
            "Edge: src={}, dst={}, label={}",
            edge.src, edge.dst, edge.label
        );
    }
}

/// Handles edge deletion command.
async fn handle_edge_delete(
    args: &[String],
    client: &http_utils::StigmergyClient,
    _output_format: cli_utils::OutputFormat,
) {
    validate_args_count_or_exit(
        args,
        4,
        4,
        "delete",
        "Usage: stigctl edge delete <src> <dst> <label>",
    );

    let src = parse_entity_id_or_exit(&args[1]);
    let dst = parse_entity_id_or_exit(&args[2]);
    let label = parse_entity_id_or_exit(&args[3]);

    let path = format!("edge/from/{}/to/{}/labeled/{}", src, dst, label);

    http_utils::execute_or_exit(|| client.delete(&path), "Failed to delete edge").await;

    println!("Deleted edge: src={}, dst={}, label={}", src, dst, label);
}
