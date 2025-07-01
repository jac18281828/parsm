/// Parser registry module for managing different document parsers
/// 
/// This module implements the registry pattern to allow structured
/// document parsing with intelligent format detection and fallback.

use crate::format_detector::{DetectedFormat, FormatDetector};
use serde_json::Value;
use std::error::Error;

/// Trait for document parsers that can handle specific formats
pub trait DocumentParser: Send + Sync {
    /// Check if this parser can likely handle the given input
    fn can_parse(&self, input: &str) -> bool;
    
    /// Parse the input into a JSON value
    fn parse(&self, input: &str) -> Result<Value, Box<dyn Error>>;
    
    /// Get the format name this parser handles
    fn format_name(&self) -> &'static str;
    
    /// Get the detected format enum variant
    fn format_type(&self) -> DetectedFormat;
}

/// JSON document parser
pub struct JsonParser;

impl DocumentParser for JsonParser {
    fn can_parse(&self, input: &str) -> bool {
        let trimmed = input.trim_start();
        (trimmed.starts_with('{') || trimmed.starts_with('[')) && 
        serde_json::from_str::<Value>(input).is_ok()
    }
    
    fn parse(&self, input: &str) -> Result<Value, Box<dyn Error>> {
        Ok(serde_json::from_str(input)?)
    }
    
    fn format_name(&self) -> &'static str {
        "JSON"
    }
    
    fn format_type(&self) -> DetectedFormat {
        // Default to JSON, specific detection happens in can_parse
        DetectedFormat::Json
    }
}

/// TOML document parser
pub struct TomlParser;

impl DocumentParser for TomlParser {
    fn can_parse(&self, input: &str) -> bool {
        FormatDetector::is_likely_toml(input) && 
        toml::from_str::<toml::Value>(input).is_ok()
    }
    
    fn parse(&self, input: &str) -> Result<Value, Box<dyn Error>> {
        let toml_value = toml::from_str::<toml::Value>(input)?;
        Ok(serde_json::to_value(toml_value)?)
    }
    
    fn format_name(&self) -> &'static str {
        "TOML"
    }
    
    fn format_type(&self) -> DetectedFormat {
        DetectedFormat::Toml
    }
}

/// YAML document parser
pub struct YamlParser;

impl DocumentParser for YamlParser {
    fn can_parse(&self, input: &str) -> bool {
        FormatDetector::is_likely_yaml(input) && 
        serde_yaml::from_str::<serde_yaml::Value>(input).is_ok()
    }
    
    fn parse(&self, input: &str) -> Result<Value, Box<dyn Error>> {
        let yaml_value = serde_yaml::from_str::<serde_yaml::Value>(input)?;
        Ok(serde_json::to_value(yaml_value)?)
    }
    
    fn format_name(&self) -> &'static str {
        "YAML"
    }
    
    fn format_type(&self) -> DetectedFormat {
        DetectedFormat::Yaml
    }
}

/// Registry for managing and trying different document parsers
pub struct ParserRegistry {
    parsers: Vec<Box<dyn DocumentParser>>,
}

impl ParserRegistry {
    /// Create a new parser registry with default parsers
    pub fn new() -> Self {
        let mut registry = Self { 
            parsers: Vec::new() 
        };
        
        // Register parsers in order of preference/performance
        registry.register(Box::new(JsonParser));
        registry.register(Box::new(TomlParser));
        registry.register(Box::new(YamlParser));
        
        registry
    }
    
    /// Register a new parser with the registry
    pub fn register(&mut self, parser: Box<dyn DocumentParser>) {
        self.parsers.push(parser);
    }
    
    /// Try to parse a document using format detection and registered parsers
    /// 
    /// Returns the parsed JSON value and the format name on success.
    /// Uses format detection to optimize parser order for better performance.
    pub fn parse_document(&self, input: &str) -> Result<(Value, &'static str), Box<dyn Error>> {
        // Use format detection to optimize parsing order
        let detected_formats = FormatDetector::detect(input);
        
        // Try detected formats first (in confidence order)
        for (format, confidence) in &detected_formats {
            if *confidence > 0.5 { // Only try high-confidence detections
                if let Some(parser) = self.find_parser_for_format(format) {
                    match parser.parse(input) {
                        Ok(value) => return Ok((value, parser.format_name())),
                        Err(_) => continue, // Try next format
                    }
                }
            }
        }
        
        // Fallback: try all parsers in registration order
        for parser in &self.parsers {
            if parser.can_parse(input) {
                match parser.parse(input) {
                    Ok(value) => return Ok((value, parser.format_name())),
                    Err(_) => continue, // Try next parser
                }
            }
        }
        
        Err("No parser could handle the input".into())
    }
    
    /// Find a parser that handles the given detected format
    fn find_parser_for_format(&self, format: &DetectedFormat) -> Option<&Box<dyn DocumentParser>> {
        self.parsers.iter().find(|parser| {
            match format {
                DetectedFormat::Json | DetectedFormat::JsonArray => {
                    parser.format_name() == "JSON"
                }
                DetectedFormat::Toml => parser.format_name() == "TOML",
                DetectedFormat::Yaml => parser.format_name() == "YAML",
                _ => false, // Other formats not handled by document parsers
            }
        })
    }
    
    /// Get all registered parser format names
    pub fn get_supported_formats(&self) -> Vec<&'static str> {
        self.parsers.iter().map(|p| p.format_name()).collect()
    }
}

impl Default for ParserRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_parser() {
        let parser = JsonParser;
        let json_input = r#"{"name": "Alice", "age": 30}"#;
        
        assert!(parser.can_parse(json_input));
        assert_eq!(parser.format_name(), "JSON");
        
        let result = parser.parse(json_input).unwrap();
        assert_eq!(result["name"], "Alice");
        assert_eq!(result["age"], 30);
    }

    #[test]
    fn test_toml_parser() {
        let parser = TomlParser;
        let toml_input = r#"name = "Alice"
age = 30"#;
        
        assert!(parser.can_parse(toml_input));
        assert_eq!(parser.format_name(), "TOML");
        
        let result = parser.parse(toml_input).unwrap();
        assert_eq!(result["name"], "Alice");
        assert_eq!(result["age"], 30);
    }

    #[test]
    fn test_yaml_parser() {
        let parser = YamlParser;
        let yaml_input = r#"name: Alice
age: 30"#;
        
        assert!(parser.can_parse(yaml_input));
        assert_eq!(parser.format_name(), "YAML");
        
        let result = parser.parse(yaml_input).unwrap();
        assert_eq!(result["name"], "Alice");
        assert_eq!(result["age"], 30);
    }

    #[test]
    fn test_parser_registry() {
        let registry = ParserRegistry::new();
        let formats = registry.get_supported_formats();
        
        assert!(formats.contains(&"JSON"));
        assert!(formats.contains(&"TOML"));
        assert!(formats.contains(&"YAML"));
    }

    #[test]
    fn test_registry_json_parsing() {
        let registry = ParserRegistry::new();
        let json_input = r#"{"name": "Alice", "age": 30}"#;
        
        let (result, format) = registry.parse_document(json_input).unwrap();
        assert_eq!(format, "JSON");
        assert_eq!(result["name"], "Alice");
        assert_eq!(result["age"], 30);
    }

    #[test]
    fn test_registry_toml_parsing() {
        let registry = ParserRegistry::new();
        let toml_input = r#"name = "Alice"
age = 30"#;
        
        let (result, format) = registry.parse_document(toml_input).unwrap();
        assert_eq!(format, "TOML");
        assert_eq!(result["name"], "Alice");
        assert_eq!(result["age"], 30);
    }

    #[test]
    fn test_registry_yaml_parsing() {
        let registry = ParserRegistry::new();
        let yaml_input = r#"name: Alice
age: 30"#;
        
        let (result, format) = registry.parse_document(yaml_input).unwrap();
        assert_eq!(format, "YAML");
        assert_eq!(result["name"], "Alice");
        assert_eq!(result["age"], 30);
    }

    #[test]
    fn test_registry_format_detection_optimization() {
        let registry = ParserRegistry::new();
        
        // Test that JSON is detected and parsed correctly
        let json_input = r#"{"name": "Alice"}"#;
        let (_, format) = registry.parse_document(json_input).unwrap();
        assert_eq!(format, "JSON");
        
        // Test that YAML document marker is detected
        let yaml_input = r#"---
name: Alice"#;
        let (_, format) = registry.parse_document(yaml_input).unwrap();
        assert_eq!(format, "YAML");
    }

    #[test]
    fn test_registry_fallback_behavior() {
        let registry = ParserRegistry::new();
        let invalid_input = "not valid structured data";
        
        let result = registry.parse_document(invalid_input);
        assert!(result.is_err());
    }

    #[test]
    fn test_json_array_detection() {
        let registry = ParserRegistry::new();
        let json_array_input = r#"[{"name": "Alice"}, {"name": "Bob"}]"#;
        
        let (result, format) = registry.parse_document(json_array_input).unwrap();
        assert_eq!(format, "JSON");
        assert!(result.is_array());
        assert_eq!(result.as_array().unwrap().len(), 2);
    }
}
