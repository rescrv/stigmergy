use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SystemConfig {
    pub name: String,
    pub description: String,
    pub tools: Vec<String>,
    pub model: String,
    pub color: String,
    pub content: String,
}

#[derive(Debug)]
pub enum ParseError {
    NoFrontmatter,
    MissingRequiredField(String),
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

pub struct SystemParser;

impl SystemParser {
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
