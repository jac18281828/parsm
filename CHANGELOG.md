0.8.4 (2026-07-07)

* added `-f`/`--file` to read input from one or more files instead of stdin; repeatable,
  each file is detected and processed independently, and `-` means stdin
* relabeled the first positional argument from `[FILTER]` to `[EXPR]` in `--help` output to
  reflect that it can be a field selector, filter, template, or filter+template

0.8.3 (2026-07-05)

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

0.8.2 (2025-07-06)

* docs improvements

0.8.1 (2025-07-06)

* major refactoring supporting document parsing (CSV, YAML, JSON, TOML)
* cleanup of parser and general code
* implementation of regex operation
* many fixes and improvements

0.2.0 (2025-06-28)

* initial implementation - general dsl and functionality