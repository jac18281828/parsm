/// Format detection module for identifying structured data formats
///
/// This module provides heuristic-based format detection to optimize parsing
/// performance by trying the most likely formats first.

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DetectedFormat {
    Json,
    JsonArray,
    Toml,
    Yaml,
    Csv,
    Logfmt,
    PlainText,
}

pub struct FormatDetector;

impl FormatDetector {
    /// Analyze input and return likely formats with confidence scores
    ///
    /// Returns a vector of (format, confidence) pairs sorted by confidence (highest first).
    /// Confidence scores range from 0.0 to 1.0.
    pub fn detect(input: &str) -> Vec<(DetectedFormat, f32)> {
        let mut candidates = Vec::new();

        // Fast path: check first few bytes for common structural indicators
        let prefix = &input[..input.len().min(100)];
        let trimmed_prefix = prefix.trim_start();

        // High confidence structural indicators
        if trimmed_prefix.starts_with('{') {
            candidates.push((DetectedFormat::Json, 0.9));
        }
        if trimmed_prefix.starts_with('[') {
            candidates.push((DetectedFormat::JsonArray, 0.9));
        }
        if trimmed_prefix.starts_with("---") {
            candidates.push((DetectedFormat::Yaml, 0.95));
        }

        // Heuristic-based detection for formats without clear delimiters
        if Self::is_likely_toml(input) {
            candidates.push((DetectedFormat::Toml, 0.8));
        }
        if Self::is_likely_yaml(input) && !trimmed_prefix.starts_with("---") {
            // Lower confidence if we didn't already detect YAML via document marker
            candidates.push((DetectedFormat::Yaml, 0.7));
        }
        if Self::is_likely_csv(input) {
            candidates.push((DetectedFormat::Csv, 0.6));
        }
        if Self::is_likely_logfmt(input) {
            candidates.push((DetectedFormat::Logfmt, 0.5));
        }

        // Always include plain text as fallback
        candidates.push((DetectedFormat::PlainText, 0.1));

        // Sort by confidence (highest first)
        candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Remove duplicates while preserving highest confidence
        let mut seen = std::collections::HashSet::new();
        candidates.retain(|(format, _)| seen.insert(format.clone()));

        candidates
    }

    /// Check if content looks like TOML format
    ///
    /// TOML characteristics:
    /// - Key = value assignments
    /// - Section headers in brackets [section]
    /// - Comments starting with #
    pub fn is_likely_toml(input: &str) -> bool {
        let lines: Vec<&str> = input.lines().take(10).collect(); // Check first 10 lines
        let mut toml_indicators = 0;

        for line in &lines {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            // Look for key = value pattern typical of TOML (with spaces around =)
            if trimmed.contains(" = ") && !trimmed.starts_with('"') {
                toml_indicators += 2;
            }

            // Look for TOML section headers
            if trimmed.starts_with('[') && trimmed.ends_with(']') && !trimmed.contains(':') {
                toml_indicators += 3;
            }

            // TOML table arrays
            if trimmed.starts_with("[[") && trimmed.ends_with("]]") {
                toml_indicators += 3;
            }
        }

        toml_indicators >= 2
    }

    /// Check if content looks like YAML format
    ///
    /// YAML characteristics:
    /// - Key: value patterns (colon followed by space)
    /// - List items starting with dash and space
    /// - Indentation-based structure
    /// - Document markers (---)
    pub fn is_likely_yaml(input: &str) -> bool {
        let lines: Vec<&str> = input.lines().take(10).collect(); // Check first 10 lines

        // YAML document start indicator
        if input.trim_start().starts_with("---") {
            return true;
        }

        let mut yaml_indicators = 0;

        for line in &lines {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            // Look for YAML key: value pattern (with colon and space)
            if trimmed.contains(": ") && !trimmed.starts_with('"') && !trimmed.contains(" = ") {
                yaml_indicators += 1;
            }

            // Look for YAML list items
            if trimmed.starts_with("- ") {
                yaml_indicators += 1;
            }

            // Look for indented structure (strong indicator of YAML)
            if line.starts_with("  ") && (line.contains(": ") || line.trim().starts_with("- ")) {
                yaml_indicators += 2;
            }
        }

        yaml_indicators >= 2
    }

    /// Check if content looks like CSV format
    ///
    /// CSV characteristics:
    /// - Comma-separated values
    /// - Consistent number of fields per line
    /// - Optional quoted fields
    pub fn is_likely_csv(input: &str) -> bool {
        let lines: Vec<&str> = input.lines().take(5).collect();

        if lines.is_empty() {
            return false;
        }

        // Check if lines contain commas and have consistent field counts
        let mut field_counts = Vec::new();
        let mut has_commas = false;

        for line in &lines {
            if line.trim().is_empty() {
                continue;
            }

            let field_count = line.matches(',').count() + 1;
            field_counts.push(field_count);

            if line.contains(',') {
                has_commas = true;
            }
        }

        // Must have commas and either single line with commas or consistent field counts across multiple lines
        has_commas
            && (field_counts.len() == 1
                || (field_counts.len() > 1 && field_counts.windows(2).all(|w| w[0] == w[1])))
    }

    /// Check if content looks like logfmt format
    ///
    /// Logfmt characteristics:
    /// - key=value pairs
    /// - Space-separated key=value pairs
    /// - Values may be quoted
    pub fn is_likely_logfmt(input: &str) -> bool {
        let lines: Vec<&str> = input.lines().take(5).collect();

        for line in &lines {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            // Count key=value patterns
            let kv_pairs = trimmed
                .split_whitespace()
                .filter(|part| part.contains('=') && !part.starts_with('=') && !part.ends_with('='))
                .count();

            // If most space-separated parts look like key=value, it's likely logfmt
            let total_parts = trimmed.split_whitespace().count();
            if total_parts > 0 && kv_pairs as f32 / total_parts as f32 > 0.5 {
                return true;
            }
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_detection() {
        let json_input = r#"{"name": "Alice", "age": 30}"#;
        let detected = FormatDetector::detect(json_input);

        assert_eq!(detected[0].0, DetectedFormat::Json);
        assert!(detected[0].1 > 0.8);
    }

    #[test]
    fn test_json_array_detection() {
        let json_array_input = r#"[{"name": "Alice"}, {"name": "Bob"}]"#;
        let detected = FormatDetector::detect(json_array_input);

        assert_eq!(detected[0].0, DetectedFormat::JsonArray);
        assert!(detected[0].1 > 0.8);
    }

    #[test]
    fn test_yaml_detection() {
        let yaml_input = r#"---
name: Alice
age: 30
address:
  street: 123 Main St
  city: Anytown"#;
        let detected = FormatDetector::detect(yaml_input);

        assert_eq!(detected[0].0, DetectedFormat::Yaml);
        assert!(detected[0].1 > 0.9);
    }

    #[test]
    fn test_toml_detection() {
        let toml_input = r#"name = "Alice"
age = 30

[address]
street = "123 Main St"
city = "Anytown""#;

        assert!(FormatDetector::is_likely_toml(toml_input));

        let detected = FormatDetector::detect(toml_input);
        let toml_detected = detected
            .iter()
            .find(|(format, _)| format == &DetectedFormat::Toml);
        assert!(toml_detected.is_some());
        assert!(toml_detected.unwrap().1 > 0.7);
    }

    #[test]
    fn test_csv_detection() {
        let csv_input = r#"name,age,city
Alice,30,Anytown
Bob,25,Other City"#;

        assert!(FormatDetector::is_likely_csv(csv_input));

        let detected = FormatDetector::detect(csv_input);
        let csv_detected = detected
            .iter()
            .find(|(format, _)| format == &DetectedFormat::Csv);
        assert!(csv_detected.is_some());
    }

    #[test]
    fn test_logfmt_detection() {
        let logfmt_input =
            r#"level=info msg="User logged in" user_id=123 timestamp="2023-01-01T10:00:00Z""#;

        assert!(FormatDetector::is_likely_logfmt(logfmt_input));

        let detected = FormatDetector::detect(logfmt_input);
        let logfmt_detected = detected
            .iter()
            .find(|(format, _)| format == &DetectedFormat::Logfmt);
        assert!(logfmt_detected.is_some());
    }

    #[test]
    fn test_format_detection_order() {
        let json_input = r#"{"name": "Alice", "age": 30}"#;
        let detected = FormatDetector::detect(json_input);

        // Should be sorted by confidence (highest first)
        for window in detected.windows(2) {
            assert!(window[0].1 >= window[1].1);
        }
    }

    #[test]
    fn test_no_duplicate_formats() {
        let mixed_input = r#"{"name": "Alice", "age": 30}"#; // Could be detected as JSON
        let detected = FormatDetector::detect(mixed_input);

        let mut seen_formats = std::collections::HashSet::new();
        for (format, _) in detected {
            assert!(seen_formats.insert(format), "Duplicate format detected");
        }
    }
}
