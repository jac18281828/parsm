# parsm

[![CI/CD Pipeline](https://github.com/jac18281828/parsm/actions/workflows/ci-cd.yml/badge.svg)](https://github.com/jac18281828/parsm/actions/workflows/ci-cd.yml)
[![Quick Test Suite](https://github.com/jac18281828/parsm/actions/workflows/integration-test.yml/badge.svg)](https://github.com/jac18281828/parsm/actions/workflows/integration-test.yml)
[![crates.io](https://img.shields.io/crates/v/parsm.svg)](https://crates.io/crates/parsm)

`parsm` (parse 'em) detects and parses JSON, CSV, YAML, TOML, logfmt, and
plain text, and applies one expression language for filtering and
templating - an alternative to reaching for `sed`, `awk`, `grep` or `jq` on
mixed or unknown input.

Given no expression, `parsm` converts each record to one line of JSON.
Given a filter, it writes each matching record's own source text; given a
template, it writes the template's rendered output instead.

## Installation

### From crates.io

```bash
cargo install parsm
```

### Prebuilt binaries

Each [release](https://github.com/jac18281828/parsm/releases) publishes
binaries for Linux, Windows and macOS (x86_64 and aarch64).

### From source

```bash
git clone https://github.com/jac18281828/parsm.git
cd parsm
cargo build --release
./target/release/parsm --examples
```

## Quick Start

```bash
echo '{"name": "Alice", "age": 30}' | parsm 'name'
# Output: Alice

echo '{"user": {"email": "alice@example.com"}}' | parsm 'user.email'
# Output: alice@example.com

echo '{"age": 30}' | parsm 'age > 25'
# Output: {"age": 30}

echo '{"name": "Alice", "age": 30}' | parsm 'age > 25 [${name} is ${age}]'
# Output: Alice is 30

parsm -f Cargo.toml 'package.name'
# Output: parsm
```

## Supported Formats

JSON, CSV, YAML, TOML, logfmt, plain text. Detection reads the input, never
the expression; see `doc/syntax.md` for the precedence table and how to
force a format.

## Syntax

An expression is a field selector (`name`), a filter (`age > 25`), a
template (`[${name} is ${age}]`), or a filter with a template
(`age > 25 [${name} is ${age}]`). See [`doc/syntax.md`](doc/syntax.md) for
the full reference: every operator, template variable form, and the format
detection rules.

## Examples

### JSON

```bash
echo '{"user": {"email": "alice@example.com"}}' | parsm 'user.email'
# Output: alice@example.com
```

### YAML / TOML

```bash
cat Cargo.toml | parsm 'package.name'
# Output: parsm
```

### CSV

```bash
echo 'Alice,30,Engineer' | parsm 'field_1 > "25" [${1} (${2})]'
# Output: Alice (30)
```

### Logfmt

```bash
echo 'level=error msg="DB error"' | parsm 'level == "error" [${msg}]'
# Output: DB error
```

## CLI

```console
Understands structured text better than sed or awk

Usage: parsm [OPTIONS] [EXPR] [TEMPLATE]

Arguments:
  [EXPR]      Expression: field selector, filter, template, or filter with a template (optional)
  [TEMPLATE]  Template expression for output formatting (optional)

Options:
  -f, --file <FILE>  Read input from FILE instead of stdin (repeatable; '-' = stdin)
      --examples     Show usage examples
      --json         Read input as json, skipping detection
      --yaml         Read input as yaml, skipping detection
      --csv          Read input as csv, skipping detection
      --toml         Read input as toml, skipping detection
      --logfmt       Read input as logfmt, skipping detection
      --text         Read input as text, skipping detection
  -h, --help         Print help
  -V, --version      Print version
```

## Comparison with Other Tools

| Capability | parsm | jq | awk | sed |
|---|---|---|---|---|
| Formats | JSON, CSV, YAML, TOML, logfmt, text | JSON | text | text |
| Format detection | automatic | n/a (JSON only) | manual | manual |
| Field extraction | `name`, `user.email` | `.name`, `.user.email` | positional (`$1`) | pattern-based |
| Templating | `[${name} is ${age}]` | `-r` with string interpolation | `printf` | none |

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md), including how to run the test suite
before pushing.

## License

See [LICENSE](LICENSE).
