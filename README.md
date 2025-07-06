# parsm - **Parse 'Em** - An 'everything' parser, Sedder, Awkker, Grokker, Grepper

Parsm is the powerful command-line tool that understands structured text better than `sed`, `awk`, `grep` or `grok`.

<img src="eatcookie.jpg" alt="Eat more cookie!" width="25%">

## Overview

`parsm` automatically detects and parses multiple data formats (**JSON**, **CSV**, **YAML**, **TOML**, **logfmt**, and plain text) and provides powerful filtering and templating capabilities through an intuitive syntax.

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

# Examples
parsm --examples

# Extract a field
echo '{"name": "Alice"}' | parsm 'name'

# Nested fields
echo '{"user": {"email": "alice@example.com"}}' | parsm 'user.email'

# Filtering
echo '{"age": 30}' | parsm 'age > 25'

# Filter and format
echo '{"name": "Alice", "age": 30}' | parsm 'age > 25 [${name} is ${age}]'
```

## Supported Formats

- JSON
- CSV
- YAML
- TOML
- Logfmt
- Plain Text

## Force Format Parsing

| Flag     | Format   |
|----------|----------|
| `--json` | JSON     |
| `--yaml` | YAML     |
| `--csv`  | CSV      |
| `--toml` | TOML     |
| `--logfmt` | logfmt |
| `--text` | Plain Text |

## Syntax Reference

### Filters

- Comparison: `==`, `!=`, `<`, `<=`, `>`, `>=`
- String ops: `~` (contains), `^=` (starts with), `$=` (ends with)
- Boolean logic: `&&`, `||`, `!`
- Existence check: `field?`

Examples:

```bash
name == "Alice" && age > 25
email ~ "@example.com"
user.active?
```

### Templates

- Variables: `[${name}]` or `$name`
- Literal: `[name]`

Example:

```bash
parsm 'age > 25 [${name} is ${age}]'
```

### Field Selectors

- Simple: `name`
- Nested: `user.email`
- Quoted (special chars): `'special-field'`
- CSV/Text: `field_0`, `word_0`

## Examples

### JSON/YAML/TOML

```bash
cat Cargo.toml | parsm 'package.name'
echo '{"user": {"email": "alice@example.com"}}' | parsm 'user.email'
```

### CSV

```bash
echo 'Alice,30,Engineer' | parsm 'field_1 > "25" [${1} (${2})]'
```

### Logs

```bash
echo 'level=error msg="DB error"' | parsm 'level == "error" [${msg}]'
```

## CLI Usage

```bash
parsm [OPTIONS] [FILTER] [TEMPLATE]

Options:
  --examples     Show usage examples
  -h, --help     Show help
  -V, --version  Show version
```

## Comparison with Other Tools

| Feature          | parsm       | jq         | awk        | sed        |
|------------------|-------------|------------|------------|------------|
| Multi-format     | ✅ JSON, CSV, YAML, TOML, logfmt, text | JSON only  | Text       | Text       |
| Auto-detection   | ✅ Automatic | ❌ Manual  | ❌ Manual  | ❌ Manual  |
| Field extraction | ✅ Simple `name` syntax | ✅ `.name` syntax | Limited    | ❌ No       |
| Simple syntax    | ✅ Low       | Medium     | Complex    | Medium     |

## Development

- Build: `cargo build`
- Test: `cargo test`
- Lint: `cargo fmt && cargo clippy`

## Contributing

1. Fork repository
2. Create feature branch
3. Write tests and code
4. Run tests and lint checks
5. Submit a pull request

## License

See [LICENSE](LICENSE).

## Changelog

See [CHANGELOG.md](CHANGELOG.md).
