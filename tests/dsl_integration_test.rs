//! Comprehensive DSL Integration Tests
//!
//! This module contains regression tests and comprehensive integration tests for the DSL parser.
//! These tests ensure that critical parsing distinctions and edge cases are preserved
//! across refactoring and modularization efforts.

mod common;

use common::command;
use parsm::dsl::parse_command;
use parsm::filter::{ComparisonOp, FilterExpr, FilterValue};

/// Run the built binary with `args` as its DSL arguments and `stdin_data` on
/// stdin. Stdout and stderr are captured verbatim (stdout's trailing
/// newline trimmed) alongside the exit code.
fn run_parsm(args: &[&str], stdin_data: &str) -> (String, String, i32) {
    let mut cmd = command();
    cmd.args(args);
    let output = common::run(cmd, stdin_data);
    (
        common::stdout_of(&output)
            .trim_end_matches('\n')
            .to_string(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
        output.status.code().expect("process exited with a code"),
    )
}

/// Test all comparison operators in DSL
#[test]
fn test_all_comparison_operators() {
    // Test equality operator (==)
    let result = parse_command(r#"name == "Alice""#).unwrap();
    assert!(result.filter.is_some());
    if let Some(FilterExpr::Comparison { field, op, value }) = result.filter {
        assert_eq!(field.parts, vec!["name"]);
        assert_eq!(op, ComparisonOp::Equal);
        assert_eq!(value, FilterValue::String("Alice".to_string()));
    } else {
        panic!("Expected equality comparison");
    }

    // Test not equal operator (!=)
    let result = parse_command(r#"name != "Bob""#).unwrap();
    assert!(result.filter.is_some());
    if let Some(FilterExpr::Comparison { field, op, value }) = result.filter {
        assert_eq!(field.parts, vec!["name"]);
        assert_eq!(op, ComparisonOp::NotEqual);
        assert_eq!(value, FilterValue::String("Bob".to_string()));
    } else {
        panic!("Expected not equal comparison");
    }

    // Test less than operator (<)
    let result = parse_command("age < 30").unwrap();
    assert!(result.filter.is_some());
    if let Some(FilterExpr::Comparison { field, op, value }) = result.filter {
        assert_eq!(field.parts, vec!["age"]);
        assert_eq!(op, ComparisonOp::LessThan);
        assert_eq!(value, FilterValue::Number(30.0));
    } else {
        panic!("Expected less than comparison");
    }

    // Test less than or equal operator (<=)
    let result = parse_command("age <= 30").unwrap();
    assert!(result.filter.is_some());
    if let Some(FilterExpr::Comparison { field, op, value }) = result.filter {
        assert_eq!(field.parts, vec!["age"]);
        assert_eq!(op, ComparisonOp::LessThanOrEqual);
        assert_eq!(value, FilterValue::Number(30.0));
    } else {
        panic!("Expected less than or equal comparison");
    }

    // Test greater than operator (>)
    let result = parse_command("age > 25").unwrap();
    assert!(result.filter.is_some());
    if let Some(FilterExpr::Comparison { field, op, value }) = result.filter {
        assert_eq!(field.parts, vec!["age"]);
        assert_eq!(op, ComparisonOp::GreaterThan);
        assert_eq!(value, FilterValue::Number(25.0));
    } else {
        panic!("Expected greater than comparison");
    }

    // Test greater than or equal operator (>=)
    let result = parse_command("age >= 25").unwrap();
    assert!(result.filter.is_some());
    if let Some(FilterExpr::Comparison { field, op, value }) = result.filter {
        assert_eq!(field.parts, vec!["age"]);
        assert_eq!(op, ComparisonOp::GreaterThanOrEqual);
        assert_eq!(value, FilterValue::Number(25.0));
    } else {
        panic!("Expected greater than or equal comparison");
    }

    // Test contains operator (*=)
    let result = parse_command(r#"name *= "lic""#).unwrap();
    assert!(result.filter.is_some());
    if let Some(FilterExpr::Comparison { field, op, value }) = result.filter {
        assert_eq!(field.parts, vec!["name"]);
        assert_eq!(op, ComparisonOp::Contains);
        assert_eq!(value, FilterValue::String("lic".to_string()));
    } else {
        panic!("Expected contains comparison");
    }

    // Test starts with operator (^=)
    let result = parse_command(r#"name ^= "Al""#).unwrap();
    assert!(result.filter.is_some());
    if let Some(FilterExpr::Comparison { field, op, value }) = result.filter {
        assert_eq!(field.parts, vec!["name"]);
        assert_eq!(op, ComparisonOp::StartsWith);
        assert_eq!(value, FilterValue::String("Al".to_string()));
    } else {
        panic!("Expected starts with comparison");
    }

    // Test ends with operator ($=)
    let result = parse_command(r#"name $= "ice""#).unwrap();
    assert!(result.filter.is_some());
    if let Some(FilterExpr::Comparison { field, op, value }) = result.filter {
        assert_eq!(field.parts, vec!["name"]);
        assert_eq!(op, ComparisonOp::EndsWith);
        assert_eq!(value, FilterValue::String("ice".to_string()));
    } else {
        panic!("Expected ends with comparison");
    }

    // Test regex operator (~=) - a string value under ~= compiles once at
    // parse time into CompiledRegex too, the same as a /pattern/ literal.
    let result = parse_command(r#"email ~= "@.*\.com$""#).unwrap();
    assert!(result.filter.is_some());
    if let Some(FilterExpr::Comparison { field, op, value }) = result.filter {
        assert_eq!(field.parts, vec!["email"]);
        assert_eq!(op, ComparisonOp::Regex);
        match value {
            FilterValue::Regex(compiled) => assert_eq!(compiled.pattern, "@.*\\.com$"),
            other => panic!("Expected compiled regex value, got {other:?}"),
        }
    } else {
        panic!("Expected regex comparison");
    }
}

/// Test operator parsing with different value types
#[test]
fn test_operators_with_different_value_types() {
    // String values
    let result = parse_command(r#"name == "Alice""#).unwrap();
    assert!(result.filter.is_some());

    // Numeric values
    let result = parse_command("age >= 18").unwrap();
    assert!(result.filter.is_some());

    // Boolean values
    let result = parse_command("active == true").unwrap();
    assert!(result.filter.is_some());
    if let Some(FilterExpr::Comparison { field, op, value }) = result.filter {
        assert_eq!(field.parts, vec!["active"]);
        assert_eq!(op, ComparisonOp::Equal);
        assert_eq!(value, FilterValue::Boolean(true));
    } else {
        panic!("Expected boolean comparison");
    }

    let result = parse_command("active != false").unwrap();
    assert!(result.filter.is_some());
    if let Some(FilterExpr::Comparison { field, op, value }) = result.filter {
        assert_eq!(field.parts, vec!["active"]);
        assert_eq!(op, ComparisonOp::NotEqual);
        assert_eq!(value, FilterValue::Boolean(false));
    } else {
        panic!("Expected boolean not equal comparison");
    }

    // Decimal numbers
    let result = parse_command("score <= 98.5").unwrap();
    assert!(result.filter.is_some());
    if let Some(FilterExpr::Comparison { field, op, value }) = result.filter {
        assert_eq!(field.parts, vec!["score"]);
        assert_eq!(op, ComparisonOp::LessThanOrEqual);
        assert_eq!(value, FilterValue::Number(98.5));
    } else {
        panic!("Expected decimal number comparison");
    }
}

/// Test operator precedence and spacing
#[test]
fn test_operator_precedence_and_spacing() {
    // Test that multi-character operators are parsed correctly even without spaces
    let result = parse_command("age>=25").unwrap();
    assert!(result.filter.is_some());
    if let Some(FilterExpr::Comparison {
        field: _,
        op,
        value: _,
    }) = result.filter
    {
        assert_eq!(op, ComparisonOp::GreaterThanOrEqual);
    } else {
        panic!("Expected >= operator parsing without spaces");
    }

    // Test that <= is parsed correctly
    let result = parse_command("age<=30").unwrap();
    assert!(result.filter.is_some());
    if let Some(FilterExpr::Comparison {
        field: _,
        op,
        value: _,
    }) = result.filter
    {
        assert_eq!(op, ComparisonOp::LessThanOrEqual);
    } else {
        panic!("Expected <= operator parsing without spaces");
    }

    // Test that != is parsed correctly
    let result = parse_command(r#"name!="Bob""#).unwrap();
    assert!(result.filter.is_some());
    if let Some(FilterExpr::Comparison {
        field: _,
        op,
        value: _,
    }) = result.filter
    {
        assert_eq!(op, ComparisonOp::NotEqual);
    } else {
        panic!("Expected != operator parsing without spaces");
    }

    // Test single character operators with spaces (required for disambiguation)
    let result = parse_command("age < 30").unwrap();
    assert!(result.filter.is_some());
    if let Some(FilterExpr::Comparison {
        field: _,
        op,
        value: _,
    }) = result.filter
    {
        assert_eq!(op, ComparisonOp::LessThan);
    } else {
        panic!("Expected < operator with spaces");
    }

    let result = parse_command("age > 25").unwrap();
    assert!(result.filter.is_some());
    if let Some(FilterExpr::Comparison {
        field: _,
        op,
        value: _,
    }) = result.filter
    {
        assert_eq!(op, ComparisonOp::GreaterThan);
    } else {
        panic!("Expected > operator with spaces");
    }
}

// Proof cases (prompt section 8): each documents one fixed behavior and
// goes red when its fix is reverted.

/// #1: a template variable follows the grammar's identifier rule, so
/// `$name.` is the field `name` followed by a literal ".", never a re-parse
/// of the matched text that swallows the dot into the variable name.
#[test]
fn proof_dollar_variable_stops_before_trailing_dot() {
    let (stdout, stderr, code) = run_parsm(&["[Hello $name.]"], r#"{"name":"Al"}"#);
    assert_eq!(code, 0, "stderr: {stderr}");
    assert_eq!(stdout, "Hello Al.");
}

/// #2: a braced template accepts a conditional, the same as a bracketed one.
#[test]
fn proof_braced_template_accepts_conditional() {
    let (stdout, stderr, code) = run_parsm(&["{${a?x:y}}"], r#"{"a":true}"#);
    assert_eq!(code, 0, "stderr: {stderr}");
    assert_eq!(stdout, "x");
}

/// #3: a balanced "[...]" pair inside a bracketed template is literal, its
/// variable still interpolating, mirroring a braced template's "[...]".
#[test]
fn proof_nested_brackets_in_bracketed_template() {
    let (stdout, stderr, code) = run_parsm(
        &[r#"level == "error" [[${level}] ${msg}]"#],
        r#"level=error msg="DB error""#,
    );
    assert_eq!(code, 0, "stderr: {stderr}");
    assert_eq!(stdout, "[error] DB error");
}

/// OWNER: `\[` and `\]` escape a lone bracket character inside a bracketed
/// template, so it need not balance with a matching `[` or `]`.
#[test]
fn proof_bracket_escape_is_a_literal_bracket() {
    let (stdout, stderr, code) = run_parsm(&[r"[a \[ b]"], "{}");
    assert_eq!(code, 0, "stderr: {stderr}");
    assert_eq!(stdout, "a [ b");
}

/// OWNER: an unescaped, unbalanced `[` inside a bracketed template is a
/// parse error naming the `\[` escape as the fix.
#[test]
fn proof_unbalanced_bracket_is_a_parse_error() {
    let (_, stderr, code) = run_parsm(&["[a [ b]"], "{}");
    assert_eq!(code, 1);
    assert!(
        stderr.contains(r"\["),
        "stderr should name the '\\[' escape: {stderr}"
    );
}

/// OWNER: `\]` escapes a literal `]` too, symmetric with `\[`.
#[test]
fn proof_close_bracket_escape_is_a_literal_bracket() {
    let (stdout, stderr, code) = run_parsm(&[r"[a \] b]"], "{}");
    assert_eq!(code, 0, "stderr: {stderr}");
    assert_eq!(stdout, "a ] b");
}

/// OWNER: an unescaped, unbalanced `]` inside a bracketed template is a
/// parse error naming the `\]` escape as the fix.
#[test]
fn proof_unbalanced_close_bracket_is_a_parse_error() {
    let (_, stderr, code) = run_parsm(&["[a ] b]"], "{}");
    assert_eq!(code, 1);
    assert!(
        stderr.contains(r"\]"),
        "stderr should name the '\\]' escape: {stderr}"
    );
}

/// OWNER: the two-argument form's hint dispatch covers argument 2 too - an
/// unbalanced bracket in the template position names both the argument and
/// the `\[` escape.
#[test]
fn proof_two_argument_form_names_bracket_hint_in_template_position() {
    let (_, stderr, code) = run_parsm(&["a > 0", "[a [ b]"], r#"{"a":5}"#);
    assert_eq!(code, 1);
    assert!(
        stderr.contains("argument 2"),
        "stderr should name argument 2: {stderr}"
    );
    assert!(
        stderr.contains(r"\["),
        "stderr should name the '\\[' escape: {stderr}"
    );
}

/// #4: `~` is contains; a regex literal after it is a parse error naming `~=`.
#[test]
fn proof_tilde_contains_rejects_regex_literal() {
    let (_, stderr, code) = run_parsm(&["name ~ /A.*e/"], r#"{"name":"Alice"}"#);
    assert_eq!(code, 1);
    assert!(stderr.contains("~="), "stderr should name '~=': {stderr}");
}

/// #5: a string operator stringifies a number on the right, so `port *= 80`
/// matches `8080`. No template means the pipeline prints the record.
#[test]
fn proof_string_operator_stringifies_numeric_rhs() {
    let (stdout, stderr, code) = run_parsm(&["port *= 80"], r#"{"port":8080}"#);
    assert_eq!(code, 0, "stderr: {stderr}");
    assert_eq!(stdout, r#"{"port":8080}"#);
}

/// OWNER: `~=` stringifies a number literal on the right, like
/// `*=`/`^=`/`$=`, and compiles its text as a regex pattern at parse time.
#[test]
fn proof_regex_operator_stringifies_numeric_rhs() {
    let (stdout, stderr, code) = run_parsm(&["a ~= 5"], r#"{"a":"150"}"#);
    assert_eq!(code, 0, "stderr: {stderr}");
    assert_eq!(stdout, r#"{"a":"150"}"#);
}

/// OWNER: `~=` stringifies a boolean literal on the right the same way; a
/// non-matching pattern filters the record out, exit 0.
#[test]
fn proof_regex_operator_stringifies_boolean_rhs() {
    let (stdout, stderr, code) = run_parsm(&["a ~= true"], r#"{"a":"x"}"#);
    assert_eq!(code, 0, "stderr: {stderr}");
    assert_eq!(stdout, "");
}

/// #6: an invalid regex literal is a parse error naming the pattern, never
/// a silent substring-match fallback.
#[test]
fn proof_invalid_regex_literal_is_a_parse_error() {
    let (_, stderr, code) = run_parsm(&["s ~= /[b/"], r#"{"s":"a[b"}"#);
    assert_eq!(code, 1);
    assert!(
        stderr.contains("invalid regex pattern '[b'"),
        "stderr should name the specific pattern: {stderr}"
    );
}

/// F1: a quoted string under `~=` compiles as a regex at parse time, same
/// as a `/pattern/` literal - an invalid pattern is a parse error naming
/// it, not a silent non-match at every record.
#[test]
fn proof_invalid_regex_string_is_a_parse_error() {
    let (_, stderr, code) = run_parsm(&[r#"s ~= "[b""#], r#"{"s":"a[b"}"#);
    assert_eq!(code, 1);
    assert!(
        stderr.contains("invalid regex pattern '[b'"),
        "stderr should name the specific pattern: {stderr}"
    );
}

/// #7: the 'x' (extended) flag takes effect - insignificant whitespace in
/// the pattern.
#[test]
fn proof_regex_x_flag_takes_effect() {
    let (stdout, stderr, code) = run_parsm(&["s ~= /a b/x"], r#"{"s":"ab"}"#);
    assert_eq!(code, 0, "stderr: {stderr}");
    assert_eq!(stdout, r#"{"s":"ab"}"#);
}

/// F3: `\'` inside a single-quoted string unescapes to a literal `'`, the
/// same as `\"` and `\\`.
#[test]
fn proof_single_quote_escape_unescapes() {
    let (stdout, stderr, code) = run_parsm(&[r"'it\'s'"], r#"{"it's":"Y"}"#);
    assert_eq!(code, 0, "stderr: {stderr}");
    assert_eq!(stdout, "Y");
}

/// #8: the two-argument form's first argument must be a filter or empty; a
/// field selector there is an error naming the argument.
#[test]
fn proof_two_argument_form_rejects_selector_in_filter_position() {
    let (_, stderr, code) = run_parsm(&["name", "[${x}]"], r#"{"name":"x"}"#);
    assert_eq!(code, 1);
    assert!(
        stderr.contains("argument 1"),
        "stderr should name argument 1: {stderr}"
    );
}

/// #9: the two-argument form's second argument must be a template; a filter
/// there is an error naming the argument.
#[test]
fn proof_two_argument_form_rejects_filter_in_template_position() {
    let (_, stderr, code) = run_parsm(&["a > 0", "a > 3"], r#"{"a":5}"#);
    assert_eq!(code, 1);
    assert!(
        stderr.contains("argument 2"),
        "stderr should name argument 2: {stderr}"
    );
}

/// #10: a quoted selector is one literal key, dots included.
#[test]
fn proof_quoted_selector_is_one_literal_key() {
    let input = r#"{"user.name":"L","user":{"name":"N"}}"#;
    let (stdout, stderr, code) = run_parsm(&[r#""user.name""#], input);
    assert_eq!(code, 0, "stderr: {stderr}");
    assert_eq!(stdout, "L");
}

/// #11: a bare selector stays the nested-path form.
#[test]
fn proof_bare_selector_is_a_nested_path() {
    let input = r#"{"user.name":"L","user":{"name":"N"}}"#;
    let (stdout, stderr, code) = run_parsm(&["user.name"], input);
    assert_eq!(code, 0, "stderr: {stderr}");
    assert_eq!(stdout, "N");
}

/// #12: bare `!field` stays rejected, with an error naming the `!field?` fix.
#[test]
fn proof_bare_not_field_names_the_fix() {
    let (_, stderr, code) = run_parsm(&["!active"], r#"{"active":true}"#);
    assert_eq!(code, 1);
    assert!(
        stderr.contains("!active?"),
        "stderr should name '!active?': {stderr}"
    );
}

/// A6: `!a..b` has no valid field path after `!`, so the hint must not
/// suggest the equally-invalid `!a..b?` - only the generic parse error.
#[test]
fn proof_bare_not_invalid_field_gets_no_bad_hint() {
    let (_, stderr, code) = run_parsm(&["!a..b"], "{}");
    assert_eq!(code, 1);
    assert!(
        !stderr.contains("!a..b?"),
        "stderr should not offer a '!a..b?' style hint: {stderr}"
    );
}

/// #13: no default template injection - a filter-only expression prints the
/// record via the pipeline's own "no template" fallback.
#[test]
fn proof_filter_only_has_no_injected_template() {
    let (stdout, stderr, code) = run_parsm(&["a > 3"], r#"{"a":5}"#);
    assert_eq!(code, 0, "stderr: {stderr}");
    assert_eq!(stdout, r#"{"a":5}"#);
}

// Grammar regression cases: pest grammar corners that must keep parsing (or
// keep rejecting) a specific shape. `render` exercises the full pipeline
// in-process; `run_parsm` shells the real binary for escaping-heavy cases
// where an in-process Rust string literal would mask the bug under test.

const CANONICAL: &str = r#"{"name":"Alice","age":30,"active":true,"premium":true,"admin":false,"moderator":true,"banned":false,"email":"alice@example.com","a":5,"b":5,"c":5,"role":"admin","score":98.5,"threshold":95.0,"limit":25,"version":"2.0","target":"2.0","user":{"verified":true,"email":"alice@example.com","name":"Alice"},"count":0,"text":"say hi"}"#;

/// Render `expr` against `raw_input` exactly the way the CLI does: read it as
/// JSON through the record pipeline (which exposes the raw source text as the
/// `$0` pseudo-field that the default `${0}` template and "no template"
/// output resolve to), then filter and render. Returns stdout with the
/// trailing newline stripped, or `""` if the record was filtered out.
fn render(expr: &str, raw_input: &str) -> String {
    let parsed =
        parsm::parse_command(expr).unwrap_or_else(|e| panic!("'{expr}' should parse: {e}"));
    let mut buf = Vec::new();
    parsm::process(
        std::io::Cursor::new(raw_input),
        Some(parsm::Format::Json),
        parsm::Action::Evaluate(&parsed),
        &mut buf,
    )
    .expect("processing should succeed");
    String::from_utf8(buf)
        .expect("output must be utf8")
        .trim_end_matches('\n')
        .to_string()
}

/// Sanity-checks the `render` harness itself against an input that already
/// works correctly. If this ever fails, the tests below are not
/// trustworthy either.
#[test]
fn render_harness_sanity_check() {
    assert_eq!(render(r#"name == "Alice""#, CANONICAL), CANONICAL);
}

/// Bare `~` (contains) evaluates as a substring `Contains` check.
#[test]
fn bare_tilde_parses_as_contains() {
    assert_eq!(render(r#"email ~ "@example.com""#, CANONICAL), CANONICAL);
}

/// `!active` (bare NOT, no `?`, as the *entire* expression) is a correct,
/// intentional rejection per the DSL's conservative-parsing design
/// (`src/dsl/mod.rs` module docs).
#[test]
fn stays_rejected_bare_not_top_level() {
    assert!(parsm::parse_command("!active").is_err());
}

/// `!active && age >` (dangling operator) is a correct, intentional
/// rejection.
#[test]
fn stays_rejected_dangling_operator() {
    assert!(parsm::parse_command("!active && age >").is_err());
}

/// `${}` (empty braced template variable) is a correct, intentional
/// rejection.
#[test]
fn stays_rejected_empty_braced_variable() {
    assert!(parsm::parse_command("${}").is_err());
}

#[test]
fn field_vs_field_equal_renders_unchanged() {
    assert_eq!(render("a == b", CANONICAL), CANONICAL);
}

#[test]
fn field_vs_field_greater_than_renders_unchanged() {
    assert_eq!(render("age > limit", CANONICAL), CANONICAL);
}

#[test]
fn field_vs_field_greater_equal_renders_unchanged() {
    assert_eq!(render("score >= threshold", CANONICAL), CANONICAL);
}

#[test]
fn field_vs_field_string_equal_renders_unchanged() {
    assert_eq!(render("version == target", CANONICAL), CANONICAL);
}

#[test]
fn regex_case_insensitive_flag_is_accepted() {
    assert_eq!(render("email ~= /ALICE/i", CANONICAL), CANONICAL);
}

#[test]
fn regex_multiline_flag_is_accepted() {
    assert_eq!(render("text ~= /^say/m", CANONICAL), CANONICAL);
}

#[test]
fn tilde_contains_combined_with_and() {
    assert_eq!(
        render(r#"email ~ "@example.com" && age > 25"#, CANONICAL),
        CANONICAL
    );
}

#[test]
fn tilde_contains_combined_with_or() {
    assert_eq!(
        render(r#"email ~ "@x" || role == "admin""#, CANONICAL),
        CANONICAL
    );
}

#[test]
fn template_conditional_in_braces() {
    assert_eq!(render("${active?yes:no}", CANONICAL), "yes");
}

#[test]
fn template_conditional_in_brackets() {
    assert_eq!(render("[${active?yes:no}]", CANONICAL), "yes");
}

#[test]
fn comparison_then_bracket_template_conditional() {
    assert_eq!(render("age > 25 [${active?on:off}]", CANONICAL), "on");
}

/// Escaping-heavy: shell the real binary with raw-string argv so the
/// shell/DSL escaping under test isn't masked by an in-process Rust string
/// literal.
#[test]
fn escaped_quotes_in_string_literal() {
    let expr = r#"text == "say \"hi\"""#;
    let input = r#"{"text":"say \"hi\""}"#;
    let (stdout, stderr, code) = run_parsm(&[expr], input);
    assert_eq!(code, 0, "stderr: {stderr}");
    assert_eq!(stdout, input);
}

#[test]
fn tilde_contains_with_bracket_template() {
    assert_eq!(
        render(r#"email ~ "@example" [${name}]"#, CANONICAL),
        "Alice"
    );
}

/// A template nesting a `[...]` bracket span inside a `{...}` brace
/// template renders correctly regardless of input format (this pins the
/// JSON-input case; logfmt input is covered separately).
#[test]
fn nested_bracket_template_inside_brace_template() {
    assert_eq!(render("{[${name}] ${role}}", CANONICAL), "[Alice] admin");
}

/// `!field` (no `?`) as one term of a chain is rejected, the same as the
/// bare shape as the entire expression: bare identifiers are ambiguous, so
/// every position requires the explicit `?`.
#[test]
fn stays_rejected_bare_not_in_and_chain_cmp() {
    assert!(parsm::parse_command("!active && age > 25").is_err());
}

/// Same bare-`!field`-in-chain rejection, `&&` with a truthy term.
#[test]
fn stays_rejected_bare_not_in_and_chain_truthy() {
    assert!(parsm::parse_command("!active && premium?").is_err());
}

/// Same bare-`!field`-in-chain rejection, `||`.
#[test]
fn stays_rejected_bare_not_in_or_chain() {
    assert!(parsm::parse_command("!active || age > 25").is_err());
}

/// A bare, unescaped `{nested}` span inside a `{...}` template alongside a
/// `$var` is rejected: no grammar rule documents nested unescaped braces as
/// intended syntax.
#[test]
fn stays_rejected_nested_unescaped_brace_with_var() {
    assert!(parsm::parse_command("{a {nested} $name}").is_err());
}

/// Same rejection, a bare nested `{literal}` span with no `$var` at all.
#[test]
fn stays_rejected_nested_unescaped_brace_literal() {
    assert!(parsm::parse_command("{a{nested}b}").is_err());
}

#[test]
fn dollar_digits_are_a_literal_template() {
    assert_eq!(render("$20", CANONICAL), "$20");
}

/// A digit-leading `$1abc` bare variable is rejected: digit-leading names
/// collide with number-literal parsing.
#[test]
fn stays_rejected_digit_prefixed_variable() {
    assert!(parsm::parse_command("$1abc").is_err());
}

#[test]
fn braced_zero_renders_original_input() {
    assert_eq!(render("${0}", CANONICAL), CANONICAL);
}

/// A digit-leading bare field selector (`123abc`) is rejected: digit-leading
/// names collide with number-literal parsing.
#[test]
fn stays_rejected_digit_leading_field_selector() {
    assert!(parsm::parse_command("123abc").is_err());
}

/// A digit-leading field name on the LHS of a comparison (`1field == 5`) is
/// the same identifier gap, in comparison position.
#[test]
fn stays_rejected_digit_leading_field_comparison() {
    assert!(parsm::parse_command("1field == 5").is_err());
}

/// `age>25[name]extra]` is a malformed, unbalanced-bracket template
/// combined with a filter; pest rejects it directly (`expected EOI` at the
/// second `]`). Not a bug: it must keep rejecting.
#[test]
fn stays_rejected_malformed_bracket_template_with_filter() {
    assert!(parsm::parse_command("age>25[name]extra]").is_err());
}

/// Adversarial inputs covering unbalanced brackets/braces/parens, lone
/// operators, dangling `${`/`$`/`~`/`:` fragments, malformed template
/// conditionals, unterminated strings/regexes, deep nesting, empty string,
/// and mixed template+filter shapes. Each must return a `Result` (`Ok` or
/// `Err`) without panicking - this guards against a grammar edit turning a
/// graceful parse error into an `unwrap`/`unreachable!` panic. Hermetic: no
/// network, no files.
#[test]
fn fuzz_sweep_parse_command_never_panics() {
    let mut inputs: Vec<String> = [
        "",
        "{",
        "}",
        "[",
        "]",
        "{{{{{{{{{{",
        "}}}}}}}}}}",
        "((((((((((",
        ")))))))))",
        "&&",
        "||",
        "!",
        "!!",
        "!!!",
        "==",
        "~",
        "~=",
        "$",
        "${",
        "${}",
        "${0",
        "${0}",
        "${?}",
        "${a?b}",
        "${a?b:}",
        "${a?:b}",
        "${a?:}",
        ":",
        "a:b",
        "a ~ ",
        "a ==",
        "a == ",
        "a && ",
        "a &&",
        " && b",
        "!a &&",
        "!a && ",
        "{[}]",
        "[{]}",
        "{a [b}",
        "[a {b]",
        "\"",
        "\"unterminated",
        "'unterminated",
        "\"a\\\"",
        "/unterminated",
        "a ~= /",
        "a ~= //",
        "a ~= /x",
        "age > 25 [",
        "age > 25 {",
        "age > 25 [${}]",
        "age>25[name]extra]",
        "a == b == c",
        "a && b || c &&",
        "((a))",
        "!(",
        "!()",
        "()",
    ]
    .into_iter()
    .map(String::from)
    .collect();

    // Deep-but-bounded nesting: deep enough to be a meaningful adversarial
    // case, shallow enough not to blow the test harness's (smaller) worker
    // thread stack - recursive-descent stack depth on pathologically deep
    // input is a pre-existing architectural characteristic of the
    // recursive parser and is out of scope here.
    inputs.push("(".repeat(100));
    inputs.push(")".repeat(100));
    inputs.push("{".repeat(100));
    inputs.push("!".repeat(100));
    inputs.push("${".repeat(100));
    inputs.push("[".repeat(100));
    inputs.push(format!("{}active?{}", "(".repeat(40), ")".repeat(40)));
    // A depth-100 balanced bracket span, recursing bracket_literal_span
    // rather than tripping the unbalanced-bracket case above.
    inputs.push(format!("{}{}", "[".repeat(100), "]".repeat(100)));

    let first_panic = inputs.iter().find_map(|input| {
        let owned = input.clone();
        std::panic::catch_unwind(|| {
            let _ = parsm::parse_command(&owned);
        })
        .err()
        .map(|_| input.clone())
    });

    assert!(
        first_panic.is_none(),
        "parse_command panicked on input: {:?}",
        first_panic.unwrap_or_default()
    );
}

/// One row of the ambiguity sweep: a DSL argument list, its stdin input, and
/// the output it must produce.
enum Expect {
    Stdout(&'static str),
    ExitCode(i32),
}

struct SweepCase {
    args: &'static [&'static str],
    input: &'static str,
    expect: Expect,
}

/// Table-driven sweep of the grammar's disambiguating hazards, across every
/// top-level DSL form: field selector, filter, `[...]` template, `{...}`
/// template, filter followed by template, and the two-argument form. Each
/// row asserts the CLI's actual stdout or exit code, replacing
/// `tests/ambiguous_regression_test.rs`'s parse-only assertions.
const AMBIGUITY_SWEEP: &[SweepCase] = &[
    // Field selector: bare word vs dotted path vs quoted key.
    SweepCase {
        args: &["name"],
        input: r#"{"name":"Alice"}"#,
        expect: Expect::Stdout("Alice"),
    },
    SweepCase {
        args: &["user.name"],
        input: r#"{"user.name":"Literal","user":{"name":"Nested"}}"#,
        expect: Expect::Stdout("Nested"),
    },
    SweepCase {
        args: &[r#""user.name""#],
        input: r#"{"user.name":"Literal","user":{"name":"Nested"}}"#,
        expect: Expect::Stdout("Literal"),
    },
    SweepCase {
        args: &[r#""full name""#],
        input: r#"{"full name":"Alice Smith"}"#,
        expect: Expect::Stdout("Alice Smith"),
    },
    SweepCase {
        args: &["a.b.c"],
        input: r#"{"a":{"b":{"c":"deep"}}}"#,
        expect: Expect::Stdout("deep"),
    },
    // Filter: comparison operators with and without spaces.
    SweepCase {
        args: &["age > 25"],
        input: r#"{"age":30}"#,
        expect: Expect::Stdout(r#"{"age":30}"#),
    },
    SweepCase {
        args: &["age>25"],
        input: r#"{"age":30}"#,
        expect: Expect::Stdout(r#"{"age":30}"#),
    },
    SweepCase {
        args: &["age > 25"],
        input: r#"{"age":10}"#,
        expect: Expect::Stdout(""),
    },
    SweepCase {
        args: &[r#"name == "Alice""#],
        input: r#"{"name":"Alice"}"#,
        expect: Expect::Stdout(r#"{"name":"Alice"}"#),
    },
    SweepCase {
        args: &[r#"name!="Bob""#],
        input: r#"{"name":"Alice"}"#,
        expect: Expect::Stdout(r#"{"name":"Alice"}"#),
    },
    SweepCase {
        args: &["age>=25"],
        input: r#"{"age":30}"#,
        expect: Expect::Stdout(r#"{"age":30}"#),
    },
    SweepCase {
        args: &[r#"name^="Al""#],
        input: r#"{"name":"Alice"}"#,
        expect: Expect::Stdout(r#"{"name":"Alice"}"#),
    },
    // Filter: explicit truthy and negated truthy, standalone and chained.
    SweepCase {
        args: &["active?"],
        input: r#"{"active":true}"#,
        expect: Expect::Stdout(r#"{"active":true}"#),
    },
    SweepCase {
        args: &["!active?"],
        input: r#"{"active":false}"#,
        expect: Expect::Stdout(r#"{"active":false}"#),
    },
    SweepCase {
        args: &["active? && premium?"],
        input: r#"{"active":true,"premium":true}"#,
        expect: Expect::Stdout(r#"{"active":true,"premium":true}"#),
    },
    // `[...]` and `{...}` templates, field substitution.
    SweepCase {
        args: &["[${name}]"],
        input: r#"{"name":"Alice"}"#,
        expect: Expect::Stdout("Alice"),
    },
    SweepCase {
        args: &["{${name}}"],
        input: r#"{"name":"Alice"}"#,
        expect: Expect::Stdout("Alice"),
    },
    // Filter followed by a template, both template kinds.
    SweepCase {
        args: &["age > 25 {${name}}"],
        input: r#"{"name":"Alice","age":30}"#,
        expect: Expect::Stdout("Alice"),
    },
    SweepCase {
        args: &["age > 25 [${name}]"],
        input: r#"{"name":"Alice","age":30}"#,
        expect: Expect::Stdout("Alice"),
    },
    // The two-argument form, both template kinds.
    SweepCase {
        args: &["age > 25", "{${name}}"],
        input: r#"{"name":"Alice","age":30}"#,
        expect: Expect::Stdout("Alice"),
    },
    SweepCase {
        args: &["age > 25", "[${name}]"],
        input: r#"{"name":"Alice","age":30}"#,
        expect: Expect::Stdout("Alice"),
    },
    // `$` amounts vs `$name`/`${name}`/`${0}`.
    SweepCase {
        args: &["$20"],
        input: "{}",
        expect: Expect::Stdout("$20"),
    },
    SweepCase {
        args: &["$0"],
        input: "{}",
        expect: Expect::Stdout("$0"),
    },
    SweepCase {
        args: &["$name"],
        input: r#"{"name":"Alice"}"#,
        expect: Expect::Stdout("Alice"),
    },
    SweepCase {
        args: &["${0}"],
        input: r#"{"a":1}"#,
        expect: Expect::Stdout(r#"{"a":1}"#),
    },
    // Brackets and braces inside templates.
    SweepCase {
        args: &["{[${level}] ${msg}}"],
        input: r#"{"level":"error","msg":"timeout"}"#,
        expect: Expect::Stdout("[error] timeout"),
    },
    SweepCase {
        args: &[r"[a \[ b]"],
        input: "{}",
        expect: Expect::Stdout("a [ b"),
    },
    SweepCase {
        args: &["[{$name}]"],
        input: r#"{"name":"Alice"}"#,
        expect: Expect::Stdout("{Alice}"),
    },
    // `!field?` and the bare-field ambiguity the `?` resolves.
    SweepCase {
        args: &["name && age"],
        input: r#"{"name":"Alice","age":30}"#,
        expect: Expect::ExitCode(1),
    },
    SweepCase {
        args: &["user || admin"],
        input: r#"{"user":true,"admin":true}"#,
        expect: Expect::ExitCode(1),
    },
    SweepCase {
        args: &["name && age [${name}]"],
        input: r#"{"name":"Alice","age":30}"#,
        expect: Expect::ExitCode(1),
    },
    SweepCase {
        args: &["name? && age? [${name}]"],
        input: r#"{"name":"Alice","age":30}"#,
        expect: Expect::Stdout("Alice"),
    },
    // Bare interpolated text is rejected; bracketed interpolated text works.
    SweepCase {
        args: &["Hello ${name}"],
        input: r#"{"name":"Alice"}"#,
        expect: Expect::ExitCode(1),
    },
    SweepCase {
        args: &["[Hello ${name}]"],
        input: r#"{"name":"Alice"}"#,
        expect: Expect::Stdout("Hello Alice"),
    },
];

/// Runs every [`AMBIGUITY_SWEEP`] row against the release binary and checks
/// its stdout or exit code.
#[test]
fn ambiguity_sweep() {
    assert!(
        AMBIGUITY_SWEEP.len() >= 30,
        "sweep should cover at least 30 hazards"
    );
    for (index, case) in AMBIGUITY_SWEEP.iter().enumerate() {
        let mut cmd = command();
        cmd.args(case.args);
        let output = common::run(cmd, case.input);
        match case.expect {
            Expect::Stdout(expected) => {
                assert!(
                    output.status.success(),
                    "row {index} {:?}: expected success, stderr={}",
                    case.args,
                    String::from_utf8_lossy(&output.stderr)
                );
                assert_eq!(
                    common::stdout_of(&output).trim(),
                    expected,
                    "row {index} {:?}",
                    case.args
                );
            }
            Expect::ExitCode(code) => {
                assert_eq!(
                    output.status.code(),
                    Some(code),
                    "row {index} {:?}: stdout={}",
                    case.args,
                    common::stdout_of(&output)
                );
            }
        }
    }
}
