use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
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
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::NoFrontmatter => write!(f, "No header frontmatter found"),
            ParseError::MissingRequiredField(field) => {
                write!(f, "Missing required field: {}", field)
            }
        }
    }
}

impl std::error::Error for ParseError {}

pub struct SystemParser;

impl SystemParser {
    pub fn parse(content: &str) -> Result<SystemConfig, ParseError> {
        let (header_section, markdown_content) = Self::split_frontmatter(content)?;
        let header_data = Self::parse_header_section(&header_section)?;

        Ok(SystemConfig {
            name: Self::get_required_field(&header_data, "name")?,
            description: Self::get_required_field(&header_data, "description")?,
            tools: Self::parse_tools(&header_data)?,
            model: Self::get_required_field(&header_data, "model")?,
            color: Self::get_required_field(&header_data, "color")?,
            content: markdown_content.trim().to_string(),
        })
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

    #[test]
    fn parse_actual_dry_principal_file() {
        let content = std::fs::read_to_string("dry-principal.md").unwrap();
        let config = SystemParser::parse(&content).unwrap();

        assert_eq!(config.name, "dry-principal");
        assert!(config.description.contains("DRY (Don't Repeat Yourself)"));
        assert!(config.tools.contains(&"Glob".to_string()));
        assert!(config.tools.contains(&"Grep".to_string()));
        assert_eq!(config.model, "inherit");
        assert_eq!(config.color, "purple");
        assert!(config.content.contains("You are the DRY Principal"));
    }
}
