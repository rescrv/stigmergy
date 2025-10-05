use arrrg::CommandLine;
use arrrg_derive::CommandLine;

use stigmergy::{
    cli_utils,
    commands::{
        handle_component_command, handle_componentdefinition_command, handle_entity_command,
        handle_system_command,
    },
    http_utils,
};

#[derive(CommandLine, Default, PartialEq, Eq)]
struct Options {
    #[arrrg(optional, "Base URL of the Stigmergy API server")]
    base_url: String,
}

const USAGE: &str = r#"Usage: stigctl <command> [args...]

Commands:
  entity create                                Create a new entity
  entity list                                  List all entities
  entity delete <entity-id>                    Delete an entity
  system create <config-json>                 Create a system from config
  system create-from-md <file.md>             Create a system from markdown file
  system list                                  List all systems
  system get <system-id>                       Get a system by ID
  system update <system-id> <config-json>     Update a system
  system delete <system-id>                   Delete a system
  componentdefinition create <name> <schema>   Create a component definition
  componentdefinition list                     List all component definitions
  componentdefinition get <id>                 Get a component definition by ID
  componentdefinition update <id> <schema>     Update a component definition
  componentdefinition delete <id>              Delete a component definition
  component create <entity> <component> <data> Create a component instance for an entity
  component list <entity-id>                   List all component instances for an entity
  component get <entity-id> <comp-id>          Get a component instance by ID for an entity
  component update <entity-id> <comp-id> <data> Update a component instance for an entity
  component delete <entity-id> <comp-id>       Delete a component instance from an entity"#;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (options, free) = Options::from_command_line_relaxed("USAGE: stigctl <command> [args...]");

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
        "entity" => {
            handle_entity_command(&free[1..], &client).await;
        }
        "system" => {
            handle_system_command(&free[1..], &client).await;
        }
        "componentdefinition" => {
            handle_componentdefinition_command(&free[1..], &client).await;
        }
        "component" => {
            handle_component_command(&free[1..], &client).await;
        }
        _ => {
            cli_utils::exit_with_error(&format!(
                "Unknown command '{}'. Available commands: entity, system, componentdefinition, component",
                free[0]
            ));
        }
    }

    Ok(())
}
