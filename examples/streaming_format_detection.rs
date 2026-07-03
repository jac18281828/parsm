//! Realistic example: parsm auto-detects format per input stream and parses
//! accordingly, using the library API directly (no CLI, no subprocess).
//!
//! Run with: cargo run --example streaming_format_detection

use parsm::StreamingParser;

fn main() {
    let samples = [
        ("json", r#"{"name": "Alice", "age": 30}"#),
        ("csv", "Alice,30,Engineer"),
        ("logfmt", "level=error msg=timeout service=api"),
    ];

    for (label, line) in samples {
        let mut parser = StreamingParser::new();
        let parsed = parser.parse_line(line).expect("sample line should parse");
        println!("{label}: {parsed:?}");
    }
}
