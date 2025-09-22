use std::path::PathBuf;
use std::sync::Arc;

use arrrg::CommandLine;
use arrrg_derive::CommandLine;
use axum::Router;
use tokio::net::TcpListener;

use stigmergy::{
    create_component_router, create_entity_router, DurableLogger, InMemoryDataStore,
};

#[derive(CommandLine, Default, PartialEq, Eq)]
struct Args {
    #[arrrg(optional, "Path to log file for durable logging")]
    log_file: Option<String>,
    #[arrrg(optional, "Host to bind the HTTP server")]
    host: Option<String>,
    #[arrrg(optional, "Port to bind the HTTP server")]
    port: Option<u16>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (args, free) = Args::from_command_line("USAGE: stigmergyd [OPTIONS]");

    if !free.is_empty() && free[0] == "help" {
        println!("stigmergyd - Stigmergy daemon");
        println!();
        println!("USAGE:");
        println!("    stigmergyd [OPTIONS]");
        println!();
        println!("OPTIONS:");
        println!("    --log-file <PATH>    Path to log file for durable logging [default: stigmergy.jsonl]");
        println!("    --host <HOST>        Host to bind the HTTP server [default: 127.0.0.1]");
        println!("    --port <PORT>        Port to bind the HTTP server [default: 8080]");
        println!();
        println!("DESCRIPTION:");
        println!("    Runs the Stigmergy daemon with entity and component management");
        println!("    endpoints mounted under /api/v1/");
        return Ok(());
    }

    let log_file = args.log_file
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("stigmergy.jsonl"));
    let host = args.host.unwrap_or_else(|| "127.0.0.1".to_string());
    let port = args.port.unwrap_or(8080);

    let logger = Arc::new(DurableLogger::new(log_file));
    let data_store = Arc::new(InMemoryDataStore::new());

    let entity_router = create_entity_router(logger.clone(), data_store.clone());
    let component_router = create_component_router(logger.clone(), data_store.clone());

    let app = Router::new()
        .nest("/api/v1", entity_router)
        .nest("/api/v1", component_router);

    let addr = format!("{}:{}", host, port);
    println!("Starting stigmergyd server on {}", addr);
    println!("Entity endpoints available at:");
    println!("  POST   /api/v1/entity");
    println!("  DELETE /api/v1/entity/{{id}}");
    println!();
    println!("Component definition endpoints available at:");
    println!("  GET    /api/v1/componentdefinition");
    println!("  POST   /api/v1/componentdefinition");
    println!("  PUT    /api/v1/componentdefinition");
    println!("  PATCH  /api/v1/componentdefinition");
    println!("  DELETE /api/v1/componentdefinition");
    println!("  GET    /api/v1/componentdefinition/{{id}}");
    println!("  PUT    /api/v1/componentdefinition/{{id}}");
    println!("  PATCH  /api/v1/componentdefinition/{{id}}");
    println!("  DELETE /api/v1/componentdefinition/{{id}}");
    println!();
    println!("Component instance endpoints available at:");
    println!("  GET    /api/v1/component");
    println!("  POST   /api/v1/component");
    println!("  PUT    /api/v1/component");
    println!("  PATCH  /api/v1/component");
    println!("  DELETE /api/v1/component");
    println!("  GET    /api/v1/component/{{id}}");
    println!("  PUT    /api/v1/component/{{id}}");
    println!("  PATCH  /api/v1/component/{{id}}");
    println!("  DELETE /api/v1/component/{{id}}");

    let listener = TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}