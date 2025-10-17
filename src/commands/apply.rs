//! # Apply Command Handler
//!
//! This module handles the `stigctl apply` command for applying configurations
//! from directories containing entity, component, system, and invariant definitions.
//!
//! ## Supported Formats
//!
//! - **YAML files**: Multi-document YAML files with `.yaml` or `.yml` extensions
//! - **Markdown files**: System definitions with YAML frontmatter (`.md` extension)
//!
//! ## Directory Structure
//!
//! The apply command expects the following directory structure:
//! ```text
//! foo/
//! ├── entity/               # Entity definitions (YAML)
//! ├── component_definition/ # Component definition schemas (YAML)
//! ├── component/            # Component instance definitions (YAML)
//! ├── system/               # System definitions (Markdown with YAML frontmatter)
//! └── invariant/            # Invariant definitions (YAML)
//! ```
//!
//! ## Operation Types
//!
//! The command converts files into batch operations that are sent to the `/api/v1/apply` endpoint:
//! - Entity creation/deletion
//! - Component definition upsertion/deletion
//! - Component upsertion/deletion
//! - System creation (from markdown)
//! - Invariant creation/deletion

use crate::{
    Component, ComponentDefinition, Entity, InvariantID,
    apply::{ApplyRequest, ApplyResponse, Operation, OperationResult},
    cli_utils,
    http_utils::StigmergyClient,
};
use serde::Deserialize;
use serde_json;
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;

/// Error types for apply command operations
#[derive(Debug)]
#[allow(clippy::enum_variant_names)]
enum ApplyError {
    IoError(std::io::Error),
    YamlError(serde_yml::Error),
    JsonError(serde_json::Error),
    ParseError(String),
    HttpError(Box<dyn std::error::Error>),
}

impl std::fmt::Display for ApplyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApplyError::IoError(e) => write!(f, "IO error: {}", e),
            ApplyError::YamlError(e) => write!(f, "YAML parsing error: {}", e),
            ApplyError::JsonError(e) => write!(f, "JSON error: {}", e),
            ApplyError::ParseError(e) => write!(f, "Parse error: {}", e),
            ApplyError::HttpError(e) => write!(f, "HTTP error: {}", e),
        }
    }
}

impl std::error::Error for ApplyError {}

impl From<std::io::Error> for ApplyError {
    fn from(e: std::io::Error) -> Self {
        ApplyError::IoError(e)
    }
}

impl From<serde_yml::Error> for ApplyError {
    fn from(e: serde_yml::Error) -> Self {
        ApplyError::YamlError(e)
    }
}

impl From<serde_json::Error> for ApplyError {
    fn from(e: serde_json::Error) -> Self {
        ApplyError::JsonError(e)
    }
}

/// Parsed representation of a system from markdown with YAML frontmatter
#[derive(Debug, serde::Deserialize, serde::Serialize)]
// NOTE(rescrv): deserialize and debug are ignored for deadcode, so this is dead.
#[allow(dead_code)]
struct SystemDefinition {
    name: String,
    description: String,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    color: Option<String>,
    #[serde(default)]
    component: Vec<String>,
    #[serde(default)]
    bid: Vec<String>,
}

/// Parsed representation of a component instance
#[derive(Debug, serde::Deserialize)]
struct ComponentInstance {
    entity: Entity,
    component: String,
    data: serde_json::Value,
}

/// Parsed representation of a component definition from YAML
#[derive(Debug, serde::Deserialize)]
struct ComponentDefinitionYaml {
    component: String,
    schema: serde_json::Value,
}

/// Parsed representation of an invariant
#[derive(Debug, serde::Deserialize)]
struct InvariantYaml {
    asserts: String,
    #[serde(default)]
    invariant_id: Option<String>,
}

/// Main entry point for the apply command
pub async fn handle_apply_command(args: &[String], client: &StigmergyClient) {
    if args.is_empty() {
        cli_utils::exit_with_usage_error("No directory specified", get_apply_usage());
    }

    let directory = &args[0];
    let path = Path::new(directory);

    if !path.exists() {
        cli_utils::exit_with_error(&format!("Directory '{}' does not exist", directory));
    }

    if !path.is_dir() {
        cli_utils::exit_with_error(&format!("'{}' is not a directory", directory));
    }

    match process_directory(path, client).await {
        Ok(()) => {
            println!("✓ Successfully applied configuration from '{}'", directory);
        }
        Err(e) => {
            cli_utils::exit_with_error(&format!("Failed to apply configuration: {}", e));
        }
    }
}

/// Get usage information for the apply command
fn get_apply_usage() -> &'static str {
    r#"Usage: stigctl apply <directory>

Applies configuration from a directory containing entity, component, system, and invariant definitions.

Directory structure:
  <directory>/
    entity/               # Entity definitions (YAML)
    component_definition/ # Component definition schemas (YAML)
    component/            # Component instance definitions (YAML) 
    system/               # System definitions (Markdown with YAML frontmatter)
    invariant/            # Invariant definitions (YAML)

Supported file formats:
  - YAML files (.yaml, .yml): Multi-document YAML files
  - Markdown files (.md): System definitions with YAML frontmatter

Examples:
  stigctl apply foo/
  stigctl apply ./config/"#
}

/// Process a directory and all its subdirectories
async fn process_directory(path: &Path, client: &StigmergyClient) -> Result<(), ApplyError> {
    let mut operations = Vec::new();

    // Process each subdirectory in the expected order
    let entity_path = path.join("entity");
    if entity_path.exists() {
        println!("Processing entities from '{}'...", entity_path.display());
        process_entity_directory(&entity_path, &mut operations)?;
    }

    let component_definition_path = path.join("component_definition");
    if component_definition_path.exists() {
        println!(
            "Processing component definitions from '{}'...",
            component_definition_path.display()
        );
        process_component_definition_directory(&component_definition_path, &mut operations)?;
    }

    let component_path = path.join("component");
    if component_path.exists() {
        println!(
            "Processing components from '{}'...",
            component_path.display()
        );
        process_component_directory(&component_path, &mut operations)?;
    }

    let system_path = path.join("system");
    if system_path.exists() {
        println!("Processing systems from '{}'...", system_path.display());
        process_system_directory(&system_path, &mut operations)?;
    }

    let invariant_path = path.join("invariant");
    if invariant_path.exists() {
        println!(
            "Processing invariants from '{}'...",
            invariant_path.display()
        );
        process_invariant_directory(&invariant_path, &mut operations)?;
    }

    if operations.is_empty() {
        println!("No configuration files found in directory");
        return Ok(());
    }

    println!("Applying {} operations...", operations.len());

    let request = ApplyRequest { operations };
    let response: ApplyResponse = client
        .post("apply", &request)
        .await
        .map_err(ApplyError::HttpError)?;

    print_apply_results(&response);

    if !response.committed {
        return Err(ApplyError::ParseError(
            "Transaction rolled back due to errors".to_string(),
        ));
    }

    Ok(())
}

/// Process entity directory - creates entities
fn process_entity_directory(
    path: &Path,
    operations: &mut Vec<Operation>,
) -> Result<(), ApplyError> {
    let yaml_files = find_yaml_files(path)?;

    for file_path in yaml_files {
        let content = fs::read_to_string(&file_path)?;

        // Try to parse as an array of entities first
        if let Ok(entities) = serde_yml::from_str::<Vec<String>>(&content) {
            for entity_str in entities {
                if let Ok(entity) = Entity::from_str(&entity_str) {
                    operations.push(Operation::CreateEntity {
                        entity: Some(entity),
                    });
                } else {
                    return Err(ApplyError::ParseError(format!(
                        "Invalid entity ID in file: {:?}",
                        file_path
                    )));
                }
            }
        } else {
            // Fall back to multi-document YAML
            for doc in serde_yml::Deserializer::from_str(&content) {
                let entity_value = serde_yml::Value::deserialize(doc)?;

                // Convert to JSON value for easier processing
                let entity_json = serde_json::to_value(&entity_value)?;

                // If it's just an entity ID, use it directly
                if let Some(entity_str) = entity_json.as_str() {
                    if let Ok(entity) = Entity::from_str(entity_str) {
                        operations.push(Operation::CreateEntity {
                            entity: Some(entity),
                        });
                    } else {
                        return Err(ApplyError::ParseError(format!(
                            "Invalid entity ID in file: {:?}",
                            file_path
                        )));
                    }
                } else if let Some(entity_obj) = entity_json.as_object() {
                    // Check for 'entity' field
                    if let Some(entity_val) = entity_obj.get("entity") {
                        if let Some(entity_str) = entity_val.as_str() {
                            if let Ok(entity) = Entity::from_str(entity_str) {
                                operations.push(Operation::CreateEntity {
                                    entity: Some(entity),
                                });
                            } else {
                                return Err(ApplyError::ParseError(format!(
                                    "Invalid entity ID in file: {:?}",
                                    file_path
                                )));
                            }
                        }
                    } else {
                        // Generate random entity
                        operations.push(Operation::CreateEntity { entity: None });
                    }
                }
            }
        }
    }

    Ok(())
}

/// Process component definition directory - creates/updates component definitions
fn process_component_definition_directory(
    path: &Path,
    operations: &mut Vec<Operation>,
) -> Result<(), ApplyError> {
    let yaml_files = find_yaml_files(path)?;

    for file_path in yaml_files {
        let content = fs::read_to_string(&file_path)?;

        if let Ok(definitions) = serde_yml::from_str::<Vec<ComponentDefinitionYaml>>(&content) {
            for def_yaml in definitions {
                let component = Component::new(&def_yaml.component).ok_or_else(|| {
                    ApplyError::ParseError(format!(
                        "Invalid component name: {}",
                        def_yaml.component
                    ))
                })?;

                let definition = ComponentDefinition::new(component, def_yaml.schema);

                operations.push(Operation::UpsertComponentDefinition { definition });
            }
        } else {
            for doc in serde_yml::Deserializer::from_str(&content) {
                let def_yaml: ComponentDefinitionYaml = ComponentDefinitionYaml::deserialize(doc)?;
                let component = Component::new(&def_yaml.component).ok_or_else(|| {
                    ApplyError::ParseError(format!(
                        "Invalid component name: {}",
                        def_yaml.component
                    ))
                })?;

                let definition = ComponentDefinition::new(component, def_yaml.schema);

                operations.push(Operation::UpsertComponentDefinition { definition });
            }
        }
    }

    Ok(())
}

/// Process component directory - creates/updates component instances
fn process_component_directory(
    path: &Path,
    operations: &mut Vec<Operation>,
) -> Result<(), ApplyError> {
    let yaml_files = find_yaml_files(path)?;

    for file_path in yaml_files {
        let content = fs::read_to_string(&file_path)?;

        // Try to parse as an array of components first
        if let Ok(components) = serde_yml::from_str::<Vec<ComponentInstance>>(&content) {
            for comp_instance in components {
                let component = Component::new(&comp_instance.component).ok_or_else(|| {
                    ApplyError::ParseError(format!(
                        "Invalid component name: {}",
                        comp_instance.component
                    ))
                })?;

                operations.push(Operation::UpsertComponent {
                    entity: comp_instance.entity,
                    component,
                    data: comp_instance.data,
                });
            }
        } else {
            // Fall back to multi-document YAML
            for doc in serde_yml::Deserializer::from_str(&content) {
                let comp_instance: ComponentInstance = ComponentInstance::deserialize(doc)?;
                let component = Component::new(&comp_instance.component).ok_or_else(|| {
                    ApplyError::ParseError(format!(
                        "Invalid component name: {}",
                        comp_instance.component
                    ))
                })?;

                operations.push(Operation::UpsertComponent {
                    entity: comp_instance.entity,
                    component,
                    data: comp_instance.data,
                });
            }
        }
    }

    Ok(())
}

/// Process system directory - creates systems from markdown files
fn process_system_directory(
    path: &Path,
    _operations: &mut Vec<Operation>,
) -> Result<(), ApplyError> {
    let md_files = find_markdown_files(path)?;

    for file_path in md_files {
        let content = fs::read_to_string(&file_path)?;
        let system_def = parse_markdown_system(&content)?;

        // For now, we'll just print the system definition
        // TODO: Add system creation operation when the API supports it
        println!(
            "  Found system: {} (system creation not yet supported)",
            system_def.name
        );
    }

    Ok(())
}

/// Process invariant directory - creates invariants
fn process_invariant_directory(
    path: &Path,
    operations: &mut Vec<Operation>,
) -> Result<(), ApplyError> {
    let yaml_files = find_yaml_files(path)?;

    for file_path in yaml_files {
        let content = fs::read_to_string(&file_path)?;

        if let Ok(invariants) = serde_yml::from_str::<Vec<InvariantYaml>>(&content) {
            for inv_yaml in invariants {
                let invariant_id = if let Some(id_str) = inv_yaml.invariant_id {
                    Some(InvariantID::from_str(&id_str).map_err(|_| {
                        ApplyError::ParseError(format!("Invalid invariant ID: {}", id_str))
                    })?)
                } else {
                    None
                };

                operations.push(Operation::CreateInvariant {
                    invariant_id,
                    asserts: inv_yaml.asserts,
                });
            }
        } else {
            for doc in serde_yml::Deserializer::from_str(&content) {
                let inv_yaml: InvariantYaml = InvariantYaml::deserialize(doc)?;

                let invariant_id = if let Some(id_str) = inv_yaml.invariant_id {
                    Some(InvariantID::from_str(&id_str).map_err(|_| {
                        ApplyError::ParseError(format!("Invalid invariant ID: {}", id_str))
                    })?)
                } else {
                    None
                };

                operations.push(Operation::CreateInvariant {
                    invariant_id,
                    asserts: inv_yaml.asserts,
                });
            }
        }
    }

    Ok(())
}

/// Find all YAML files in a directory
fn find_yaml_files(path: &Path) -> Result<Vec<PathBuf>, ApplyError> {
    let mut files = Vec::new();

    if path.is_dir() {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let file_path = entry.path();

            if file_path.is_file() {
                let extension = file_path
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .unwrap_or("");

                if extension == "yaml" || extension == "yml" {
                    files.push(file_path);
                }
            }
        }
    }

    files.sort();
    Ok(files)
}

/// Find all Markdown files in a directory
fn find_markdown_files(path: &Path) -> Result<Vec<PathBuf>, ApplyError> {
    let mut files = Vec::new();

    if path.is_dir() {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let file_path = entry.path();

            if file_path.is_file() {
                let extension = file_path
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .unwrap_or("");

                if extension == "md" {
                    files.push(file_path);
                }
            }
        }
    }

    files.sort();
    Ok(files)
}

/// Parse markdown file with YAML frontmatter into a SystemDefinition
fn parse_markdown_system(content: &str) -> Result<SystemDefinition, ApplyError> {
    // Simple YAML frontmatter extraction
    let frontmatter_start = content.find("---\n");
    let frontmatter_end = content[4..].find("\n---\n").map(|pos| pos + 4);

    if let (Some(start), Some(end)) = (frontmatter_start, frontmatter_end) {
        let yaml_content = &content[start + 4..end];
        let system_def: SystemDefinition = serde_yml::from_str(yaml_content)?;
        Ok(system_def)
    } else {
        Err(ApplyError::ParseError(
            "No YAML frontmatter found in markdown file".to_string(),
        ))
    }
}

/// Print the results of the apply operation
fn print_apply_results(response: &ApplyResponse) {
    let mut created_entities = 0;
    let mut created_component_definitions = 0;
    let mut updated_component_definitions = 0;
    let mut updated_components = 0;
    let mut created_components = 0;
    let mut created_invariants = 0;
    let mut errors = 0;

    for result in &response.results {
        match result {
            OperationResult::CreateEntity { created, .. } => {
                if *created {
                    created_entities += 1;
                }
            }
            OperationResult::UpsertComponentDefinition { created, .. } => {
                if *created {
                    created_component_definitions += 1;
                } else {
                    updated_component_definitions += 1;
                }
            }
            OperationResult::UpsertComponent { created, .. } => {
                if *created {
                    created_components += 1;
                } else {
                    updated_components += 1;
                }
            }
            OperationResult::CreateInvariant { .. } => {
                created_invariants += 1;
            }
            OperationResult::Error { .. } => {
                errors += 1;
            }
            _ => {}
        }
    }

    println!("Results:");
    if created_entities > 0 {
        println!("  ✓ Created {} entities", created_entities);
    }
    if created_component_definitions > 0 {
        println!(
            "  ✓ Created {} component definitions",
            created_component_definitions
        );
    }
    if updated_component_definitions > 0 {
        println!(
            "  ✓ Updated {} component definitions",
            updated_component_definitions
        );
    }
    if created_components > 0 {
        println!("  ✓ Created {} components", created_components);
    }
    if updated_components > 0 {
        println!("  ✓ Updated {} components", updated_components);
    }
    if created_invariants > 0 {
        println!("  ✓ Created {} invariants", created_invariants);
    }
    if errors > 0 {
        println!("  ✗ {} errors occurred", errors);

        for result in response.results.iter() {
            if let OperationResult::Error {
                operation_index,
                error,
            } = result
            {
                println!("    Operation {}: {}", operation_index + 1, error);
            }
        }
    }

    if response.committed {
        println!("  ✓ Transaction committed successfully");
    } else {
        println!("  ✗ Transaction rolled back");
    }
}
