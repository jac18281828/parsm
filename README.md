# parsm - **Parse 'Em** - Multi-Format Data Processor Architecture

## Overview

`parsm` is a powerful CLI tool that combines multi-format data parsing with advanced filtering and templating capabilities. It's designed to understand structured text better than traditional tools like `sed` or `awk`.

## Architecture Components

### 1. Multi-Format Parser (`parse.rs`)

**Purpose**: Auto-detects and parses multiple data formats
**Formats Supported**: JSON, CSV, YAML, TOML, logfmt, plain text

```rust
// Key types
pub enum Format { Json, Csv, Toml, Yaml, Logfmt, Text }
pub enum ParsedLine { Json(Value), Csv(StringRecord), ... }
pub struct StreamingParser { format: Option<Format> }
```

**How it works**:
1. **Auto-detection**: Uses heuristics to detect format from first line
2. **Format persistence**: Once detected, all subsequent lines use same parser
3. **Streaming**: Processes line-by-line for memory efficiency
4. **Normalization**: Converts all formats to JSON for uniform processing

### 2. Filter Engine (`filter.rs`)

**Purpose**: Evaluates boolean expressions against parsed data

```rust
// Core AST types
pub enum FilterExpr {
    And(Box<FilterExpr>, Box<FilterExpr>),
    Or(Box<FilterExpr>, Box<FilterExpr>),
    Not(Box<FilterExpr>),
    Comparison { field: FieldPath, op: ComparisonOp, value: FilterValue },
}
```

**Features**:
- **Nested field access**: `user.email`, `data.metrics.cpu`
- **Rich operators**: `==`, `!=`, `<`, `>`, `contains`, `startswith`, `endswith`, `matches`
- **Boolean logic**: `&&`, `||`, `!`
- **Type-aware comparisons**: Handles strings, numbers, booleans, null

### 3. Template Engine (`filter.rs`)

**Purpose**: Formats output using field interpolation with `$` syntax

```rust
pub struct Template { items: Vec<TemplateItem> }
pub enum TemplateItem {
    Field(FieldPath),    // $name, ${user.email}, $1, $$
    Literal(String),     // Plain text
}
```

**Template Syntax:**
- **`$$`** - Entire original input line
- **`$1`, `$2`, `$3`** - Indexed fields (1-based for user convenience)
- **`$name`** - Simple field names 
- **`${name}`** - Braced field names for complex expressions
- **`$name.field`** - Nested field access
- **`$100`** - Literal dollar amounts (large numbers treated as literals)

### 4. DSL Parser (`dsl_parser.rs`)

**Purpose**: Converts user commands into AST using Pest parser

**Grammar** (`parsm.pest`):
- **Expressions**: `name == "Alice" && age > 25`
- **Templates**: `$name is $age years old`
- **Combined**: `age > 25 $name: $age`

### 5. Integration Layer (`lib.rs`, `main.rs`)

**Purpose**: Ties all components together with high-level API

## Data Flow

```
Input Line → Auto-detect Format → Parse to AST → Convert to JSON
     ↓
Apply Filter → Pass/Fail → Apply Template → Output
```

## Usage Examples

### Basic Filtering
```bash
# Filter JSON
echo '{"name": "Alice", "age": 30}' | parsm 'name == "Alice"'

# Filter CSV (auto-indexed as field_0, field_1, etc.)
echo 'Alice,30,Engineer' | parsm 'field_1 > "25"'

# Filter logfmt logs
echo 'level=error msg="timeout"' | parsm 'level == "error"'
```

### Advanced Features
```bash
# Complex boolean logic
parsm 'name == "Alice" && (age > 25 || active == true)'

# Template formatting with indexed fields (CSV)
echo 'Alice,30,Engineer' | parsm 'field_1 > "25"' '$1 is $2 years old'

# Template with entire input
echo 'Alice,30,Engineer' | parsm '$$ → Name: $1'

# Nested field access (JSON)
echo '{"user": {"name": "Alice", "email": "alice@co.com"}}' | \
  parsm 'user.email contains "@co.com"' 'User: ${user.name} <${user.email}>'

# Mixed template syntax
parsm 'age > 25' 'Hello $name, you are $age - Original: $$'

# Literal dollar amounts
parsm 'price > 50' 'Item costs $100 (field: $price)'
```

## Template Syntax Reference

### Field Access Patterns

| Syntax | Description | Example |
|--------|-------------|---------|
| `$$` | Entire original input line | `$$ (modified)` |
| `$1`, `$2`, `$3` | Indexed fields (1-based) | `$1: $2` → `Alice: 30` |
| `$name` | Simple field names | `Hello $name` |
| `${name}` | Braced field syntax | `${user.email}` |
| `$name.field` | Nested field access | `$user.profile.bio` |
| `$100` | Literal dollar (large numbers) | `Cost: $100` |

### Field Mapping by Format

| Format | Field Names | Example Input | Available Fields |
|--------|-------------|---------------|------------------|
| **CSV** | `field_0`, `field_1`, `field_2` | `Alice,30,Engineer` | `$1`→`field_0`, `$2`→`field_1`, `$$`→`Alice,30,Engineer` |
| **JSON** | Original field names | `{"name":"Alice","age":30}` | `$name`, `$age`, `$$` |
| **Text** | `word_0`, `word_1`, `word_2` | `hello world test` | `$1`→`word_0`, `$2`→`word_1`, `$$` |
| **logfmt** | Key names from pairs | `level=info msg=test` | `$level`, `$msg`, `$$` |

### Template Examples

```bash
# CSV with indexed access
echo 'John,25,Designer' | parsm '$1 ($2) - $3'
# Output: John (25) - Designer

# JSON with named fields  
echo '{"user":"Alice","score":95}' | parsm 'Score: $score for $user'
# Output: Score: 95 for Alice

# Mixed syntax with original input
echo 'Alice,30' | parsm 'Processed $$ → $1 is $2'
# Output: Processed Alice,30 → Alice is 30

# Literal dollars
echo 'item,50' | parsm 'Product $1 costs $50 (field: $2)'
# Output: Product item costs $50 (field: 50)
```

## Key Design Decisions

### 1. **Auto-Detection Priority**
```
JSON → YAML → TOML → logfmt → CSV → Text
```
- Most specific formats first
- Text as fallback (always succeeds)

### 2. **JSON Normalization**
All formats convert to JSON for uniform filtering:
- **CSV**: `{"field_0": "Alice", "field_1": "30", "_array": ["Alice", "30"]}`
- **Text**: `{"word_0": "hello", "word_1": "world", "_array": ["hello", "world"]}`
- **logfmt**: `{"level": "info", "msg": "test"}`

### 3. **Streaming Architecture**
- Line-by-line processing
- Constant memory usage
- Early exit on filter failures
- Error tolerance (warns but continues)

### 4. **Error Handling Strategy**
- **First line failure**: Fatal (format detection failed)
- **Subsequent line failure**: Warning + continue
- **Parse errors**: Detailed error messages
- **Filter errors**: Graceful fallback

## Extension Points

### Adding New Formats
1. Add variant to `Format` enum
2. Implement detection heuristic in `detect_format()`
3. Add parsing function (e.g., `parse_xml()`)
4. Add conversion to JSON in `convert_to_json()`

### Adding New Operators
1. Add variant to `ComparisonOp` enum
2. Update grammar in `parsm.pest`
3. Implement evaluation in `FilterEngine::evaluate_comparison()`
4. Add parser support in `parse_comparison_op()`

### Adding Template Functions
```rust
// Future enhancement: template functions
// $name | uppercase, $age | format("years: {}")
pub enum TemplateItem {
    Field(FieldPath),
    Function(String, Vec<TemplateItem>), // New
    Literal(String),
}
```

## Performance Characteristics

- **Memory**: O(1) - streaming processing
- **CPU**: O(n) where n = number of input lines
- **Format detection**: O(1) per format type
- **Filter evaluation**: O(d) where d = expression depth

## Testing Strategy

### Unit Tests
- Format detection accuracy
- Filter expression evaluation
- Template rendering
- AST parsing correctness

### Integration Tests  
- End-to-end processing pipelines
- Mixed format handling
- Error condition handling
- Performance with large datasets

## Future Enhancements

1. **Header-aware CSV**: Use first row as field names
2. **Regex matching**: Full regex support for `matches` operator
3. **Template functions**: Formatting, string manipulation
4. **Multiple output formats**: YAML, CSV, TSV output
5. **Configuration files**: Custom format detection rules
6. **Parallel processing**: Multi-threaded line processing
7. **SQL-like syntax**: `SELECT name FROM data WHERE age > 25`

## Building and Running

```bash
# Build
cargo build --release

# Test  
cargo test

# Run examples
echo '{"name": "Alice", "age": 30}' | cargo run -- 'age > 25' '$name: $age'
```

This architecture provides a solid foundation for a powerful data processing tool that combines the flexibility of format auto-detection with the expressiveness of a domain-specific filtering and templating language.