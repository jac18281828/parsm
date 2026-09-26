# Syntax Reference

Every `parsm` argument is one expression, or two arguments (a filter and a
template). This is the reference for both; the README links here for a
quick start.

## Expression kinds

An expression is one of four kinds:

- **field selector** — extracts one field: `name`, `user.email`
- **filter** — a boolean condition on the record: `age > 25`
- **template** — output formatting: `[${name} is ${age}]`
- **filter with template** — a filter and a template together:
  `age > 25 [${name} is ${age}]`

Given no expression, `parsm` converts: it writes each record as one line of
JSON. See [Convert mode](#convert-mode).

A bare field name (`name`) is always a field selector, never a filter or a
truthy check. Filters require an explicit operator; a truthy check requires
the explicit `?` suffix (`name?`).

The two-argument form (`parsm FILTER TEMPLATE`) takes a filter in the first
position and a template in the second; either argument left empty (`''`)
is skipped. Anything else in either position is an error naming the
argument and what it parsed as instead:

```console
$ parsm '$name' '[${age}]' < /dev/null
Error parsing filter and template expression: argument 1 ('$name') is not a filter expression - it parsed as a template
```

## Field selectors

```bash
echo '{"name": "Alice"}' | parsm 'name'
# Output: Alice
```

- **Bare**: `name` — one identifier.
- **Dotted**: `user.email` — a nested path, one component per `.`.
- **Array index**: `items.0` — 0-based; `items.1` is the second element.
- **Quoted**: `"user.name"` or `'user.name'` — one literal key, `.`s
  included; use this when the key itself contains a `.` or another
  character a bare path would split on. `\"` and `\'` escape a quote inside
  the matching quote style.

```bash
echo '{"user.name": "dotted key", "user": {"name": "nested"}}' | parsm '"user.name"'
# Output: dotted key
echo '{"user": {"name": "nested"}}' | parsm 'user.name'
# Output: nested
echo '{"items": ["a", "b", "c"]}' | parsm 'items.1'
# Output: b
```

A field selector on an object or array value prints its pretty-printed
JSON; a missing field prints nothing. See [Missing fields and
warnings](#missing-fields-and-warnings).

## Filters

Every comparison operator, from `src/dsl/operators.rs`:

| Operator | Meaning |
|---|---|
| `==` | equal |
| `!=` | not equal |
| `<`, `<=`, `>`, `>=` | numeric comparison (each side is parsed as a number; a numeric string works) |
| `*=` | contains |
| `~` | contains (same as `*=`) |
| `^=` | starts with |
| `$=` | ends with |
| `~=` | regex match |

```bash
echo '{"age": 30}' | parsm 'age > 25'
# Output: {"age": 30}
echo '{"email": "alice@example.com"}' | parsm 'email ~ "@example.com"'
# Output: {"email": "alice@example.com"}
```

`*=`, `^=`, `$=` and `~` compare as text; a non-string field is stringified
first, so `port *= "80"` matches the field `8080`:

```bash
echo '{"port": 8080}' | parsm 'port *= "80"'
# Output: {"port": 8080}
```

`~` and `*=` both mean contains; neither accepts a `/pattern/` regex
literal — that is `~=`'s job, and using one after `~` or `*=` is an error
naming the fix:

```console
$ echo '{"name": "Alice"}' | parsm 'name ~ /Ali.e/'
Error parsing expression:  --> 1:8
  |
1 | name ~ /Ali.e/
  |        ^---
  |
  = '~' is contains, not regex match - use '~=' for a regex pattern
```

### Regex (`~=`)

A `~=` pattern is a quoted string or a `/pattern/flags` literal. Either
form compiles once, when the expression is parsed; an invalid pattern is a
parse error, not a silent non-match. Flags are any combination of `i`
(case-insensitive), `m` (multi-line `^`/`$`), `s` (`.` matches newline) and
`x` (ignore whitespace and `#` comments in the pattern).

```bash
echo '{"name": "ALICE"}' | parsm 'name ~= /alice/i'
# Output: {"name": "ALICE"}
```

```console
$ echo '{"a": "x"}' | parsm 'a ~= "(["'
Error parsing expression:  --> 1:6
  |
1 | a ~= "(["
  |      ^---
  |
  = invalid regex pattern '([': regex parse error:
    ([
     ^
error: unclosed character class
```

A number or boolean on the right of `~=` is stringified into the pattern,
the same as `*=`/`^=`/`$=`.

### Boolean logic and truthiness

`&&`, `||` and `!` combine comparisons and parenthesized groups. A bare
field is never a boolean operand; a truthy check on a field's own value
needs the explicit `?` suffix:

```bash
echo '{"active": true}' | parsm 'active?'
# Output: {"active": true}
echo '{"active": true, "verified": true}' | parsm 'active? && verified?'
# Output: {"active": true, "verified": true}
```

`!` negates a parenthesized condition or a truthy check; a bare `!field`
fails, naming the fix:

```console
$ echo '{"active": true}' | parsm '!active'
Error parsing expression: bare '!active' is not supported - negation requires the explicit truthy check '!active?'
```

A field is truthy unless it is `null`, `false`, `0`, an empty string, an
empty array, an empty object, or one of the strings `false`/`f`/`0`/`no`/
`off` (case-insensitive). A missing field is falsy.

### Field-to-field comparison

The right-hand side of a comparison can itself be a field path, resolved
against the same record:

```bash
echo '{"a": 1, "b": 1}' | parsm 'a == b'
# Output: {"a": 1, "b": 1}
```

## Templates

A template is `[...]` or `{...}` — the two forms accept the same content
and render the same way; `[...]` is preferred. Inside either:

- `$name` / `${name}` — a field's value. `${a.b}` reaches a nested field
  the same way a field selector does.
- `${0}` — the record's own source text (its `$0`), always requires
  braces.
- `${1}`, `${2}`, … — 1-based positional field access for CSV and text
  records (`${1}` is the first column or word). This is distinct from a
  field selector's array index, which is 0-based (`items.0` is the first
  element).
- `${a?x:y}` — a conditional: `x` if `a` is truthy, `y` otherwise. Either
  branch may itself hold template content, `${...}` variables included.
- `$20`, `$0`, `$100` — a `$` followed only by digits and nothing else is a
  literal dollar amount, not a variable, at any digit count.
- `\[` and `\]` — inside `[...]`, an escaped literal bracket. A `[...]`
  span that is itself balanced is literal content (its own variables still
  interpolate); an unescaped, unbalanced bracket is a parse error naming
  the fix.

```bash
echo '{"name": "Alice", "age": 30}' | parsm '[${name} is ${age}]'
# Output: Alice is 30
echo '{"name": "Alice", "age": 30}' | parsm '{${name} is ${age}}'
# Output: Alice is 30
echo '{"name": "Alice"}' | parsm '$name'
# Output: Alice
echo 'Alice,30' | parsm '[${0}]'
# Output: Alice,30
echo 'Alice,30,Engineer' | parsm '[${1} (${2})]'
# Output: Alice (30)
echo '{"admin": true, "name": "Alice"}' | parsm '${admin?Admin:Guest}'
# Output: Admin
echo '{"name": "Bob"}' | parsm '[I have $20, ${name} has $100]'
# Output: I have $20, Bob has $100
echo '{"a": "x"}' | parsm '[literal \[bracket\] here ${a}]'
# Output: literal [bracket] here x
```

## Format detection

Detection reads the first non-blank line that does not start with `#` (the
probe) and applies this precedence table; the first row that matches
chooses the format. Every line detection reads is replayed to the chosen
format, leading `#` lines included.

| Row | Format | Chosen when |
|---|---|---|
| 1 | JSON | the first JSON value parses (it may span lines) and only whitespace or further `{…}`/`[…]` values follow it on its last line |
| 2 | logfmt | every whitespace-separated token is `key=value` and the line parses as logfmt |
| 3 | TOML | a `key = value` line, or a `[table]` line followed by one, that parses |
| 4 | YAML | a block mapping `key: value`, a `- item`, `---`, or a `{…}` flow map |
| 5 | CSV | the line parses as a CSV row of 2+ fields with every separator comma tight (no trailing whitespace), or with a loose separator comma when the next line parses as a CSV row of the same field count |
| 6 | text | everything else |

A guessed YAML or TOML input whose first document fails to parse is read
as text instead.

A format flag (`--json`, `--yaml`, `--csv`, `--toml`, `--logfmt`, `--text`)
skips detection and forces that format; at most one is accepted. A forced
format's first record failing is fatal, unlike a guessed one falling
through to text:

```console
$ echo 'hello world' | parsm --json
Error processing stream from '-': input is not json: line 1: expected value
```

```console
$ parsm --json --yaml < /dev/null
error: the argument '--json' cannot be used with '--yaml'

Usage: parsm --json [EXPR] [TEMPLATE]

For more information, try '--help'.
```

## Records and `$0` per format

Every record carries its own source text as `$0`, whatever its format:

- **JSON, YAML, TOML, logfmt**: an object's own keys, plus `$0`.
- **CSV**: `1`, `2`, … (1-based) and `field_0`, `field_1`, … (0-based) hold
  the columns. A header row's names are keys too, as written, plus a
  lower-case alias when that differs from the written name; the header row
  itself is not a record. A quoted field may span multiple lines, up to 64;
  past that its row fails rather than holding the read open until EOF.
- **Text**: `1`, `2`, … (1-based) and `word_0`, `word_1`, … (0-based) hold
  the whitespace-split words.

```bash
printf 'Name,Age\nTom,45\n' | parsm '[${Name} / ${name}]'
# Output: Tom / Tom
```

## Streaming

JSON, logfmt, CSV and text records are written as they complete; a filter
or template runs against each record as it arrives rather than after the
whole input is read. YAML and TOML read to the end of input first: YAML
yields one record per `---`-separated document, and one record per item of
a top-level sequence; TOML is always one record.

## Convert mode

With no expression, `parsm` writes each record as one line of compact
JSON:

```bash
echo 'name: Alice' | parsm
# Output: {"name":"Alice"}
echo 'Alice,30' | parsm
# Output: ["Alice","30"]
printf 'name,age\nTom,45\n' | parsm
# Output: {"name":"Tom","age":"45"}
```

A CSV or text row without a header converts to a JSON array of its fields;
with a header, to an object keyed by the header names (a header row itself
never converts to a record).

## Missing fields and warnings

A field selector, filter or template referring to a field the record
lacks treats it as absent: a filter comparison is false, a truthy check is
false, a template variable renders empty, and a field selector prints
nothing for that record.

```bash
printf '{"a": 1}\n{"b": 2}\n' | parsm 'a'
# Output: 1
```

A record that fails to parse after at least one other record has already
parsed is a warning on stderr, and is skipped; the record before or after
it still reaches stdout:

```console
$ printf 'a=1\nnot logfmt\nb=2\n' | parsm
{"a":"1"}
Warning: failed to parse line 2: expected key=value pairs
{"b":"2"}
```

A failure before anything has parsed is fatal instead.
