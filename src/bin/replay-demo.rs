use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;
use stigmergy::{
    Component, ComponentDefinition, DataStore, DurableLogger, Entity, InMemoryDataStore, LogEntry,
    LogMetadata, LogOperation, OperationStatus, ValidationResult,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔄 Log Replay Demo");
    println!("=================");

    // Create a logger and data store
    let log_path = PathBuf::from("demo.jsonl");
    let logger = DurableLogger::new(log_path.clone());
    let data_store = Arc::new(InMemoryDataStore::new());

    // Create some sample operations
    println!("\n📝 Creating sample log entries...");

    // Entity creation
    let entity = Entity::new([1u8; 32]);
    let entity_log = LogEntry::new(
        LogOperation::EntityCreate {
            entity,
            was_random: false,
        },
        LogMetadata::rest_api(Some("demo".to_string())).with_status(OperationStatus::Success),
    );
    logger.log_or_error(&entity_log);
    println!("   ✓ Logged entity creation: {}", entity);

    // Component definition creation
    let component = Component::new("DemoComponent").ok_or("Failed to create component")?;
    let definition = ComponentDefinition::new(component, json!({"type": "string"}));
    let def_log = LogEntry::new(
        LogOperation::ComponentDefinitionCreate {
            definition: definition.clone(),
            validation_result: ValidationResult::Success,
        },
        LogMetadata::rest_api(Some("demo".to_string())).with_status(OperationStatus::Success),
    );
    logger.log_or_error(&def_log);
    println!(
        "   ✓ Logged component definition creation: {:?}",
        definition.component
    );

    // Component instance creation
    let comp_data = json!({"message": "Hello, World!"});
    let comp_log = LogEntry::new(
        LogOperation::ComponentCreate {
            component_id: "demo-instance".to_string(),
            component_data: comp_data.clone(),
            validation_result: Some(ValidationResult::Success),
        },
        LogMetadata::rest_api(Some("demo".to_string())).with_status(OperationStatus::Success),
    );
    logger.log_or_error(&comp_log);
    println!("   ✓ Logged component instance creation: demo-instance");

    // Also log a failed operation to demonstrate skipping
    let failed_log = LogEntry::new(
        LogOperation::EntityDelete {
            entity_id: "nonexistent".to_string(),
            success: false,
        },
        LogMetadata::rest_api(Some("demo".to_string())).with_status(OperationStatus::Failed),
    );
    logger.log_or_error(&failed_log);
    println!("   ✓ Logged failed operation (will be skipped in replay)");

    // Show the data store is currently empty
    println!("\n📊 Data store before replay:");
    println!("   Entities: {}", data_store.list_entities()?.len());
    println!(
        "   Component Definitions: {}",
        data_store.list_component_definitions()?.len()
    );
    println!(
        "   Component Instances: {}",
        data_store.list_components()?.len()
    );

    // Replay the log against the data store
    println!("\n🔄 Replaying log against data store...");
    let result = logger.replay_against_store(&*data_store)?;
    println!("   {}", result);

    // Show the data store after replay
    println!("\n📊 Data store after replay:");
    let entities = data_store.list_entities()?;
    let definitions = data_store.list_component_definitions()?;
    let components = data_store.list_components()?;

    println!("   Entities: {}", entities.len());
    for entity in &entities {
        println!("     - {}", entity);
    }

    println!("   Component Definitions: {}", definitions.len());
    for (id, def) in &definitions {
        println!("     - {}: {:?}", id, def.component);
    }

    println!("   Component Instances: {}", components.len());
    for (id, data) in &components {
        println!("     - {}: {}", id, data);
    }

    // Clean up
    std::fs::remove_file(&log_path).ok();
    println!("\n✅ Demo completed successfully!");

    Ok(())
}
