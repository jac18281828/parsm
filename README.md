# parsm - **Parse 'Em** - An 'everything' parser, Sedder, Awkker, Grokker, Grepper

Parsm is the powerful command-line tool that understands structured text better than sed, awk, grep or grok.

![cookie](eatcookie.jpg)

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

# Filter JSON data
echo '{"name": "Alice", "age": 30}' | parsm 'age > 25'

# Filter and format output
echo '{"name": "Alice", "age": 30}' | parsm 'age > 25' '{${name} is ${age} years old}'

# Extract specific fields
echo '{"name": "Alice", "age": 30}' | parsm '"name"'
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

### Templates
Templates are **always** enclosed in braces `{...}` and contain literal text with variable substitutions:

```bash
{${name} is ${age} years old}    # Variables with ${...}
{Hello $name}                    # Simple variables (non-numeric)  
{${0}}                          # Original input (requires braces)
{User: ${user.name}}            # Nested fields
{Price: $$100}                  # Literal dollar signs ($$)
```

### Variables
Variables use two formats depending on type:

| Syntax | Use Case | Example |
|--------|----------|---------|
| `${variable}` | Recommended for all variables | `${name}`, `${user.email}` |
| `${number}` | **Required** for numeric variables | `${0}`, `${1}`, `${2}` |
| `$variable` | Simple non-numeric variables only | `$name`, `$user` |

**Important**: `$0`, `$1`, etc. are treated as **literals** (like "$0 fee"), not variables. Only `${0}`, `${1}` access fields.

### Field Selectors
Extract specific fields using quoted strings:

```bash
"name"           # Simple field
"user.email"     # Nested field  
"field with spaces"  # Quoted when needed
```

### Examples

```bash
# Template with variables
echo '{"name": "Alice", "age": 30}' | parsm '{${name} is ${age} years old}'

# Original input variable  
echo '{"name": "Alice"}' | parsm '{Original: ${0} → Name: ${name}}'

# CSV positional fields
echo 'Alice,30,Engineer' | parsm '{Employee: ${1}, Age: ${2}, Role: ${3}}'

# Nested JSON fields
echo '{"user": {"name": "Alice", "email": "alice@example.com"}}' | \
  parsm '{User: ${user.name}, Email: ${user.email}}'

# Literal dollar signs
echo '{"item": "coffee", "price": 5}' | parsm '{${item} costs $$${price}}'
```

## Field Selection

Extract specific fields using quoted syntax:

```bash
# Extract single field
echo '{"name": "Alice", "age": 30}' | parsm '"name"'
# Output: "Alice"

# Extract nested field
echo '{"user": {"email": "alice@example.com"}}' | parsm '"user.email"'
# Output: "alice@example.com"

# Extract complex object
echo '{"state": {"status": "running", "pid": 1234}}' | parsm '"state"'
# Output: {"status": "running", "pid": 1234}

# Works with arrays too
echo '[{"name": "Alice"}, {"name": "Bob"}]' | parsm '"name"'
# Output:
# "Alice"
# "Bob"
```

## Complete Examples

### JSON Processing

```bash
# Basic filtering
echo '{"name": "Alice", "age": 30}' | parsm 'age > 25'

# Filter and format
echo '{"name": "Alice", "age": 30}' | parsm 'age > 25' '{${name} is ${age} years old}'

# Complex nested data
echo '{"user": {"name": "Alice", "profile": {"verified": true}}}' | \
  parsm 'user.profile.verified == true' '{Verified user: ${user.name}}'

# Field extraction
echo '{"users": [{"name": "Alice"}, {"name": "Bob"}]}' | parsm '"users"'
```

### CSV Processing

```bash
# Filter CSV data
echo 'Alice,30,Engineer' | parsm 'field_1 > "25"' '{${1} works as ${3}}'

# Multiple conditions
users.csv | parsm 'field_1 > "25" && field_2 == "Engineer"' '{${1} (${2} years old)}'

# Include original data
echo 'Alice,30,Engineer' | parsm '{${0} → Name: ${1}, Age: ${2}}'
```

### Log Processing

```bash
# Filter error logs
echo 'level=error msg="DB connection failed" service=api' | \
  parsm 'level == "error"' '{[${level}] ${msg}}'

# Complex log filtering
logs.txt | parsm 'level == "error" && service == "payment"' '{${timestamp}: ${msg}}'

# Performance monitoring
app.log | parsm 'duration > 1000' '{Slow request: ${path} took ${duration}ms}'
```

### YAML/TOML Processing

```bash
# Filter configuration
config.yaml | parsm 'database.enabled == true' '{DB: ${database.host}:${database.port}}'

# Convert format
echo 'name: Alice\nage: 30' | parsm '{${name} is ${age} years old}'

# Extract configuration sections
config.toml | parsm '"server"'
```

### Multi-line Processing

```bash
# Process log files
tail -f app.log | parsm 'level == "error"' '{${date}: ${msg}}'

# Filter and transform data
cat users.csv | parsm 'field_1 > "21"' '{{"name": "${1}", "age": ${2}}}'

# Real-time monitoring
docker stats --format "table {{.Name}},{{.CPUPerc}}" | \
  parsm 'field_1 ~ "%"' '{Container ${1} using ${2} CPU}'
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

# Filtering only  
cat data.json | parsm 'age > 25'

# Template only
cat data.csv | parsm '' '{${1}: ${2}}'

# Filter and template
cat data.log | parsm 'level == "error"' '{[${timestamp}] ${msg}}'

# Field selection
cat data.json | parsm '"user.email"'
```

## Comparison with Other Tools

| Feature | parsm | jq | awk | sed |
|---------|-------|----|----- |----- |
| **Multi-format input** | ✅ JSON, CSV, YAML, TOML, logfmt, text | JSON only | Text | Text |
| **Auto-detection** | ✅ Automatic | Manual | Manual | Manual |
| **Filter syntax** | Simple expressions | JQ query language | Programming | Regex |
| **Template output** | ✅ `${field}` syntax | ✅ Complex | ✅ `${1}, ${2}` | Limited |
| **Learning curve** | Low | Medium-High | High | Medium |
| **Boolean logic** | ✅ `&&`, `\|\|`, `!` | ✅ Complex | ✅ Programming | Limited |
| **Nested fields** | ✅ `user.email` | ✅ `.user.email` | Limited | No |
| **Performance** | Good | Excellent | Excellent | Excellent |

### When to use parsm

- **Multi-format data**: When working with mixed JSON, CSV, YAML, etc.
- **Simple filtering**: When jq syntax is too complex
- **Quick transformations**: When awk programming is overkill  
- **Log processing**: Especially structured logs (JSON, logfmt)
- **Data exploration**: Quick inspection and filtering of structured data

### Migration from other tools

```bash
# From jq
jq '.name' data.json              → parsm '"name"' < data.json
jq 'select(.age > 25)' data.json  → parsm 'age > 25' < data.json

# From awk  
awk '$2 > 25' data.csv           → parsm 'field_1 > "25"' < data.csv
awk '{print $1, $2}' data.txt    → parsm '{${1} ${2}}' < data.txt

# From grep + cut
grep "error" logs | cut -d' ' -f3  → parsm 'word_0 == "error"' '{${3}}' < logs
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

1. **JSON normalization**: All formats convert to JSON for uniform processing
2. **Streaming processing**: Line-by-line for memory efficiency
3. **Automatic format detection**: Users don't specify input format
4. **Simple syntax**: Easy to learn and remember
5. **Error tolerance**: Continues processing on non-fatal errors

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

[Add your license here]

## Changelog

See [CHANGELOG.md](CHANGELOG.md) for version history.