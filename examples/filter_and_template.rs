//! Realistic example: filter a batch of JSON records and render a template
//! for each match, using parsm's library API directly (no CLI, no
//! subprocess).
//!
//! Run with: cargo run --example filter_and_template

use parsm::{FilterEngine, parse_command};
use serde_json::json;

fn main() {
    let dsl = parse_command(r#"age > 25 && active == true {${name} (${age}) - ${role}}"#)
        .expect("valid DSL expression");

    let records = vec![
        json!({"name": "Alice", "age": 30, "active": true, "role": "admin"}),
        json!({"name": "Bob", "age": 22, "active": true, "role": "user"}),
        json!({"name": "Carol", "age": 41, "active": false, "role": "admin"}),
        json!({"name": "Dave", "age": 35, "active": true, "role": "moderator"}),
    ];

    for record in &records {
        let passes = match &dsl.filter {
            Some(filter) => FilterEngine::evaluate(filter, record),
            None => true,
        };
        if !passes {
            continue;
        }
        if let Some(template) = &dsl.template {
            println!("{}", template.render(record));
        }
    }
}
