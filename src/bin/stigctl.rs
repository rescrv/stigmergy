use arrrg::CommandLine;
use arrrg_derive::CommandLine;
use serde_json::Value;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use stigmergy::{
    ComponentDefinition, CreateEntityRequest, CreateEntityResponse, Entity, LogEntry, LogOperation,
    cli_utils, component_utils, http_utils,
};

#[derive(CommandLine, Default, PartialEq, Eq)]
struct Options {
    #[arrrg(optional, "Base URL of the Stigmergy API server")]
    base_url: String,
}

const USAGE: &str = r#"Usage: stigctl <command> [args...]

Commands:
  apply <file.jsonl>                           Apply JSONL log file operations
  entity create                                Create a new entity
  entity list                                  List all entities
  entity delete <entity-id>                    Delete an entity
  componentdefinition create <name> <schema>   Create a component definition
  componentdefinition list                     List all component definitions
  componentdefinition get <id>                 Get a component definition by ID
  componentdefinition update <id> <schema>     Update a component definition
  componentdefinition delete <id>              Delete a component definition
  component create <entity-id> <data>          Create a component instance for an entity
  component list <entity-id>                   List all component instances for an entity
  component get <entity-id> <comp-id>          Get a component instance by ID for an entity
  component update <entity-id> <comp-id> <data> Update a component instance for an entity
  component delete <entity-id> <comp-id>       Delete a component instance from an entity"#;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (options, free) = Options::from_command_line("USAGE: stigctl <command> [args...]");

    if free.is_empty() {
        cli_utils::exit_with_usage_error("No command specified", USAGE);
    }

    let base_url = if options.base_url.is_empty() {
        "http://localhost:8080".to_string()
    } else {
        options.base_url
    };

    let client = http_utils::StigmergyClient::new(base_url.clone());

    match free[0].as_str() {
        "apply" => {
            if free.len() < 2 {
                cli_utils::exit_with_usage_error(
                    "apply command requires at least one file path",
                    "Usage: stigctl apply <file1.jsonl> [file2.jsonl] [...]",
                );
            }

            // Process all files provided
            for file_path in &free[1..] {
                println!("\n=== Processing {} ===", file_path);
                if let Err(e) = apply_jsonl_file(file_path, &base_url).await {
                    cli_utils::exit_with_error(&format!("Error processing {}: {}", file_path, e));
                }
            }
        }
        "entity" => {
            if free.len() < 2 {
                cli_utils::exit_with_usage_error(
                    "entity command requires a subcommand",
                    "Usage: stigctl entity <create|list|delete> [args...]",
                );
            }
            handle_entity_command(&free[1..], &client).await;
        }
        "componentdefinition" => {
            if free.len() < 2 {
                cli_utils::exit_with_usage_error(
                    "componentdefinition command requires a subcommand",
                    "Usage: stigctl componentdefinition <create|list|get|update|delete> [args...]",
                );
            }
            handle_componentdefinition_command(&free[1..], &client).await;
        }
        "component" => {
            if free.len() < 2 {
                cli_utils::exit_with_usage_error(
                    "component command requires a subcommand",
                    "Usage: stigctl component <create|list|get|update|delete> [args...] (Note: all component operations now require an entity-id)",
                );
            }
            handle_component_command(&free[1..], &client).await;
        }
        _ => {
            cli_utils::exit_with_error(&format!(
                "Unknown command '{}'. Available commands: apply, entity, componentdefinition, component",
                free[0]
            ));
        }
    }

    Ok(())
}

async fn handle_entity_command(args: &[String], client: &http_utils::StigmergyClient) {
    match args[0].as_str() {
        "create" => {
            // Create request for random entity generation
            let request = CreateEntityRequest { entity: None };

            let response = http_utils::execute_or_exit(
                || client.post::<CreateEntityRequest, CreateEntityResponse>("entity", &request),
                "Failed to create entity",
            )
            .await;

            println!("Created entity: {}", response.entity);
        }
        "list" => {
            let entities = http_utils::execute_or_exit(
                || client.get::<Vec<Entity>>("entity"),
                "Failed to list entities",
            )
            .await;

            if entities.is_empty() {
                println!("No entities found");
            } else {
                println!("Entities:");
                for entity in entities {
                    println!("  {}", entity);
                }
            }
        }
        "delete" => {
            if args.len() < 2 {
                cli_utils::exit_with_usage_error(
                    "delete command requires an entity ID",
                    "Usage: stigctl entity delete <entity-id>",
                );
            }

            let entity_id_str = &args[1];
            let entity_id: Entity = entity_id_str.parse().unwrap_or_else(|_| {
                cli_utils::exit_with_error(&format!("Invalid entity ID format: {}", entity_id_str))
            });

            // Extract base64 part (skip "entity:" prefix) for URL path
            let base64_part = &entity_id_str[7..]; // Skip "entity:" prefix
            let path = format!("entity/{}", base64_part);
            http_utils::execute_or_exit(|| client.delete(&path), "Failed to delete entity").await;
            println!("Deleted entity: {}", entity_id);
        }
        _ => {
            cli_utils::exit_with_error(&format!(
                "Unknown entity subcommand '{}'. Available subcommands: create, list, delete",
                args[0]
            ));
        }
    }
}

async fn handle_componentdefinition_command(args: &[String], client: &http_utils::StigmergyClient) {
    match args[0].as_str() {
        "create" => {
            if args.len() < 3 {
                cli_utils::exit_with_usage_error(
                    "create command requires name and schema",
                    r#"Usage: stigctl componentdefinition create <name> <schema-json>
Example: stigctl componentdefinition create MyComponent '{"type":"object","properties":{"value":{"type":"string"}}}}'"#,
                );
            }

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
            cli_utils::print_json_or_exit(&created_definition, "component definition");
        }
        "list" => {
            let definitions = http_utils::execute_or_exit(
                || client.get::<Vec<ComponentDefinition>>("componentdefinition"),
                "Failed to list component definitions",
            )
            .await;

            cli_utils::print_json_or_exit(&definitions, "component definitions");
        }
        "get" => {
            if args.len() < 2 {
                cli_utils::exit_with_usage_error(
                    "get command requires a definition ID",
                    "Usage: stigctl componentdefinition get <id>",
                );
            }

            let def_id = &args[1];
            let path = format!("componentdefinition/{}", def_id);
            let error_msg = format!("Failed to get component definition {}", def_id);
            let definition = http_utils::execute_or_exit(
                || client.get::<ComponentDefinition>(&path),
                &error_msg,
            )
            .await;

            cli_utils::print_json_or_exit(&definition, "component definition");
        }
        "update" => {
            if args.len() < 3 {
                cli_utils::exit_with_usage_error(
                    "update command requires ID and schema",
                    "Usage: stigctl componentdefinition update <id> <schema-json>",
                );
            }

            let def_id = &args[1];
            let schema_str = &args[2];

            let schema = component_utils::parse_schema(schema_str)
                .unwrap_or_else(|e| cli_utils::exit_with_error(&e));

            // Validate schema before sending
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
            cli_utils::print_json_or_exit(&definition, "component definition");
        }
        "delete" => {
            if args.len() < 2 {
                cli_utils::exit_with_usage_error(
                    "delete command requires a definition ID",
                    "Usage: stigctl componentdefinition delete <id>",
                );
            }

            let def_id = &args[1];
            let path = format!("componentdefinition/{}", def_id);
            let error_msg = format!("Failed to delete component definition {}", def_id);
            http_utils::execute_or_exit(|| client.delete(&path), &error_msg).await;

            println!("Deleted component definition: {}", def_id);
        }
        _ => {
            cli_utils::exit_with_error(&format!(
                "Unknown componentdefinition subcommand '{}'. Available subcommands: create, list, get, update, delete",
                args[0]
            ));
        }
    }
}

async fn handle_component_command(args: &[String], client: &http_utils::StigmergyClient) {
    match args[0].as_str() {
        "create" => {
            if args.len() < 3 {
                cli_utils::exit_with_usage_error(
                    "create command requires entity-id and data",
                    r#"Usage: stigctl component create <entity-id> <data-json>
Example: stigctl component create entity:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA '{"value":"hello"}'"#,
                );
            }

            let entity_id = &args[1];
            let data_str = &args[2];
            let data = component_utils::parse_json_data(data_str)
                .unwrap_or_else(|e| cli_utils::exit_with_error(&e));

            let path = format!("entity/{}/component", entity_id);
            let component = http_utils::execute_or_exit(
                || client.post::<Value, Value>(&path, &data),
                "Failed to create component",
            )
            .await;

            println!("Created component:");
            cli_utils::print_json_or_exit(&component, "component");
        }
        "list" => {
            if args.len() < 2 {
                cli_utils::exit_with_usage_error(
                    "list command requires entity-id",
                    "Usage: stigctl component list <entity-id>",
                );
            }

            let entity_id = &args[1];
            let path = format!("entity/{}/component", entity_id);
            let components = http_utils::execute_or_exit(
                || client.get::<Vec<Value>>(&path),
                "Failed to list components",
            )
            .await;

            cli_utils::print_json_or_exit(&components, "components");
        }
        "get" => {
            if args.len() < 3 {
                cli_utils::exit_with_usage_error(
                    "get command requires entity-id and component-id",
                    "Usage: stigctl component get <entity-id> <component-id>",
                );
            }

            let entity_id = &args[1];
            let comp_id = &args[2];
            let path = format!("entity/{}/component/{}", entity_id, comp_id);
            let error_msg = format!(
                "Failed to get component {} for entity {}",
                comp_id, entity_id
            );
            let component =
                http_utils::execute_or_exit(|| client.get::<Value>(&path), &error_msg).await;

            cli_utils::print_json_or_exit(&component, "component");
        }
        "update" => {
            if args.len() < 4 {
                cli_utils::exit_with_usage_error(
                    "update command requires entity-id, component-id, and data",
                    "Usage: stigctl component update <entity-id> <component-id> <data-json>",
                );
            }

            let entity_id = &args[1];
            let comp_id = &args[2];
            let data_str = &args[3];
            let data = component_utils::parse_json_data(data_str)
                .unwrap_or_else(|e| cli_utils::exit_with_error(&e));

            let path = format!("entity/{}/component/{}", entity_id, comp_id);
            let error_msg = format!(
                "Failed to update component {} for entity {}",
                comp_id, entity_id
            );
            let component = http_utils::execute_or_exit(
                || client.put::<Value, Value>(&path, &data),
                &error_msg,
            )
            .await;

            println!("Updated component:");
            cli_utils::print_json_or_exit(&component, "component");
        }
        "delete" => {
            if args.len() < 3 {
                cli_utils::exit_with_usage_error(
                    "delete command requires entity-id and component-id",
                    "Usage: stigctl component delete <entity-id> <component-id>",
                );
            }

            let entity_id = &args[1];
            let comp_id = &args[2];
            let path = format!("entity/{}/component/{}", entity_id, comp_id);
            let error_msg = format!(
                "Failed to delete component {} for entity {}",
                comp_id, entity_id
            );
            http_utils::execute_or_exit(|| client.delete(&path), &error_msg).await;

            println!("Deleted component {} from entity {}", comp_id, entity_id);
        }
        _ => {
            cli_utils::exit_with_error(&format!(
                "Unknown component subcommand '{}'. Available subcommands: create, list, get, update, delete",
                args[0]
            ));
        }
    }
}

async fn apply_jsonl_file(
    file_path: &str,
    base_url: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let path = Path::new(file_path);
    if !path.exists() {
        return Err(format!("File not found: {}", file_path).into());
    }

    let file = File::open(path)?;
    let reader = BufReader::new(file);

    println!("Applying operations from {} to {}", file_path, base_url);

    let client = reqwest::Client::new();
    let mut success_count = 0;
    let mut error_count = 0;

    for (line_num, line) in reader.lines().enumerate() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        match apply_log_entry(&client, base_url, &line, line_num + 1).await {
            Ok(()) => {
                success_count += 1;
                println!("✓ Line {}: Applied successfully", line_num + 1);
            }
            Err(e) => {
                error_count += 1;
                eprintln!("✗ Line {}: Error - {}", line_num + 1, e);
            }
        }
    }

    println!("\nSummary:");
    println!("  Successful operations: {}", success_count);
    println!("  Failed operations: {}", error_count);
    println!("  Total operations: {}", success_count + error_count);

    if error_count > 0 {
        std::process::exit(1);
    }

    Ok(())
}

async fn apply_log_entry(
    client: &reqwest::Client,
    base_url: &str,
    line: &str,
    line_num: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    let log_entry: LogEntry = serde_json::from_str(line)
        .map_err(|e| format!("Failed to parse JSON at line {}: {}", line_num, e))?;

    match &log_entry.operation {
        LogOperation::EntityCreate { entity, .. } => {
            let url = format!("{}/api/v1/entity", base_url);
            let response = client.post(&url).json(entity).send().await?;
            handle_response(response, "EntityCreate").await?;
        }
        LogOperation::EntityDelete { entity_id, .. } => {
            let url = format!("{}/api/v1/entity/{}", base_url, entity_id);
            let response = client.delete(&url).send().await?;
            handle_response(response, "EntityDelete").await?;
        }
        LogOperation::ComponentDefinitionCreate { definition, .. } => {
            let url = format!("{}/api/v1/componentdefinition", base_url);
            let response = client.post(&url).json(definition).send().await?;
            handle_response(response, "ComponentDefinitionCreate").await?;
        }
        LogOperation::ComponentDefinitionUpdate {
            definition_id,
            new_definition,
            ..
        } => {
            let url = format!("{}/api/v1/componentdefinition/{}", base_url, definition_id);
            let response = client.put(&url).json(new_definition).send().await?;
            handle_response(response, "ComponentDefinitionUpdate").await?;
        }
        LogOperation::ComponentDefinitionPatch {
            definition_id,
            patch_data,
            ..
        } => {
            let url = format!("{}/api/v1/componentdefinition/{}", base_url, definition_id);
            let response = client.patch(&url).json(patch_data).send().await?;
            handle_response(response, "ComponentDefinitionPatch").await?;
        }
        LogOperation::ComponentDefinitionDelete { definition_id, .. } => {
            let url = format!("{}/api/v1/componentdefinition/{}", base_url, definition_id);
            let response = client.delete(&url).send().await?;
            handle_response(response, "ComponentDefinitionDelete").await?;
        }
        LogOperation::ComponentDefinitionDeleteAll { .. } => {
            let url = format!("{}/api/v1/componentdefinition", base_url);
            let response = client.delete(&url).send().await?;
            handle_response(response, "ComponentDefinitionDeleteAll").await?;
        }
        LogOperation::ComponentCreate {
            entity_id,
            component_id: _,
            component_data,
            ..
        } => {
            let url = format!("{}/api/v1/entity/{}/component", base_url, entity_id);
            let response = client.post(&url).json(component_data).send().await?;
            handle_response(response, "ComponentCreate").await?;
        }
        LogOperation::ComponentUpdate {
            entity_id,
            component_id,
            new_data,
            ..
        } => {
            let url = format!(
                "{}/api/v1/entity/{}/component/{}",
                base_url, entity_id, component_id
            );
            let response = client.put(&url).json(new_data).send().await?;
            handle_response(response, "ComponentUpdate").await?;
        }
        LogOperation::ComponentPatch {
            entity_id,
            component_id,
            patch_data,
            ..
        } => {
            let url = format!(
                "{}/api/v1/entity/{}/component/{}",
                base_url, entity_id, component_id
            );
            let response = client.patch(&url).json(patch_data).send().await?;
            handle_response(response, "ComponentPatch").await?;
        }
        LogOperation::ComponentDelete {
            entity_id,
            component_id,
            ..
        } => {
            let url = format!(
                "{}/api/v1/entity/{}/component/{}",
                base_url, entity_id, component_id
            );
            let response = client.delete(&url).send().await?;
            handle_response(response, "ComponentDelete").await?;
        }
        LogOperation::ComponentDeleteAll { .. } => {
            let url = format!("{}/api/v1/component", base_url);
            let response = client.delete(&url).send().await?;
            handle_response(response, "ComponentDeleteAll").await?;
        }
        LogOperation::ComponentDefinitionGet { .. } | LogOperation::ComponentGet { .. } => {
            println!(
                "  Skipping read-only operation: {}",
                log_entry.operation_type()
            );
            return Ok(());
        }
        LogOperation::ValidationPerformed { .. } => {
            println!(
                "  Skipping validation operation: {}",
                log_entry.operation_type()
            );
            return Ok(());
        }
        LogOperation::SchemaGeneration { .. } => {
            println!(
                "  Skipping schema generation operation: {}",
                log_entry.operation_type()
            );
            return Ok(());
        }
    }

    Ok(())
}

async fn handle_response(
    response: reqwest::Response,
    operation_type: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let status = response.status();

    if status.is_success() {
        Ok(())
    } else {
        let error_body = response.text().await.unwrap_or_default();
        Err(format!(
            "{} failed with status {}: {}",
            operation_type,
            status,
            if error_body.is_empty() {
                "No error details"
            } else {
                &error_body
            }
        )
        .into())
    }
}
