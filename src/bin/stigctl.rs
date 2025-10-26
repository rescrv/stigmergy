use arrrg::CommandLine;
use arrrg_derive::CommandLine;

use stigmergy::{
    cli_utils::{self, OutputFormat},
    commands::{
        handle_apply_command, handle_component_command, handle_componentdefinition_command,
        handle_config_command, handle_entity_command, handle_invariant_command,
        handle_system_command,
    },
    http_utils,
};

#[derive(CommandLine, Default, PartialEq, Eq)]
struct Options {
    #[arrrg(optional, "Base URL of the Stigmergy API server")]
    base_url: String,
    #[arrrg(
        optional,
        "Output format for get/list commands: json or yaml (default: json)"
    )]
    output: OutputFormat,
}

const USAGE: &str = r#"Usage: stigctl [options] <command> [args...]

Options:
  --base-url <url>     Base URL of the Stigmergy API server (default: http://localhost:8080)
  --output <format>    Output format for get/list commands: json or yaml (default: json)

Commands:
  apply <directory>                            Apply configuration from directory
  config get                                   Get the current configuration
  config set <file.json|file.yaml>            Set configuration from file
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
  component delete <entity-id> <comp-id>       Delete a component instance from an entity
  invariant create <expression> [id]           Create an invariant
  invariant list                               List all invariants
  invariant get <invariant-id>                 Get an invariant by ID
  invariant update <invariant-id> <expression> Update an invariant
  invariant delete <invariant-id>              Delete an invariant"#;

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
        "apply" => {
            handle_apply_command(&free[1..], &client).await;
        }
        "config" => {
            handle_config_command(&free[1..], &client, options.output).await;
        }
        "entity" => {
            handle_entity_command(&free[1..], &client, options.output).await;
        }
        "system" => {
            handle_system_command(&free[1..], &client, options.output).await;
        }
        "componentdefinition" => {
            handle_componentdefinition_command(&free[1..], &client, options.output).await;
        }
        "component" => {
            handle_component_command(&free[1..], &client, options.output).await;
        }
        "invariant" => {
            handle_invariant_command(&free[1..], &client, options.output).await;
        }
        _ => {
            cli_utils::exit_with_error(&format!(
                "Unknown command '{}'. Available commands: apply, config, entity, system, componentdefinition, component, invariant",
                free[0]
            ));
        }
    }

    Ok(())
}
