# parsm - **Parse 'Em** - An 'everything' parser, Sedder, Awkker, Grokker, Grepper

Parsm is the powerful command-line tool that understands structured text better than sed, awk, grep or grok.

<img src="eatcookie.jpg" alt="Eat more cookie!" width="25%">

## Overview

`parsm` is a multi-format data processor that automatically detects and parses JSON, CSV, TOML, YAML, logfmt, and plain text. It provides powerful filtering and templating capabilities with a simple, intuitive syntax.

## Installation

```bash
cargo install --path .
```

Or build from source:

```bash
git clone <repository-url>
cd parsm
cargo build --release
./target/release/parsm --examples
```

## Quick Start

```bash
# Basic usage
parsm [FILTER] [TEMPLATE]

# Show comprehensive examples
parsm --examples

# Extract a field (most common operation)
echo '{"name": "Alice", "age": 30}' | parsm 'name'

# Extract nested fields
echo '{"user": {"email": "alice@example.com"}}' | parsm 'user.email'

# Filter data based on field values
echo '{"name": "Alice", "age": 30}' | parsm 'age > 25'

# Filter and format output
echo '{"name": "Alice", "age": 30}' | parsm 'age > 25 [${name} is ${age} years old]'

# Simple template output
echo '{"name": "Alice", "age": 30}' | parsm '$name'

# Parse and understand text
echo "a dog is an excellent companion" | parsm 'word_1 == "dog" [The cat would not say $word_4]'
```

## Supported Input Formats

`parsm` automatically detects and parses these formats:

### JSON
```json
{"name": "Alice", "age": 30, "active": true}
```

### CSV
```csv
Alice,30,Engineer
Bob,25,Designer
```

### YAML
```yaml
name: Alice
age: 30
active: true
```

### TOML
```toml
name = "Alice"
age = 30
active = true
```

### Logfmt
```
level=error msg="Database connection failed" service=api duration=1.2s
```

### Plain Text
```
Alice 30 Engineer
Bob 25 Designer
```

### CLI Flags to force format handling
--json - Force JSON format parsing
--yaml - Force YAML format parsing
--csv - Force CSV format parsing
--toml - Force TOML format parsing
--logfmt - Force logfmt format parsing
--text - Force plain text format parsing

## Filter Syntax

### Comparison Operators

| Operator | Description | Example |
|----------|-------------|---------|
| `==` | Equal to | `name == "Alice"` |
| `!=` | Not equal to | `status != "inactive"` |
| `<` | Less than | `age < 30` |
| `<=` | Less than or equal | `score <= 95` |
| `>` | Greater than | `age > 18` |
| `>=` | Greater than or equal | `score >= 90` |

### String Operations

| Operator | Description | Example |
|----------|-------------|---------|
| `~` | Contains substring | `email ~ "@company.com"` |
| `^=` | Starts with prefix | `name ^= "A"` |
| `$=` | Ends with suffix | `file $= ".log"` |

### Boolean Logic

| Operator | Description | Example |
|----------|-------------|---------|
| `&&` | Logical AND | `age > 18 && active == true` |
| `\|\|` | Logical OR | `role == "admin" \|\| role == "user"` |
| `!` | Logical NOT | `!(status == "disabled")` |

### Field Access

#### Simple Fields
```bash
name == "Alice"
age > 25
active == true
```

#### Nested Fields (JSON/YAML/TOML)
```bash
user.email == "alice@example.com"
config.database.host == "localhost"
metrics.cpu.usage > 80
```

#### CSV Fields
CSV columns are automatically named `field_0`, `field_1`, etc.:
```bash
field_0 == "Alice"    # First column
field_1 > "25"        # Second column (string comparison)
field_2 == "Engineer" # Third column
```

#### Text Words
Plain text words are named `word_0`, `word_1`, etc.:
```bash
word_0 == "Alice"     # First word
word_1 > "25"         # Second word
word_2 == "Engineer"  # Third word
```

## Syntax Overview

The parsm DSL has three main components with distinct, unambiguous syntax:

### Field Selectors (Data Extraction)
Extract specific fields using simple, unambiguous syntax - **the most common operation**:

```bash
name                     # Simple field extraction
user.email               # Nested field access
items.0                  # Array element access  
"field with spaces"      # Quoted field names (when needed)
'special-field'          # Single-quoted alternatives
"dev-dependencies.lib"   # Complex nested paths with special characters
```

**Key principle**: Bare identifiers like `name` are ALWAYS field selectors, never filters or templates.

**Cross-format compatibility**: Field selector syntax works identically across JSON, YAML, TOML, and other structured formats:

```bash
# These work the same for JSON, YAML, and TOML:
parsm 'package.name'           # Extract nested field
parsm '"package.name"'         # Same with quotes  
parsm '"field-with-hyphens"'   # Special characters
parsm '"field with spaces"'    # Spaces in field names
```

### Templates (Dynamic Output)
Templates format output with field values using explicit variable syntax:

```bash
[${name} is ${age} years old]    # Variables with ${...} in brackets
$name                            # Simple variable shorthand
[Hello ${name}!]                 # Mixed template with literals
[${0}]                          # Original input (requires brackets/braces)
[User: ${user.name}]            # Nested fields in templates
```

**Note**: Braced templates `{${name}}` are also supported as an alternative syntax.

### Literal Text (Static Output)
Brackets or braces without variables produce literal text:

```bash
[name]                   # Outputs literal text "name"
{name}                   # Outputs literal text "name"
[Hello world]            # Outputs literal text "Hello world"
{Hello world}            # Outputs literal text "Hello world"
[Price: $100]            # Outputs literal text with dollar sign
{Price: $100}            # Outputs literal text with dollar sign
```

### Filters (Data Processing)
Filter data using comparison operators with field selectors:

```bash
age > 25                 # Numeric comparison
name == "Alice"          # String equality
user.active == true      # Boolean comparison
!(status == "disabled")  # Negation
name == "Alice" && age > 25  # Boolean logic
```

## Field Truthy Syntax

The `field?` syntax is used to check the existence or truthiness of a field in the data. This ensures unambiguous handling of field selectors and filters.

### Examples

```bash
# Check if a field exists
parsm 'field?' < data.json

# Combine truthy checks with other filters
parsm 'field? && age > 25' < data.json

# Nested truthy checks
parsm 'user.email?' < data.json

# CSV truthy checks
parsm 'field_0?' < data.csv

# Plain text truthy checks
parsm 'word_0?' < data.txt
```

### Key Benefits
- **Unambiguous**: Distinguishes between field selectors and filters.
- **Flexible**: Works across JSON, CSV, YAML, TOML, and plain text.
- **Simple Syntax**: Easy to use and understand.

### Examples

```bash
# Field extraction (most common - simple syntax)
echo '{"name": "Alice", "age": 30}' | parsm 'name'
# Output: "Alice"

echo '{"user": {"email": "alice@example.com"}}' | parsm 'user.email'  
# Output: "alice@example.com"

# Template with variables (dynamic output)
echo '{"name": "Alice", "age": 30}' | parsm '[${name} is ${age} years old]'
# Output: Alice is 30 years old

echo '{"name": "Alice", "age": 30}' | parsm '$name'
# Output: Alice

# Literal templates (static output)
echo '{"name": "Alice", "age": 30}' | parsm '[name]'
# Output: name

echo '{"name": "Alice", "age": 30}' | parsm '{name}'
# Output: name

# Filtering with field selectors
echo '{"name": "Alice", "age": 30}' | parsm 'age > 25'
# Output: {"name": "Alice", "age": 30}

# Combined filtering and templating
echo '{"name": "Alice", "age": 30}' | parsm 'age > 25 [${name} is ${age} years old]'
# Output: Alice is 30 years old

# Original input variable
echo '{"name": "Alice"}' | parsm '[Original: ${0} → Name: ${name}]'
# Output: Original: {"name": "Alice"} → Name: Alice

# CSV positional fields
echo 'Alice,30,Engineer' | parsm '[Employee: ${1}, Age: ${2}, Role: ${3}]'
# Output: Employee: Alice, Age: 30, Role: Engineer

# Nested JSON fields
echo '{"user": {"name": "Alice", "email": "alice@example.com"}}' | \
  parsm '[User: ${user.name}, Email: ${user.email}]'
# Output: User: Alice, Email: alice@example.com
```

## Field Selection

Extract specific fields with simple, unambiguous syntax - the most intuitive operation in parsm:

```bash
# Simple field extraction (bare identifiers)
echo '{"name": "Alice", "age": 30}' | parsm 'name'
# Output: "Alice"

echo '{"name": "Alice", "age": 30}' | parsm 'age'
# Output: 30

# Nested field access (dot notation)
echo '{"user": {"email": "alice@example.com"}}' | parsm 'user.email'
# Output: "alice@example.com"

echo '{"config": {"database": {"host": "localhost"}}}' | parsm 'config.database.host'
# Output: "localhost"

# Array element access (index notation)
echo '{"items": ["apple", "banana", "cherry"]}' | parsm 'items.0'
# Output: "apple"

echo '{"scores": [95, 87, 92]}' | parsm 'scores.1'
# Output: 87

# Complex nested structures
echo '{"users": [{"name": "Alice", "role": "admin"}]}' | parsm 'users.0.name'
# Output: "Alice"

# Special field names (quoted when needed)
echo '{"field name": "value"}' | parsm '"field name"'
# Output: "value"

echo '{"special-field": "data"}' | parsm "'special-field'"
# Output: "data"

# Works consistently across all formats
echo '{"package": {"name": "test"}}' | parsm 'package.name'           # JSON
echo 'package: {name: test}' | parsm 'package.name'                  # YAML  
echo '[package]\nname = "test"' | parsm 'package.name'               # TOML

# Quoted syntax works the same way
echo '{"package": {"name": "test"}}' | parsm '"package.name"'         # JSON
echo 'package: {name: test}' | parsm '"package.name"'                # YAML
echo '[package]\nname = "test"' | parsm '"package.name"'             # TOML

# Complex field names across formats
echo '{"dev-dependencies": {"my-lib": "1.0"}}' | parsm '"dev-dependencies.my-lib"'   # JSON
echo 'dev-dependencies:\n  my-lib: 1.0' | parsm '"dev-dependencies.my-lib"'         # YAML  
echo '[dev-dependencies]\nmy-lib = "1.0"' | parsm '"dev-dependencies.my-lib"'       # TOML

# Extract entire objects or arrays
echo '{"state": {"status": "running", "pid": 1234}}' | parsm 'state'
# Output: {"status": "running", "pid": 1234}

echo '[{"name": "Alice"}, {"name": "Bob"}]' | parsm 'name'
# Output:
# "Alice"
# "Bob"
```

**Key Benefits:**
- **Simplest syntax**: `name` extracts the "name" field - no quotes needed
- **Unambiguous**: Bare identifiers are ALWAYS field selectors, never filters  
- **Intuitive**: Works exactly as users expect for the most common operation
- **Powerful**: Supports nested objects, arrays, and complex data structures
- **Cross-format**: Same syntax works for JSON, YAML, TOML, and other formats
- **Flexible quoting**: Use quotes only when field names have special characters or spaces

**Quoting Rules:**
- **Unquoted**: `name`, `user.email`, `items.0` - for simple field names
- **Quoted**: `"field-name"`, `"field name"`, `"special.field"` - when needed for special characters or spaces  
- **Both work**: `package.name` and `"package.name"` are identical - use whichever you prefer

## Complete Examples

## Complete Examples

### JSON Processing

```bash
# Extract specific fields (simple syntax)
echo '{"name": "Alice", "age": 30}' | parsm 'name'
echo '{"user": {"email": "alice@example.com"}}' | parsm 'user.email'

# Basic filtering
echo '{"name": "Alice", "age": 30}' | parsm 'age > 25'

# Filter and format
echo '{"name": "Alice", "age": 30}' | parsm 'age > 25 [${name} is ${age} years old]'

# Complex nested data
echo '{"user": {"name": "Alice", "profile": {"verified": true}}}' | \
  parsm 'user.profile.verified == true [Verified user: ${user.name}]'

# Array processing
echo '{"users": [{"name": "Alice"}, {"name": "Bob"}]}' | parsm 'users.0.name'

# Extract entire objects
echo '{"users": [{"name": "Alice"}, {"name": "Bob"}]}' | parsm 'users'
```

### CSV Processing

```bash
# Filter CSV data
echo 'Alice,30,Engineer' | parsm 'field_1 > "25" [${1} works as ${3}]'

# Multiple conditions
users.csv | parsm 'field_1 > "25" && field_2 == "Engineer" [${1} (${2} years old)]'

# Include original data
echo 'Alice,30,Engineer' | parsm '[${0} → Name: ${1}, Age: ${2}]'
```

### Log Processing

```bash
# Filter error logs
echo 'level=error msg="DB connection failed" service=api' | \
  parsm 'level == "error" [[${level}] ${msg}]'

# Complex log filtering
logs.txt | parsm 'level == "error" && service == "payment" [${timestamp}: ${msg}]'

# Performance monitoring
app.log | parsm 'duration > 1000 [Slow request: ${path} took ${duration}ms]'
```

### YAML/TOML Processing

```bash
# Extract configuration values
cat Cargo.toml | parsm 'package.name'                    # Get package name
cat Cargo.toml | parsm 'package.version'                 # Get version
cat Cargo.toml | parsm '"dependencies.serde_json"'       # Get dependency version

# Filter configuration
config.yaml | parsm 'database.enabled == true [DB: ${database.host}:${database.port}]'

# Convert format with nested access
echo 'name: Alice\nconfig: {debug: true}' | parsm '[${name}: debug=${config.debug}]'

# Extract configuration sections
config.toml | parsm '"server"'                           # Get entire server section
config.toml | parsm '"dev-dependencies"'                 # Get dev dependencies

# Real-world Cargo.toml examples
cat Cargo.toml | parsm 'package.description'            # Project description
cat Cargo.toml | parsm '"dependencies.clap"'             # Specific dependency
cat Cargo.toml | parsm 'package.keywords'                # Keywords array
```

### Multi-line Processing

```bash
# Process log files
tail -f app.log | parsm 'level == "error" [${date}: ${msg}]'

# Filter and transform data (alternative braced syntax also supported)
cat users.csv | parsm 'field_1 > "21" [{"name": "${1}", "age": ${2}}]'

# Real-time monitoring
docker stats --format "table {{.Name}},{{.CPUPerc}}" | \
  parsm 'field_1 ~ "%" [Container ${1} using ${2} CPU]'
```

## Advanced Features

### Complex Boolean Logic

```bash
# Multiple conditions
parsm 'name == "Alice" && (age > 25 || active == true)'

# Negation
parsm '!(status == "disabled" || role == "guest")'

# String operations
parsm 'email ~ "@company.com" && name ^= "A"'
```

### Error Handling

- **First line errors**: Fatal (format detection failure)
- **Subsequent errors**: Warnings with continued processing
- **Missing fields**: Warnings for templates, silent for filters

### Performance

- **Streaming**: Processes line-by-line for constant memory usage
- **Format detection**: Automatic with intelligent fallback
- **Large files**: Efficient processing of gigabyte-scale data

## Command Line Interface

```bash
parsm [OPTIONS] [FILTER] [TEMPLATE]

Arguments:
  [FILTER]     Filter expression (optional)
  [TEMPLATE]   Template expression for output formatting (optional)

Options:
  --examples   Show comprehensive usage examples
  -h, --help   Print help information
  -V, --version Print version information
```

### Usage Patterns

```bash
# Just parsing (convert to JSON)
cat data.yaml | parsm

# Field extraction (most common - simple syntax)
cat data.json | parsm 'name'
cat data.json | parsm 'user.email'

# Filtering only  
cat data.json | parsm 'age > 25'

# Template only (simple variable)
cat data.csv | parsm '$name'

# Template only (complex formatting)
cat data.csv | parsm '[${1}: ${2}]'

# Filter and template
cat data.log | parsm 'level == "error" [[${timestamp}] ${msg}]'

# Literal text output
cat data.json | parsm '[User Profile]'
```

## Comparison with Other Tools

| Feature | parsm | jq | awk | sed |
|---------|-------|----|----- |----- |
| **Multi-format input** | ✅ JSON, CSV, YAML, TOML, logfmt, text | JSON only | Text | Text |
| **Auto-detection** | ✅ Automatic | Manual | Manual | Manual |
| **Field extraction** | ✅ Simple `name` syntax | ✅ `.name` syntax | Limited | No |
| **Filter syntax** | ✅ Simple expressions | JQ query language | Programming | Regex |
| **Template output** | ✅ `${field}` syntax | ✅ Complex | ✅ `${1}, ${2}` | Limited |
| **Learning curve** | ✅ Low | Medium-High | High | Medium |
| **Boolean logic** | ✅ `&&`, `\|\|`, `!` | ✅ Complex | ✅ Programming | Limited |
| **Nested fields** | ✅ `user.email` | ✅ `.user.email` | Limited | No |
| **Performance** | Good | Excellent | Excellent | Excellent |

### When to use parsm

- **Field extraction**: When you need simple `name` syntax instead of jq's `.name`
- **Multi-format data**: When working with mixed JSON, CSV, YAML, etc.
- **Simple filtering**: When jq syntax is too complex
- **Quick transformations**: When awk programming is overkill  
- **Log processing**: Especially structured logs (JSON, logfmt)
- **Data exploration**: Quick inspection and filtering of structured data
- **Intuitive syntax**: When you want field access to "just work" without quotes or dots

### Migration from other tools

```bash
# From jq
jq '.name' data.json              → parsm 'name' < data.json
jq '.user.email' data.json        → parsm 'user.email' < data.json
jq 'select(.age > 25)' data.json  → parsm 'age > 25' < data.json

# From awk  
awk '$2 > 25' data.csv           → parsm 'field_1 > "25"' < data.csv
awk '{print $1, $2}' data.txt    → parsm '{${1} ${2}}' < data.txt

# From grep + cut
grep "error" logs | cut -d' ' -f3  → parsm 'word_0 == "error" ${3}' < logs
```

## Architecture Overview

### Data Flow
```
Input → Auto-detect Format → Parse → Normalize to JSON → Filter → Template → Output
```

### Components

- **Parser**: Auto-detects and parses multiple formats
- **Filter Engine**: Evaluates boolean expressions
- **Template Engine**: Renders output with field interpolation  
- **DSL**: Simple domain-specific language for expressions

### Key Design Decisions

1. **Unambiguous field selection**: Bare identifiers like `name` are always field selectors
2. **JSON normalization**: All formats convert to JSON for uniform processing
3. **Streaming processing**: Line-by-line for memory efficiency
4. **Automatic format detection**: Users don't specify input format
5. **Simple syntax**: Easy to learn and remember, prioritizing the most common operations
6. **Error tolerance**: Continues processing on non-fatal errors

## Contributing

1. Fork the repository
2. Create a feature branch: `git checkout -b feature-name`
3. Add tests for new functionality
4. Ensure all tests pass: `cargo test`
5. Run formatting: `cargo fmt`
6. Run linting: `cargo clippy`
7. Submit a pull request

### Development

```bash
# Build
cargo build

# Test
cargo test

# Run with examples
cargo run -- --examples

# Test with sample data
echo '{"name": "Alice", "age": 30}' | cargo run -- 'age > 25' '{${name}: ${age}}'
```

## License

[LICENSE](LICENSE)

## Changelog

See [CHANGELOG.md](CHANGELOG.md) for version history.
