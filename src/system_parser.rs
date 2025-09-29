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

use crate::{Bid, BidParseError, BidParser};

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
/// - `bid`: List of bid expressions (optional, parsed from bullet list format)
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
    /// List of bid expressions (optional field, parsed from bullet list format)
    pub bid: Vec<Bid>,
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
    /// A bid expression failed to parse
    BidParseError(String, BidParseError),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::NoFrontmatter => write!(f, "No header frontmatter found"),
            ParseError::MissingRequiredField(field) => {
                write!(f, "Missing required field: {}", field)
            }
            ParseError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            ParseError::BidParseError(bid_str, bid_err) => {
                write!(
                    f,
                    "Failed to parse bid expression '{}': {}",
                    bid_str, bid_err
                )
            }
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
    /// - Bid: Maximum 100 bid expressions
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

        // Validate bid expressions (reasonable limit on count)
        if self.bid.len() > 100 {
            return Err(ParseError::ValidationError(
                "Cannot have more than 100 bid expressions".to_string(),
            ));
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
            bid: Self::parse_bid(&header_data)?,
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
        let lines: Vec<&str> = headers.lines().collect();
        let mut i = 0;

        while i < lines.len() {
            let line = lines[i].trim();
            if line.is_empty() {
                i += 1;
                continue;
            }

            if let Some(colon_pos) = line.find(':') {
                let key = line[..colon_pos].trim().to_string();
                let mut value = line[colon_pos + 1..].trim().to_string();

                // Handle multi-line bid field specially
                if key == "bid" && value.is_empty() {
                    // Collect all the bullet list lines that follow
                    i += 1;
                    let mut bid_lines = Vec::new();
                    while i < lines.len() {
                        let bid_line = lines[i].trim();
                        if bid_line.is_empty() {
                            i += 1;
                            continue;
                        }
                        // Check if this looks like a bullet point (with optional whitespace before -)
                        if bid_line.starts_with('-')
                            || (bid_line.len() > 1 && bid_line.trim_start().starts_with('-'))
                        {
                            bid_lines.push(bid_line.to_string());
                            i += 1;
                        } else if bid_line.contains(':') {
                            // This is likely the start of the next field
                            break;
                        } else {
                            // Continue collecting lines that might be part of the multi-line bid
                            bid_lines.push(bid_line.to_string());
                            i += 1;
                        }
                    }
                    value = bid_lines.join("\n");
                    i -= 1; // Back up one so we don't skip the next field
                }

                data.insert(key, value);
            }
            i += 1;
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

    fn parse_bid(data: &HashMap<String, String>) -> Result<Vec<Bid>, ParseError> {
        // bid field is optional
        if let Some(bid_str) = data.get("bid") {
            if bid_str.trim().is_empty() {
                return Ok(Vec::new());
            }

            let mut bids = Vec::new();
            for line in bid_str.lines() {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }

                // Extract the bid expression from the bullet point
                // Handle various bullet formats: "- expr", "  - expr", etc.
                let bid_expr = if let Some(stripped) = line.strip_prefix('-') {
                    stripped.trim()
                } else if let Some(dash_pos) = line.find('-') {
                    // Handle whitespace-prefixed bullets
                    line[dash_pos + 1..].trim()
                } else {
                    // No bullet found, treat the whole line as a bid expression
                    line
                };

                if !bid_expr.is_empty() {
                    match BidParser::parse(bid_expr) {
                        Ok(bid) => bids.push(bid),
                        Err(err) => {
                            return Err(ParseError::BidParseError(bid_expr.to_string(), err));
                        }
                    }
                }
            }
            Ok(bids)
        } else {
            Ok(Vec::new())
        }
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
        assert_eq!(config.bid, Vec::<Bid>::new()); // No bid field in test content, so empty
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

    #[test]
    fn parse_system_config_with_bid() {
        let content = r#"---
name: example-system
description: A system with bid expressions
tools: Glob, Grep
model: inherit
color: blue
bid:
- ON true BID 100
- ON score > 50 BID score * 2
  - ON active BID "active_bonus"
---

System content with bids.
"#;

        let config = SystemParser::parse(content).unwrap();

        assert_eq!(config.name, "example-system");
        assert_eq!(config.bid.len(), 3);

        // Verify the first bid parses correctly
        let first_bid_str = format!("{}", config.bid[0]);
        assert!(first_bid_str.contains("true"));
        assert!(first_bid_str.contains("100"));
    }

    #[test]
    fn parse_system_config_with_whitespace_tolerant_bullets() {
        let content = r#"---
name: test-system
description: Testing whitespace-tolerant bullets
tools: Read
model: inherit
color: green
bid:
  - ON price > 100 BID price * 0.9
    - ON discount_eligible BID original_price * 0.8
      - ON vip_member BID base_price / 2
---

Content here.
"#;

        let config = SystemParser::parse(content).unwrap();
        assert_eq!(config.bid.len(), 3);
    }

    #[test]
    fn parse_system_config_empty_bid() {
        let content = r#"---
name: empty-bid
description: System with empty bid section
tools: Read
model: inherit
color: red
bid:
---

Content here.
"#;

        let config = SystemParser::parse(content).unwrap();
        assert_eq!(config.bid.len(), 0);
    }

    #[test]
    fn parse_system_config_no_bid_field() {
        let content = r#"---
name: no-bid
description: System without bid field
tools: Read
model: inherit
color: yellow
---

Content here.
"#;

        let config = SystemParser::parse(content).unwrap();
        assert_eq!(config.bid.len(), 0);
    }

    #[test]
    fn invalid_bid_expression() {
        let content = r#"---
name: invalid-bid
description: System with invalid bid
tools: Read
model: inherit
color: blue
bid:
- INVALID BID SYNTAX HERE
---

Content.
"#;

        let result = SystemParser::parse(content);
        assert!(matches!(result, Err(ParseError::BidParseError(_, _))));
    }

    #[test]
    fn bid_validation_too_many() {
        let content = format!(
            r#"---
name: too-many-bids
description: System with too many bids
tools: Read
model: inherit
color: blue
bid:
{}
---

Content.
"#,
            (0..101)
                .map(|i| format!("- ON {} BID {}", i, i))
                .collect::<Vec<_>>()
                .join("\n")
        );

        let result = SystemParser::parse(&content);
        assert!(matches!(result, Err(ParseError::ValidationError(_))));
    }

    // Comprehensive bid parsing tests

    #[test]
    fn single_bid_expression_parsing() {
        let content = r#"---
name: single-bid
description: System with a single bid
tools: Read
model: inherit
color: blue
bid:
- ON user.active BID user.score * 10
---

Content.
"#;

        let config = SystemParser::parse(content).unwrap();
        assert_eq!(config.bid.len(), 1);

        let bid_str = format!("{}", config.bid[0]);
        assert!(bid_str.contains("user.active"));
        assert!(bid_str.contains("user.score"));
        assert!(bid_str.contains("10"));
    }

    #[test]
    fn multiple_bid_expressions_parsing() {
        let content = r#"---
name: multiple-bids
description: System with multiple bids
tools: Read, Write
model: inherit
color: green
bid:
- ON user.premium BID base_price * 0.8
- ON order.amount > 100 BID order.amount + bonus
- ON category == "electronics" BID price * discount_rate
---

System content.
"#;

        let config = SystemParser::parse(content).unwrap();
        assert_eq!(config.bid.len(), 3);

        // Verify each bid parsed successfully
        for bid in &config.bid {
            let bid_str = format!("{}", bid);
            assert!(bid_str.starts_with("ON "));
            assert!(bid_str.contains(" BID "));
        }
    }

    #[test]
    fn complex_bid_expressions_parsing() {
        let content = r#"---
name: complex-bids
description: System with complex bid expressions
tools: Grep, Edit
model: inherit
color: red
bid:
- ON (user.tier == "premium" && order.total > 500.0) BID base_price * (1.0 - discount_rate)
- ON !user.restricted && (category == "books" || category == "electronics") BID price + shipping_bonus
- ON user.loyalty_points >= 1000 BID max_discount ^ loyalty_multiplier
---

Complex system.
"#;

        let config = SystemParser::parse(content).unwrap();
        assert_eq!(config.bid.len(), 3);

        // Verify complex expressions parsed
        let first_bid = format!("{}", config.bid[0]);
        assert!(first_bid.contains("user.tier"));
        assert!(first_bid.contains("premium"));
        assert!(first_bid.contains("&&"));
        assert!(first_bid.contains("order.total"));
    }

    #[test]
    fn bid_whitespace_variations() {
        let content = r#"---
name: whitespace-test
description: Testing whitespace handling in bids
tools: Read
model: inherit
color: blue
bid:
- ON user.active BID score
  - ON    user.premium    BID    premium_bonus
    -   ON user.vip   BID   vip_rate
   -ON condition BID value
---

Content.
"#;

        let config = SystemParser::parse(content).unwrap();
        assert_eq!(config.bid.len(), 4);

        // All should parse successfully despite whitespace variations
        for bid in &config.bid {
            let bid_str = format!("{}", bid);
            assert!(bid_str.starts_with("ON "));
            assert!(bid_str.contains(" BID "));
        }
    }

    #[test]
    fn bid_without_bullet_points() {
        let content = r#"---
name: no-bullets
description: Bid lines without bullet points
tools: Read
model: inherit
color: yellow
bid:
ON simple BID value
ON another.condition BID another.value
---

Content.
"#;

        let config = SystemParser::parse(content).unwrap();
        assert_eq!(config.bid.len(), 2);

        let first_bid_str = format!("{}", config.bid[0]);
        assert!(first_bid_str.contains("simple"));
        assert!(first_bid_str.contains("value"));

        let second_bid_str = format!("{}", config.bid[1]);
        assert!(second_bid_str.contains("another.condition"));
        assert!(second_bid_str.contains("another.value"));
    }

    #[test]
    fn mixed_bullet_formats() {
        let content = r#"---
name: mixed-bullets
description: Mixed bullet point formats
tools: Read
model: inherit
color: purple
bid:
- ON standard.bullet BID standard.value
  - ON indented.bullet BID indented.value
ON no.bullet BID no.value
    - ON deep.indented BID deep.value
- ON final.bullet BID final.value
---

Content.
"#;

        let config = SystemParser::parse(content).unwrap();
        assert_eq!(config.bid.len(), 5);
    }

    #[test]
    fn bid_with_empty_lines() {
        let content = r#"---
name: empty-lines
description: Bid section with empty lines
tools: Read
model: inherit
color: orange
bid:

- ON first.condition BID first.value

- ON second.condition BID second.value


- ON third.condition BID third.value

---

Content.
"#;

        let config = SystemParser::parse(content).unwrap();
        assert_eq!(config.bid.len(), 3);
    }

    #[test]
    fn bid_parsing_string_literals() {
        let content = r#"---
name: string-literals
description: Bid expressions with string literals
tools: Read
model: inherit
color: blue
bid:
- ON user.category == "premium" BID "bonus_rate"
- ON product.name ~= "iPhone.*" BID base_price * 0.9
- ON description == "Special offer" BID "max_discount"
---

Content.
"#;

        let config = SystemParser::parse(content).unwrap();
        assert_eq!(config.bid.len(), 3);

        let first_bid_str = format!("{}", config.bid[0]);
        assert!(first_bid_str.contains("premium"));
        assert!(first_bid_str.contains("bonus_rate"));
    }

    #[test]
    fn bid_parsing_numeric_literals() {
        let content = r#"---
name: numeric-literals
description: Bid expressions with various numeric types
tools: Read
model: inherit
color: purple
bid:
- ON count > 42 BID count * 1.5
- ON price <= 99.99 BID price + 10
- ON rating >= 4.0 BID bonus
- ON quantity == 0 BID default_value
---

Content.
"#;

        let config = SystemParser::parse(content).unwrap();
        assert_eq!(config.bid.len(), 4);
    }

    #[test]
    fn bid_parsing_boolean_literals() {
        let content = r#"---
name: boolean-literals
description: Bid expressions with boolean values
tools: Read
model: inherit
color: blue
bid:
- ON is_active == true BID active_bonus
- ON is_restricted == false BID unrestricted_bonus
- ON true BID always_active
- ON !false BID also_always_active
---

Content.
"#;

        let config = SystemParser::parse(content).unwrap();
        assert_eq!(config.bid.len(), 4);
    }

    #[test]
    fn bid_parsing_arithmetic_operators() {
        let content = r#"---
name: arithmetic-ops
description: Bid expressions with all arithmetic operators
tools: Read
model: inherit
color: green
bid:
- ON a + b > 10 BID result
- ON x - y < 5 BID difference
- ON price * quantity > 100 BID total
- ON amount / count >= 2.5 BID average
- ON value % 10 == 0 BID modulo_bonus
- ON base ^ exponent > 1000 BID power_bonus
---

Content.
"#;

        let config = SystemParser::parse(content).unwrap();
        assert_eq!(config.bid.len(), 6);
    }

    #[test]
    fn bid_parsing_comparison_operators() {
        let content = r#"---
name: comparison-ops
description: Bid expressions with all comparison operators
tools: Read
model: inherit
color: blue
bid:
- ON a == b BID equal_bonus
- ON x != y BID not_equal_bonus
- ON price < 100 BID low_price_bonus
- ON score <= max_score BID within_limit_bonus
- ON rating > 4.5 BID high_rating_bonus
- ON age >= 18 BID adult_bonus
---

Content.
"#;

        let config = SystemParser::parse(content).unwrap();
        assert_eq!(config.bid.len(), 6);
    }

    #[test]
    fn bid_parsing_logical_operators() {
        let content = r#"---
name: logical-ops
description: Bid expressions with logical operators
tools: Read
model: inherit
color: blue
bid:
- ON active && !restricted BID combined_bonus
- ON premium || vip BID membership_bonus
- ON (a > 5 && b < 10) || (c == 0) BID complex_logic_bonus
- ON !(expired || suspended) BID valid_account_bonus
---

Content.
"#;

        let config = SystemParser::parse(content).unwrap();
        assert_eq!(config.bid.len(), 4);
    }

    #[test]
    fn bid_parsing_regex_operators() {
        let content = r#"---
name: regex-ops
description: Bid expressions with regex match operators
tools: Read
model: inherit
color: green
bid:
- ON email ~= ".*@company\.com" BID company_bonus
- ON product_code ~= "ELEC-[0-9]+" BID electronics_bonus
- ON description ~= "(?i)special.*offer" BID special_offer_bonus
---

Content.
"#;

        let config = SystemParser::parse(content).unwrap();
        assert_eq!(config.bid.len(), 3);
    }

    #[test]
    fn bid_parsing_complex_variable_paths() {
        let content = r#"---
name: complex-paths
description: Bid expressions with complex variable paths
tools: Read
model: inherit
color: red
bid:
- ON user.profile.preferences.notifications BID notification_bonus
- ON order.items.electronics.count > 0 BID electronics_bonus
- ON customer.billing.address.country == "US" BID domestic_bonus
- ON product.metadata.tags.featured BID featured_bonus
---

Content.
"#;

        let config = SystemParser::parse(content).unwrap();
        assert_eq!(config.bid.len(), 4);

        let first_bid_str = format!("{}", config.bid[0]);
        assert!(first_bid_str.contains("user.profile.preferences.notifications"));
    }

    #[test]
    fn bid_parsing_parentheses_precedence() {
        let content = r#"---
name: parentheses-precedence
description: Bid expressions with parentheses for precedence
tools: Read
model: inherit
color: blue
bid:
- ON (price + tax) * quantity > budget BID over_budget_penalty
- ON base * (1.0 + tax_rate) BID taxed_amount
- ON (user.active && !user.suspended) || admin BID access_bonus
- ON price * (discount_rate + loyalty_bonus) BID final_discount
---

Content.
"#;

        let config = SystemParser::parse(content).unwrap();
        assert_eq!(config.bid.len(), 4);
    }

    #[test]
    fn bid_parsing_unary_operators() {
        let content = r#"---
name: unary-ops
description: Bid expressions with unary operators
tools: Read
model: inherit
color: purple
bid:
- ON !inactive BID active_bonus
- ON -balance < 0 BID debt_penalty
- ON *reference_value > threshold BID reference_bonus
- ON !suspended && !expired BID valid_bonus
---

Content.
"#;

        let config = SystemParser::parse(content).unwrap();
        assert_eq!(config.bid.len(), 4);
    }

    #[test]
    fn bid_parsing_member_access() {
        let content = r#"---
name: member-access
description: Bid expressions with member access on dereferenced values
tools: Read
model: inherit
color: gray
bid:
- ON (*user_ref).active BID referenced_user_bonus
- ON (*product_ptr).price > 100 BID expensive_product_bonus
- ON (*config).max_discount <= 0.5 BID within_limit_bonus
---

Content.
"#;

        let config = SystemParser::parse(content).unwrap();
        assert_eq!(config.bid.len(), 3);
    }

    #[test]
    fn invalid_bid_syntax_handling() {
        let test_cases = vec![
            ("- INVALID SYNTAX", "invalid syntax"),
            ("- ON condition", "missing BID keyword"),
            ("- BID value", "missing ON keyword"),
            ("- ON BID", "empty expressions"),
            ("- ON condition BID", "empty expressions"),
            ("- ON condition BID value extra", "unexpected token"),
        ];

        for (bid_line, _description) in test_cases {
            let content = format!(
                r#"---
name: invalid-bid
description: System with invalid bid
tools: Read
model: inherit
color: red
bid:
{}
---

Content.
"#,
                bid_line
            );

            let result = SystemParser::parse(&content);
            assert!(
                matches!(result, Err(ParseError::BidParseError(_, _))),
                "Should fail parsing invalid bid: {}",
                bid_line
            );
        }
    }

    #[test]
    fn bid_parsing_operator_precedence() {
        let content = r#"---
name: precedence-test
description: Testing operator precedence in bids
tools: Read
model: inherit
color: yellow
bid:
- ON a || b && c == d < e + f * g ^ h BID result
- ON !x && y || z BID logical_precedence
- ON base + rate * multiplier ^ power BID arithmetic_precedence
- ON condition1 && condition2 || condition3 && condition4 BID combined_precedence
---

Content.
"#;

        let config = SystemParser::parse(content).unwrap();
        assert_eq!(config.bid.len(), 4);
    }

    #[test]
    fn bid_field_with_inline_value() {
        let content = r#"---
name: inline-bid
description: Bid field with inline value (should be empty)
tools: Read
model: inherit
color: orange
bid: ON inline BID value
---

Content.
"#;

        let config = SystemParser::parse(content).unwrap();
        // When bid has inline value, it should be parsed as a single bid
        assert_eq!(config.bid.len(), 1);

        let bid_str = format!("{}", config.bid[0]);
        assert!(bid_str.contains("inline"));
        assert!(bid_str.contains("value"));
    }

    #[test]
    fn bid_field_mixed_inline_and_multiline() {
        let content = r#"---
name: mixed-format
description: Mixed inline and multiline bid format
tools: Read
model: inherit
color: pink
bid: ON first BID first_value
- ON second BID second_value
- ON third BID third_value
---

Content.
"#;

        let config = SystemParser::parse(content).unwrap();
        // Should parse the inline value and the bullet points
        assert!(!config.bid.is_empty(), "Should have at least one bid");
    }

    #[test]
    fn bid_parsing_edge_case_empty_bullet() {
        let content = r#"---
name: empty-bullet
description: Bid with empty bullet point
tools: Read
model: inherit
color: yellow
bid:
- ON valid BID value
-
- ON another BID another_value
---

Content.
"#;

        let config = SystemParser::parse(content).unwrap();
        // Should skip empty bullet and parse valid ones
        assert_eq!(config.bid.len(), 2);
    }

    #[test]
    fn bid_parsing_special_characters_in_variables() {
        let content = r#"---
name: special-chars
description: Variables with underscores and numbers
tools: Read
model: inherit
color: yellow
bid:
- ON user_123.profile_data BID bonus_rate_2024
- ON item_count_v2 > max_items_allowed BID overflow_penalty_v1
- ON api_key_status == "valid_2024" BID api_bonus_rate
---

Content.
"#;

        let config = SystemParser::parse(content).unwrap();
        assert_eq!(config.bid.len(), 3);
    }

    #[test]
    fn bid_parsing_very_long_expressions() {
        let long_condition = "user.profile.preferences.notifications.email && user.profile.preferences.notifications.sms && user.profile.account.status == \"active\" && user.profile.account.verified == true && user.subscription.tier == \"premium\" && user.subscription.expires > current_date";
        let long_value = "base_rate * premium_multiplier * notification_bonus * verification_bonus + loyalty_points_bonus - processing_fee";

        let content = format!(
            r#"---
name: long-expressions
description: Very long bid expressions
tools: Read
model: inherit
color: orange
bid:
- ON {} BID {}
---

Content.
"#,
            long_condition, long_value
        );

        let config = SystemParser::parse(&content).unwrap();
        assert_eq!(config.bid.len(), 1);

        let bid_str = format!("{}", config.bid[0]);
        assert!(bid_str.contains("user.profile.preferences"));
        assert!(bid_str.contains("premium_multiplier"));
    }

    #[test]
    fn bid_validation_limits() {
        // Test exactly at the limit (100 bids)
        let content = format!(
            r#"---
name: limit-test
description: Exactly 100 bids
tools: Read
model: inherit
color: blue
bid:
{}
---

Content.
"#,
            (0..100)
                .map(|i| format!("- ON condition_{} BID value_{}", i, i))
                .collect::<Vec<_>>()
                .join("\n")
        );

        let config = SystemParser::parse(&content).unwrap();
        assert_eq!(config.bid.len(), 100);
    }

    #[test]
    fn bid_parsing_unicode_in_strings() {
        let content = r#"---
name: unicode-strings
description: Bid with unicode characters in strings
tools: Read
model: inherit
color: purple
bid:
- ON user.name == "José González" BID international_bonus
- ON product.emoji == "🚀" BID emoji_bonus
- ON description ~= ".*特別.*" BID special_bonus
---

Content.
"#;

        let config = SystemParser::parse(content).unwrap();
        assert_eq!(config.bid.len(), 3);

        let first_bid_str = format!("{}", config.bid[0]);
        assert!(first_bid_str.contains("José González"));
    }

    #[test]
    fn bid_parsing_float_precision() {
        let content = r#"---
name: float-precision
description: Bid with high precision floating point numbers
tools: Read
model: inherit
color: green
bid:
- ON price > 99.99 BID discount_rate * 0.05
- ON rating >= 4.7589 BID precision_bonus
- ON tax_rate == 0.08750 BID exact_match_bonus
---

Content.
"#;

        let config = SystemParser::parse(content).unwrap();
        assert_eq!(config.bid.len(), 3);
    }

    #[test]
    fn bid_integration_with_other_fields() {
        let content = r#"---
name: integration-test
description: Bid integration with all other SystemConfig fields
tools: Glob, Grep, Read, Edit, Write
model: gpt-4
color: #FF5733
bid:
- ON system.complexity > threshold BID computational_cost
- ON tools.count >= 5 BID multi_tool_bonus
- ON model == "gpt-4" BID premium_model_bonus
---

This system demonstrates bid integration with:
- Multiple tools: Glob, Grep, Read, Edit, Write
- Custom model specification
- Hex color code
- Rich markdown content

The bids should work alongside all other configuration.
"#;

        let config = SystemParser::parse(content).unwrap();

        // Verify all fields parsed correctly
        assert_eq!(config.name, "integration-test");
        assert_eq!(config.tools.len(), 5);
        assert_eq!(config.model, "gpt-4");
        assert_eq!(config.color, "#FF5733");
        assert_eq!(config.bid.len(), 3);
        assert!(config.content.contains("This system demonstrates"));

        // Verify validation passes
        config.validate().unwrap();
    }

    #[test]
    fn bid_error_messages_quality() {
        let content = r#"---
name: error-test
description: Test bid error message quality
tools: Read
model: inherit
color: red
bid:
- COMPLETELY INVALID SYNTAX HERE
---

Content.
"#;

        let result = SystemParser::parse(content);

        if let Err(ParseError::BidParseError(bid_expr, bid_error)) = result {
            assert_eq!(bid_expr, "COMPLETELY INVALID SYNTAX HERE");

            let error_msg = format!("{}", bid_error);
            assert!(error_msg.contains("Expected 'ON' keyword"));
        } else {
            panic!("Expected BidParseError with descriptive message");
        }
    }

    #[test]
    fn bid_parsing_stress_test() {
        // Test with moderately complex nested expressions to avoid timeouts
        let mut complex_condition = "user.active && !user.restricted".to_string();
        for i in 0..10 {
            complex_condition = format!("({} || condition_{})", complex_condition, i);
        }

        let content = format!(
            r#"---
name: stress-test
description: Stress test with complex bid
tools: Read
model: inherit
color: yellow
bid:
- ON {} BID result
---

Content.
"#,
            complex_condition
        );

        let config = SystemParser::parse(&content).unwrap();
        assert_eq!(config.bid.len(), 1);
    }
}
