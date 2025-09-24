//! # System Configuration Parser
//!
//! This module provides parsing capabilities for system configuration files that use
//! frontmatter-delimited format. Configuration files contain YAML-like frontmatter
//! between `---` delimiters, followed by markdown content.
//!
//! ## File Format
//!
//! ```text
//! ---
//! name: example-system
//! description: An example system configuration
//! tools: Tool1, Tool2, Tool3
//! model: inherit
//! color: blue
//! ---
//!
//! This is the system content in markdown format.
//! It can contain multiple lines and markdown formatting.
//! ```
//!
//! ## Features
//!
//! - **Frontmatter Parsing**: Extracts key-value pairs from YAML-like headers
//! - **Content Extraction**: Preserves markdown content after frontmatter
//! - **Required Fields**: Validates presence of required configuration fields
//! - **Tool Lists**: Automatically parses comma-separated tool lists
//! - **Error Handling**: Comprehensive error reporting for malformed files
//!
//! ## Usage Examples
//!
//! ```rust
//! use stigmergy::SystemParser;
//!
//! let config_content = r#"---
//! name: my-system
//! description: A sample system configuration
//! tools: Glob, Grep, Read
//! model: inherit
//! color: green
//! ---
//!
//! # My System
//!
//! This system does amazing things.
//! "#;
//!
//! let config = SystemParser::parse(config_content).unwrap();
//! assert_eq!(config.name, "my-system");
//! assert_eq!(config.tools, vec!["Glob", "Grep", "Read"]);
//! assert_eq!(config.content.trim(), "My System\n\nThis system does amazing things.");
//! ```

use std::collections::HashMap;

/// A parsed system configuration containing metadata and content.
///
/// This structure represents a complete system configuration file,
/// with the frontmatter parsed into structured fields and the
/// remaining content preserved as markdown text.
///
/// # Fields
///
/// All fields except `content` are extracted from the frontmatter section:
/// - `name`: System identifier (required)
/// - `description`: Human-readable description (required)
/// - `tools`: List of available tools (required, comma-separated in source)
/// - `model`: Model specification (required)
/// - `color`: UI color identifier (required)
/// - `content`: Markdown content after frontmatter
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SystemConfig {
    /// The system name (required field)
    pub name: String,
    /// A description of what the system does (required field)
    pub description: String,
    /// List of tools available to the system (required field, parsed from comma-separated values)
    pub tools: Vec<String>,
    /// The model configuration (required field, often "inherit")
    pub model: String,
    /// The color theme for the system (required field)
    pub color: String,
    /// The markdown content that follows the frontmatter
    pub content: String,
}

/// Errors that can occur when parsing system configuration files.
///
/// This enum represents the different ways that system configuration parsing can fail,
/// providing specific error types for different categories of parsing problems.
#[derive(Debug)]
pub enum ParseError {
    /// The configuration file does not contain frontmatter delimited by "---"
    NoFrontmatter,
    /// A required field is missing from the frontmatter
    MissingRequiredField(String),
    /// A validation error occurred during parsing or validation
    ValidationError(String),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::NoFrontmatter => write!(f, "No header frontmatter found"),
            ParseError::MissingRequiredField(field) => {
                write!(f, "Missing required field: {}", field)
            }
            ParseError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
        }
    }
}

impl std::error::Error for ParseError {}

impl SystemConfig {
    /// Validates the system configuration against business rules and constraints.
    ///
    /// This method checks that all configuration values meet the system requirements,
    /// including field length limits, valid color values, and other constraints.
    ///
    /// # Returns
    /// * `Ok(())` - Configuration is valid
    /// * `Err(ParseError::ValidationError)` - One or more validation rules failed
    ///
    /// # Validation Rules
    /// - Name: 1-100 characters, non-empty
    /// - Description: 1-500 characters, non-empty
    /// - Color: Basic color name or hex format (#RRGGBB)
    /// - Model: Non-empty string
    /// - Content: Maximum 10KB
    /// - Tools: Each tool name 1-50 characters, non-empty
    pub fn validate(&self) -> Result<(), ParseError> {
        // Validate name length (1-100 characters)
        if self.name.is_empty() {
            return Err(ParseError::ValidationError(
                "Name cannot be empty".to_string(),
            ));
        }
        if self.name.len() > 100 {
            return Err(ParseError::ValidationError(
                "Name cannot exceed 100 characters".to_string(),
            ));
        }

        // Validate description length (1-500 characters)
        if self.description.is_empty() {
            return Err(ParseError::ValidationError(
                "Description cannot be empty".to_string(),
            ));
        }
        if self.description.len() > 500 {
            return Err(ParseError::ValidationError(
                "Description cannot exceed 500 characters".to_string(),
            ));
        }

        // Validate color (basic validation for common CSS colors)
        let valid_colors = [
            "red", "blue", "green", "yellow", "orange", "purple", "pink", "gray", "black", "white",
        ];
        if !valid_colors.contains(&self.color.as_str()) && !self.color.starts_with('#') {
            return Err(ParseError::ValidationError(format!(
                "Invalid color: {}. Use a basic color name or hex code",
                self.color
            )));
        }

        // Validate hex color format if it starts with #
        if self.color.starts_with('#')
            && (self.color.len() != 7 || !self.color[1..].chars().all(|c| c.is_ascii_hexdigit()))
        {
            return Err(ParseError::ValidationError(
                "Hex color must be in format #RRGGBB".to_string(),
            ));
        }

        // Validate model
        if self.model.is_empty() {
            return Err(ParseError::ValidationError(
                "Model cannot be empty".to_string(),
            ));
        }

        // Validate content size (max 10KB)
        if self.content.len() > 10 * 1024 {
            return Err(ParseError::ValidationError(
                "Content cannot exceed 10KB".to_string(),
            ));
        }

        // Validate tools (each tool name should be reasonable)
        for tool in &self.tools {
            if tool.is_empty() {
                return Err(ParseError::ValidationError(
                    "Tool names cannot be empty".to_string(),
                ));
            }
            if tool.len() > 50 {
                return Err(ParseError::ValidationError(format!(
                    "Tool name '{}' exceeds 50 characters",
                    tool
                )));
            }
        }

        Ok(())
    }
}

/// Parser for system configuration files with frontmatter and markdown content.
///
/// This parser handles configuration files that use Jekyll-style frontmatter format,
/// with YAML-like key-value pairs between `---` delimiters followed by markdown content.
/// It validates the presence of required fields and properly structures the data.
///
/// # Examples
///
/// ```rust
/// use stigmergy::SystemParser;
///
/// let content = r#"---
/// name: example
/// description: Example configuration
/// tools: Tool1, Tool2
/// model: gpt-4
/// color: blue
/// ---
///
/// # Example Configuration
/// This is the content section.
/// "#;
///
/// let config = SystemParser::parse(content).unwrap();
/// assert_eq!(config.name, "example");
/// assert_eq!(config.tools, vec!["Tool1", "Tool2"]);
/// ```
pub struct SystemParser;

impl SystemParser {
    /// Parses a system configuration file from its string content.
    ///
    /// This method extracts frontmatter metadata and markdown content from a
    /// configuration file, validating that all required fields are present
    /// and properly formatted.
    ///
    /// # Arguments
    /// * `content` - The full content of the configuration file
    ///
    /// # Returns
    /// * `Ok(SystemConfig)` - Successfully parsed configuration
    /// * `Err(ParseError)` - Error during parsing (missing frontmatter, required fields, etc.)
    ///
    /// # Required Fields
    /// - `name`: System identifier
    /// - `description`: Human-readable description
    /// - `tools`: Comma-separated list of available tools
    /// - `model`: Model specification
    /// - `color`: UI color identifier
    ///
    /// # Examples
    ///
    /// ```rust
    /// use stigmergy::SystemParser;
    ///
    /// let content = r#"---
    /// name: test-system
    /// description: A test system
    /// tools: Grep, Glob
    /// model: inherit
    /// color: red
    /// ---
    ///
    /// System content goes here.
    /// "#;
    ///
    /// let config = SystemParser::parse(content).unwrap();
    /// assert_eq!(config.name, "test-system");
    /// assert_eq!(config.tools.len(), 2);
    /// ```
    ///
    /// # Errors
    ///
    /// - `ParseError::NoFrontmatter` - File doesn't start with `---` or doesn't have closing `---`
    /// - `ParseError::MissingRequiredField` - One or more required fields are missing
    pub fn parse(content: &str) -> Result<SystemConfig, ParseError> {
        let (header_section, markdown_content) = Self::split_frontmatter(content)?;
        let header_data = Self::parse_header_section(&header_section)?;

        let config = SystemConfig {
            name: Self::get_required_field(&header_data, "name")?,
            description: Self::get_required_field(&header_data, "description")?,
            tools: Self::parse_tools(&header_data)?,
            model: Self::get_required_field(&header_data, "model")?,
            color: Self::get_required_field(&header_data, "color")?,
            content: markdown_content.trim().to_string(),
        };

        config.validate()?;
        Ok(config)
    }

    fn split_frontmatter(content: &str) -> Result<(String, String), ParseError> {
        let lines: Vec<&str> = content.lines().collect();

        if lines.is_empty() || lines[0] != "---" {
            return Err(ParseError::NoFrontmatter);
        }

        let mut header_end = None;
        for (i, line) in lines.iter().enumerate().skip(1) {
            if *line == "---" {
                header_end = Some(i);
                break;
            }
        }

        let header_end = header_end.ok_or(ParseError::NoFrontmatter)?;

        let header_section = lines[1..header_end].join("\n");
        let markdown_content = if header_end + 1 < lines.len() {
            lines[header_end + 1..].join("\n")
        } else {
            String::new()
        };

        Ok((header_section, markdown_content))
    }

    fn parse_header_section(headers: &str) -> Result<HashMap<String, String>, ParseError> {
        let mut data = HashMap::new();

        for line in headers.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            if let Some(colon_pos) = line.find(':') {
                let key = line[..colon_pos].trim().to_string();
                let value = line[colon_pos + 1..].trim().to_string();
                data.insert(key, value);
            }
        }

        Ok(data)
    }

    fn get_required_field(
        data: &HashMap<String, String>,
        field: &str,
    ) -> Result<String, ParseError> {
        data.get(field)
            .cloned()
            .ok_or_else(|| ParseError::MissingRequiredField(field.to_string()))
    }

    fn parse_tools(data: &HashMap<String, String>) -> Result<Vec<String>, ParseError> {
        let tools_str = Self::get_required_field(data, "tools")?;
        Ok(tools_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_system_config() {
        let content = r#"---
name: dry-principal
description: Use this agent when you need to identify and eliminate code duplication
tools: Glob, Grep, Read, Edit
model: inherit
color: purple
---

You are the DRY Principal, an expert code architect.
"#;

        let config = SystemParser::parse(content).unwrap();

        assert_eq!(config.name, "dry-principal");
        assert_eq!(
            config.description,
            "Use this agent when you need to identify and eliminate code duplication"
        );
        assert_eq!(config.tools, vec!["Glob", "Grep", "Read", "Edit"]);
        assert_eq!(config.model, "inherit");
        assert_eq!(config.color, "purple");
        assert_eq!(
            config.content,
            "You are the DRY Principal, an expert code architect."
        );
    }

    #[test]
    fn missing_frontmatter() {
        let content = "Just some markdown content";
        let result = SystemParser::parse(content);
        assert!(matches!(result, Err(ParseError::NoFrontmatter)));
    }

    #[test]
    fn missing_required_field() {
        let content = r#"---
name: dry-principal
---
Content here
"#;
        let result = SystemParser::parse(content);
        assert!(matches!(result, Err(ParseError::MissingRequiredField(_))));
    }
}
