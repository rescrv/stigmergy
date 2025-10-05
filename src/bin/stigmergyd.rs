use std::path::PathBuf;
use std::sync::Arc;

use arrrg::CommandLine;
use arrrg_derive::CommandLine;
use axum::Router;
use tokio::net::TcpListener;
use tokio::signal;

use stigmergy::{
    InMemoryDataStore, SavefileManager, create_component_router, create_system_router,
};

#[derive(CommandLine, Default, PartialEq, Eq)]
struct Args {
    #[arrrg(optional, "Path to savefile for persistent state storage")]
    savefile: Option<String>,
    #[arrrg(optional, "Host to bind the HTTP server")]
    host: Option<String>,
    #[arrrg(optional, "Port to bind the HTTP server")]
    port: Option<u16>,
    #[arrrg(flag, "Enable verbose logging")]
    verbose: bool,
}

const HELP_TEXT: &str = r#"stigmergyd - Stigmergy daemon

USAGE:
    stigmergyd [OPTIONS]

OPTIONS:
    --savefile <PATH>    Path to savefile for persistent state storage [default: stigmergy.jsonl]
    --host <HOST>        Host to bind the HTTP server [default: 127.0.0.1]
    --port <PORT>        Port to bind the HTTP server [default: 8080]
    --verbose            Enable verbose logging

DESCRIPTION:
    Runs the Stigmergy daemon with entity and component management
    endpoints mounted under /api/v1/

    The server supports graceful shutdown via SIGTERM or Ctrl+C.

API ENDPOINTS:
    Entity Management:
      POST   /api/v1/entity              Create a new entity
      DELETE /api/v1/entity/{id}         Delete an entity

    System Management:
      GET    /api/v1/system              List all systems
      POST   /api/v1/system              Create a system
      POST   /api/v1/system/from-markdown Create system from markdown
      GET    /api/v1/system/{id}         Get a specific system
      PUT    /api/v1/system/{id}         Update a system
      PATCH  /api/v1/system/{id}         Patch a system
      DELETE /api/v1/system/{id}         Delete a system
      DELETE /api/v1/system              Delete all systems

    Component Definitions:
      GET    /api/v1/componentdefinition       List all definitions
      POST   /api/v1/componentdefinition       Create a definition
      GET    /api/v1/componentdefinition/{id}  Get a specific definition
      PUT    /api/v1/componentdefinition/{id}  Update a definition
      PATCH  /api/v1/componentdefinition/{id}  Patch a definition
      DELETE /api/v1/componentdefinition/{id}  Delete a definition
      DELETE /api/v1/componentdefinition       Delete all definitions

    Component Instances:
      GET    /api/v1/component       List all components
      POST   /api/v1/component       Create a component
      GET    /api/v1/component/{id}  Get a specific component
      PUT    /api/v1/component/{id}  Update a component
      PATCH  /api/v1/component/{id}  Patch a component
      DELETE /api/v1/component/{id}  Delete a component
      DELETE /api/v1/component       Delete all components"#;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (args, free) = Args::from_command_line("USAGE: stigmergyd [OPTIONS]");

    if !free.is_empty() && free[0] == "help" {
        println!("{}", HELP_TEXT);
        return Ok(());
    }

    let config = ServerConfig::from_args(args);

    if config.verbose {
        println!("Stigmergy daemon starting with configuration:");
        println!("  Savefile: {}", config.savefile_path.display());
        println!("  Bind address: {}:{}", config.host, config.port);
    }

    // Initialize savefile manager and data storage
    let logger = Arc::new(SavefileManager::new(config.savefile_path.clone()));
    let data_store = Arc::new(InMemoryDataStore::new());

    if config.verbose {
        println!("Initialized savefile manager and data store");
    }

    // Create routers
    // TODO: Entity router now requires PostgreSQL connection pool
    // let entity_router = create_entity_router(pool);
    let component_router = create_component_router(logger.clone(), data_store.clone());
    let system_router = create_system_router(logger.clone(), data_store.clone());

    let app = Router::new()
        // .nest("/api/v1", entity_router)
        .nest("/api/v1", component_router)
        .nest("/api/v1", system_router);

    // Bind to address
    let addr = format!("{}:{}", config.host, config.port);
    let listener = TcpListener::bind(&addr)
        .await
        .map_err(|e| format!("Failed to bind to {}: {}", addr, e))?;

    println!("🚀 Stigmergy daemon started successfully!");
    println!("📡 Server listening on: http://{}", addr);
    println!("💾 Savefile: {}", config.savefile_path.display());
    println!("🔄 Ready to accept API requests");

    if config.verbose {
        print_api_endpoints();
    }

    println!("💡 Use Ctrl+C or send SIGTERM for graceful shutdown");
    println!();

    // Set up graceful shutdown
    let shutdown_signal = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    // Run server with graceful shutdown
    let server = axum::serve(listener, app);

    tokio::select! {
        result = server => {
            if let Err(e) = result {
                eprintln!("❌ Server error: {}", e);
                std::process::exit(1);
            }
        }
        () = shutdown_signal => {
            println!();
            println!("🛑 Shutdown signal received, stopping server gracefully...");

            if config.verbose {
                println!("📊 Final statistics:");
                println!("   Savefile: {}", config.savefile_path.display());
                println!("   Shutdown completed successfully");
            }

            println!("👋 Stigmergy daemon stopped");
        }
    }

    Ok(())
}

struct ServerConfig {
    savefile_path: PathBuf,
    host: String,
    port: u16,
    verbose: bool,
}

impl ServerConfig {
    fn from_args(args: Args) -> Self {
        Self {
            savefile_path: args
                .savefile
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("stigmergy.jsonl")),
            host: args.host.unwrap_or_else(|| "127.0.0.1".to_string()),
            port: args.port.unwrap_or(8080),
            verbose: args.verbose,
        }
    }
}

fn print_api_endpoints() {
    println!();
    println!("📋 Available API endpoints:");
    println!();
    println!("  Entity Management:");
    println!("    POST   /api/v1/entity              Create a new entity");
    println!("    DELETE /api/v1/entity/{{id}}         Delete an entity");
    println!();
    println!("  System Management:");
    println!("    GET    /api/v1/system              List all systems");
    println!("    POST   /api/v1/system              Create a system");
    println!("    POST   /api/v1/system/from-markdown Create system from markdown");
    println!("    GET    /api/v1/system/{{id}}         Get a specific system");
    println!("    PUT    /api/v1/system/{{id}}         Update a system");
    println!("    PATCH  /api/v1/system/{{id}}         Patch a system");
    println!("    DELETE /api/v1/system/{{id}}         Delete a system");
    println!("    DELETE /api/v1/system              Delete all systems");
    println!();
    println!("  Component Definitions:");
    println!("    GET    /api/v1/componentdefinition       List all definitions");
    println!("    POST   /api/v1/componentdefinition       Create a definition");
    println!("    GET    /api/v1/componentdefinition/{{id}}  Get a specific definition");
    println!("    PUT    /api/v1/componentdefinition/{{id}}  Update a definition");
    println!("    PATCH  /api/v1/componentdefinition/{{id}}  Patch a definition");
    println!("    DELETE /api/v1/componentdefinition/{{id}}  Delete a definition");
    println!("    DELETE /api/v1/componentdefinition       Delete all definitions");
    println!();
    println!("  Component Instances:");
    println!("    GET    /api/v1/component       List all components");
    println!("    POST   /api/v1/component       Create a component");
    println!("    GET    /api/v1/component/{{id}}  Get a specific component");
    println!("    PUT    /api/v1/component/{{id}}  Update a component");
    println!("    PATCH  /api/v1/component/{{id}}  Patch a component");
    println!("    DELETE /api/v1/component/{{id}}  Delete a component");
    println!("    DELETE /api/v1/component       Delete all components");
    println!();
}
