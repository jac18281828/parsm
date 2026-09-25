//! Realistic example: parsm auto-detects the format of each input stream and
//! reads it as records, using the library API directly (no CLI, no
//! subprocess).
//!
//! Run with: cargo run --example streaming_format_detection

use parsm::Records;
use std::io::Cursor;

fn main() {
    let samples = [
        ("json", r#"{"name": "Alice", "age": 30}"#),
        ("csv", "Alice,30,Engineer"),
        ("logfmt", "level=error msg=timeout service=api"),
    ];

    for (label, input) in samples {
        let mut records = Records::open(Cursor::new(input), None).expect("in-memory input reads");
        let format = records.format();
        let record = records
            .next()
            .expect("sample has one record")
            .expect("sample record parses");
        println!("{label}: {format} {}", record.to_json());
    }
}
