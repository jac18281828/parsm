0.9.0 (2026-09-28)

* **Breaking**
  - convert mode (no expression) writes one line of compact JSON per record
    instead of echoing the input; a CSV or text row without a header
    converts to a JSON array, and a CSV header row no longer converts to a
    record of its own
  - a quoted field selector (`"user.name"`) now reads as one literal key,
    dots included, instead of splitting on `.`; use the bare form
    (`user.name`) for the nested path
  - `~` is contains-only and rejects a `/pattern/` regex literal (use `~=`
    for a regex); a `~=` pattern - literal, or a number or boolean
    stringified the same way `*=`/`^=`/`$=` already did - compiles once
    when the expression is parsed, and an invalid pattern is a parse error
    instead of a silent non-match
  - the CLI's two-argument form (`parsm FILTER TEMPLATE`) requires the
    first argument to resolve to a filter and the second to a template;
    anything else is an error naming the argument and what it parsed as
    instead of guessing
  - only one format flag (`--json`/`--yaml`/`--csv`/`--toml`/`--logfmt`/
    `--text`) is accepted at a time (a second is a usage error, exit 2),
    and a forced format's first record failing is fatal instead of falling
    through to another format
  - a missing field is silent everywhere: a filter comparison is false, a
    truthy check is false, a template variable renders empty, and a field
    selector prints nothing for that record
  - JSON and TOML objects keep their input key order through conversion
    instead of being re-sorted
  - a CSV header's field names key the record view by their written case
    plus a lower-case alias, instead of always lower-casing them; a ragged
    row keeps every field past the header under `field_N`
* text records expose `${1}`, `${2}`, … as 1-based positional fields, the
  same convention CSV already used (`field_0`, `field_1`, … stay available
  as the 0-based legacy form for both)
* a YAML document that is a top-level sequence yields one record per item,
  the same way a JSON array already did
* a CSV quoted field may span up to 64 lines; past that its row fails
  instead of holding the read open until end of input
* a JSON, YAML, TOML or logfmt object's own keys are exposed as-is; every
  record - whatever its format - now carries its own source text as `$0`
* a later line that is not valid UTF-8 warns and is skipped in every
  format, instead of only some; one before the first record is fatal
* format detection, record reading and output are unified behind one
  detector (`src/detect.rs`), one record pipeline (`src/records.rs`,
  `src/documents.rs`) and one writer (`src/pipeline.rs`) for every format
* `bin/integration_test.py` is retired; `bin/quick_test.sh` and the Rust
  integration test suite (`tests/`) are the only test harnesses
* fixed a panic when a multibyte character straddled byte 100 of the input, and a panic
  on an invalid `RUST_LOG`; tracing output goes to stderr
* JSON Lines, logfmt, text and CSV stream: each record prints as it arrives instead of
  after end of input
* a pretty-printed JSON array reads in linear time (12 MB: 55.6s to 0.32s); CSV reads about
  twice as fast
* short prose with a comma (`Hello, world`) reads as text, and logfmt is detected only when
  the line parses as logfmt

0.8.3 (2026-07-08)

* removed the legacy hand-rolled fallback parser (`fallback.rs`, ~800 lines) that silently
  rescued pest grammar failures with different, sometimes-wrong semantics; pest is now the
  sole parser for every input
* fixed several silent-wrong-output bugs uncovered by that removal:
  - bare `~` contains-operator was not recognized (`email ~ "@example.com"` silently failed)
  - `${cond?a:b}` template conditionals were completely broken (the literal grammar rule
    greedily consumed the required `:`, and the parser had no arm for the rule at all)
  - field-to-field comparisons (`a == b`) compared against the literal field name string,
    not the field's actual value
  - string literals could not contain escaped quotes (`\"`)
  - regex flags (`i`/`m`/`s`/`x`) were parsed but silently dropped before reaching the match
    engine
  - top-level `$NNN` / `${N}` template literals (e.g. `$20`, `${0}`) were rejected
  - a template nesting `[...]` inside `{...}` (e.g. `{[${level}] ${msg}}`) failed to parse
  - bare `!field` (no trailing `?`) is now rejected consistently in every position, instead
    of only some
* hardened `bin/quick_test.sh` / `bin/integration_test.py` to assert on actual output content
  instead of just exit code — the previous checks could report "all passing" while several of
  the bugs above were silently producing wrong output underneath
* added a fuzz sweep (65 adversarial inputs) guarding the top-level grammar against panics
* no measured change in parsing performance: benchmarked via `bin/microbenchmark.py` against
  the prior release binary across every supported format and input size, and results were
  within noise in both directions. The fallback parser was only ever invoked for inputs pest
  already rejected, so removing it simplifies the code path without changing the common case's
  throughput
* fixed a silent-empty-output bug where multi-document JSON input (JSON Lines) through a
  filter or field selector was misdetected as headered CSV and produced no output with exit
  code 0
* added `-f`/`--file` to read input from one or more files instead of stdin; repeatable,
  each file is detected and processed independently, and `-` means stdin
* relabeled the first positional argument from `[FILTER]` to `[EXPR]` in `--help` output to
  reflect that it can be a field selector, filter, template, or filter+template

0.8.2 (2025-07-06)

* docs improvements

0.8.1 (2025-07-06)

* major refactoring supporting document parsing (CSV, YAML, JSON, TOML)
* cleanup of parser and general code
* implementation of regex operation
* many fixes and improvements

0.2.0 (2025-06-28)

* initial implementation - general dsl and functionality