//! # Apply Command Handler
//!
//! This module handles JSONL operation application commands for replaying
//! saved operations against a stigmergy API server.

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use crate::{
    CreateSystemRequest, SaveEntry, SaveOperation, cli_utils, commands::errors::HttpOperationError,
    commands::shared::validate_args_count_or_exit,
};

/// Handles the apply command for processing JSONL files.
///
/// # Arguments
/// * `args` - Command arguments (subcommand "apply" plus file paths)
/// * `base_url` - The base URL of the API server
pub async fn handle_apply_command(
    args: &[String],
    base_url: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    validate_args_count_or_exit(
        args,
        2,
        usize::MAX, // Allow unlimited files
        "apply",
        "Usage: stigctl apply <file1.jsonl> [file2.jsonl] [...]",
    );

    // Process all files provided
    for file_path in &args[1..] {
        println!("\n=== Processing {} ===", file_path);
        if let Err(e) = apply_jsonl_file(file_path, base_url).await {
            cli_utils::exit_with_error(&format!("Error processing {}: {}", file_path, e));
        }
    }

    Ok(())
}

/// Processes a single JSONL file by applying each operation.
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

        match apply_save_entry(&client, base_url, &line, line_num + 1).await {
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
        return Err(HttpOperationError::new(
            "Apply JSONL operations",
            &format!("{} operations failed", error_count),
        )
        .into());
    }

    Ok(())
}

/// Applies a single save entry by making the appropriate HTTP request.
async fn apply_save_entry(
    client: &reqwest::Client,
    base_url: &str,
    line: &str,
    line_num: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    let save_entry: SaveEntry = serde_json::from_str(line)
        .map_err(|e| format!("Failed to parse JSON at line {}: {}", line_num, e))?;

    match &save_entry.operation {
        SaveOperation::EntityCreate { entity, .. } => {
            let url = format!("{}/api/v1/entity", base_url);
            let response = client.post(&url).json(entity).send().await?;
            handle_response(response, "EntityCreate").await?;
        }
        SaveOperation::EntityDelete { entity_id, .. } => {
            let url = format!("{}/api/v1/entity/{}", base_url, entity_id);
            let response = client.delete(&url).send().await?;
            handle_response(response, "EntityDelete").await?;
        }
        SaveOperation::ComponentDefinitionCreate { definition, .. } => {
            let url = format!("{}/api/v1/componentdefinition", base_url);
            let response = client.post(&url).json(definition).send().await?;
            handle_response(response, "ComponentDefinitionCreate").await?;
        }
        SaveOperation::ComponentDefinitionUpdate {
            definition_id,
            new_definition,
            ..
        } => {
            let url = format!("{}/api/v1/componentdefinition/{}", base_url, definition_id);
            let response = client.put(&url).json(new_definition).send().await?;
            handle_response(response, "ComponentDefinitionUpdate").await?;
        }
        SaveOperation::ComponentDefinitionPatch {
            definition_id,
            patch_data,
            ..
        } => {
            let url = format!("{}/api/v1/componentdefinition/{}", base_url, definition_id);
            let response = client.patch(&url).json(patch_data).send().await?;
            handle_response(response, "ComponentDefinitionPatch").await?;
        }
        SaveOperation::ComponentDefinitionDelete { definition_id, .. } => {
            let url = format!("{}/api/v1/componentdefinition/{}", base_url, definition_id);
            let response = client.delete(&url).send().await?;
            handle_response(response, "ComponentDefinitionDelete").await?;
        }
        SaveOperation::ComponentDefinitionDeleteAll { .. } => {
            let url = format!("{}/api/v1/componentdefinition", base_url);
            let response = client.delete(&url).send().await?;
            handle_response(response, "ComponentDefinitionDeleteAll").await?;
        }
        SaveOperation::ComponentCreate {
            entity_id,
            component_id: _,
            component_data,
            ..
        } => {
            let url = format!("{}/api/v1/entity/{}/component", base_url, entity_id);
            let response = client.post(&url).json(component_data).send().await?;
            handle_response(response, "ComponentCreate").await?;
        }
        SaveOperation::ComponentUpdate {
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
        SaveOperation::ComponentPatch {
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
        SaveOperation::ComponentDelete {
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
        SaveOperation::ComponentDeleteAll { .. } => {
            let url = format!("{}/api/v1/component", base_url);
            let response = client.delete(&url).send().await?;
            handle_response(response, "ComponentDeleteAll").await?;
        }
        SaveOperation::SystemCreate { config, .. } => {
            let url = format!("{}/api/v1/system", base_url);
            if let Some(config) = config {
                let request = CreateSystemRequest {
                    config: config.clone(),
                };
                let response = client.post(&url).json(&request).send().await?;
                handle_response(response, "SystemCreate").await?;
            } else {
                return Err("SystemCreate operation missing config".into());
            }
        }
        SaveOperation::SystemUpdate {
            system_id,
            new_config,
            ..
        } => {
            let url = format!("{}/api/v1/system/{}", base_url, system_id);
            let response = client.put(&url).json(new_config).send().await?;
            handle_response(response, "SystemUpdate").await?;
        }
        SaveOperation::SystemPatch {
            system_id,
            patch_data,
            ..
        } => {
            let url = format!("{}/api/v1/system/{}", base_url, system_id);
            let response = client.patch(&url).json(patch_data).send().await?;
            handle_response(response, "SystemPatch").await?;
        }
        SaveOperation::SystemDelete { system_id, .. } => {
            let url = format!("{}/api/v1/system/{}", base_url, system_id);
            let response = client.delete(&url).send().await?;
            handle_response(response, "SystemDelete").await?;
        }
        SaveOperation::SystemDeleteAll { .. } => {
            let url = format!("{}/api/v1/system", base_url);
            let response = client.delete(&url).send().await?;
            handle_response(response, "SystemDeleteAll").await?;
        }
        SaveOperation::ComponentDefinitionGet { .. }
        | SaveOperation::ComponentGet { .. }
        | SaveOperation::SystemGet { .. } => {
            println!(
                "  Skipping read-only operation: {}",
                save_entry.operation_type()
            );
            return Ok(());
        }
        SaveOperation::ValidationPerformed { .. } => {
            println!(
                "  Skipping validation operation: {}",
                save_entry.operation_type()
            );
            return Ok(());
        }
        SaveOperation::SchemaGeneration { .. } => {
            println!(
                "  Skipping schema generation operation: {}",
                save_entry.operation_type()
            );
            return Ok(());
        }
    }

    Ok(())
}

/// Handles HTTP response status and error reporting.
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
