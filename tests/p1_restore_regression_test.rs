//! P1 fallback-redaction regression tests.
//!
//! `src/dsl/fallback.rs` (an 803-line hand-rolled string-splitter) used to catch
//! every input pest's grammar rejected and parse it with different, sometimes
//! wrong, semantics. P1.2 deleted it and made a pest parse error a real error
//! that propagates out of `parse_command`, instead of being silently rescued.
//!
//! That redaction leaves two kinds of inputs pinned red here, on purpose:
//!
//! - **Set A** - inputs that were *already* semantically wrong even while the
//!   fallback handled them (e.g. field-vs-field comparisons, dropped regex
//!   flags, unimplemented template conditionals, string-escape bugs).
//! - **Set B** - inputs that only ever worked *because* the fallback rescued
//!   them from a pest parse failure; deleting it made them start erroring.
//!
//! Each is asserted against its *correct* target output and pinned
//! `#[ignore]`, so `cargo test` stays green while `cargo test -- --ignored`
//! shows them red until P1.3 restores the behavior in the pest grammar (or
//! wherever the named fix belongs) and un-ignores the test.
//!
//! Three additional inputs are *correct* rejections today (the DSL's
//! conservative-parsing design intentionally errors on them) and must keep
//! rejecting forever - those get permanent, non-ignored guards instead of a
//! restore-in-P1.3 pin.

use serde_json::Value;
use std::io::Write;
use std::process::{Command, Stdio};

const CANONICAL: &str = r#"{"name":"Alice","age":30,"active":true,"premium":true,"admin":false,"moderator":true,"banned":false,"email":"alice@example.com","a":5,"b":5,"c":5,"role":"admin","score":98.5,"threshold":95.0,"limit":25,"version":"2.0","target":"2.0","user":{"verified":true,"email":"alice@example.com","name":"Alice"},"count":0,"text":"say hi"}"#;

/// Render `expr` against `raw_input` exactly the way the CLI does for a single
/// record: parse the JSON, inject the raw source text under the `$0`
/// pseudo-field (this is what the default `${0}` template and "no template"
/// fallback resolve to - see `process_single_value` in `src/lib.rs`), then run
/// the real filter+template pipeline. Returns stdout with the trailing
/// newline stripped, or `""` if the record was filtered out.
fn render(expr: &str, raw_input: &str) -> String {
    let mut data: Value = serde_json::from_str(raw_input).expect("fixture must be valid JSON");
    if let Value::Object(ref mut obj) = data {
        obj.insert("$0".to_string(), Value::String(raw_input.to_string()));
    }
    let parsed =
        parsm::parse_command(expr).unwrap_or_else(|e| panic!("'{expr}' should parse: {e}"));
    let mut buf = Vec::new();
    parsm::process_single_value(&data, &parsed, &mut buf).expect("processing should succeed");
    String::from_utf8(buf)
        .expect("output must be utf8")
        .trim_end_matches('\n')
        .to_string()
}

/// Shell the real release binary with raw-string argv. Reserved for
/// escaping-heavy cases (quoted strings containing escaped quotes, regex
/// literals with backslashes) where an in-process DSL-string literal would
/// mask exactly the escaping bug under test.
fn run_binary(expr: &str, stdin_data: &str) -> (String, bool) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_parsm"))
        .arg(expr)
        .env("RUST_LOG", "parsm=error")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn parsm");
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(stdin_data.as_bytes())
        .expect("write stdin");
    let output = child.wait_with_output().expect("wait for parsm");
    (
        String::from_utf8_lossy(&output.stdout)
            .trim_end_matches('\n')
            .to_string(),
        output.status.success(),
    )
}

// ---------------------------------------------------------------------------
// Permanent guards (non-ignored) - these must hold both before and after
// P1.3, forever.
// ---------------------------------------------------------------------------

/// Sanity-checks the `render` harness itself against an input that already
/// works correctly today, independent of the fallback redaction. If this
/// ever fails, the pinned tests below are not trustworthy either.
#[test]
fn render_harness_sanity_check() {
    assert_eq!(render(r#"name == "Alice""#, CANONICAL), CANONICAL);
}

/// Bare `~` (contains) was missing from the pest grammar (only `~=` was),
/// so with the fallback gone it used to be a hard parse error rather than a
/// silent rescue. P1.3 restores it as a real grammar operator (Set B:
/// `foss-contains-tilde`) instead of resurrecting `fallback.rs`, so this now
/// asserts the corrected, permanent behavior: parses, and evaluates as
/// substring `Contains`.
#[test]
fn bare_tilde_parses_as_contains() {
    assert_eq!(
        render(r#"email ~ "@example.com""#, CANONICAL),
        CANONICAL
    );
}

/// `!active` (bare NOT, no `?`, as the *entire* expression) is a correct,
/// intentional rejection per the DSL's conservative-parsing design
/// (`src/dsl/mod.rs` module docs) - not a Set A/B bug. It must keep
/// rejecting forever; `corpus.json`'s `foss-not-bare`.
#[test]
fn stays_rejected_bare_not_top_level() {
    assert!(parsm::parse_command("!active").is_err());
}

/// `!active && age >` (dangling operator) is a correct, intentional
/// rejection; `corpus.json`'s `cov-bool-catchall-reject`.
#[test]
fn stays_rejected_dangling_operator() {
    assert!(parsm::parse_command("!active && age >").is_err());
}

/// `${}` (empty braced template variable) is a correct, intentional
/// rejection; `corpus.json`'s `cov-tmpl-empty-braced-var`.
#[test]
fn stays_rejected_empty_braced_variable() {
    assert!(parsm::parse_command("${}").is_err());
}

// ---------------------------------------------------------------------------
// Set A - already semantically wrong even with the fallback in place.
// ---------------------------------------------------------------------------

#[test]
fn set_a_adv_field_vs_field() {
    assert_eq!(render("a == b", CANONICAL), CANONICAL);
}

#[test]
fn set_a_adv_fvf_gt() {
    assert_eq!(render("age > limit", CANONICAL), CANONICAL);
}

#[test]
fn set_a_adv_fvf_ge() {
    assert_eq!(render("score >= threshold", CANONICAL), CANONICAL);
}

#[test]
fn set_a_adv_fvf_str() {
    assert_eq!(render("version == target", CANONICAL), CANONICAL);
}

#[test]
fn set_a_gram_op_regex_flag() {
    assert_eq!(render("email ~= /ALICE/i", CANONICAL), CANONICAL);
}

#[test]
fn set_a_gram_regex_flag_m() {
    assert_eq!(render("text ~= /^say/m", CANONICAL), CANONICAL);
}

#[test]
fn set_a_foss_tilde_and() {
    assert_eq!(
        render(r#"email ~ "@example.com" && age > 25"#, CANONICAL),
        CANONICAL
    );
}

#[test]
fn set_a_foss_tilde_or() {
    assert_eq!(
        render(r#"email ~ "@x" || role == "admin""#, CANONICAL),
        CANONICAL
    );
}

#[test]
fn set_a_gram_tmpl_cond() {
    assert_eq!(render("${active?yes:no}", CANONICAL), "yes");
}

#[test]
fn set_a_gram_tmpl_cond_br() {
    assert_eq!(render("[${active?yes:no}]", CANONICAL), "yes");
}

#[test]
fn set_a_adv_cmp_then_tmpl_cond() {
    assert_eq!(render("age > 25 [${active?on:off}]", CANONICAL), "on");
}

#[test]
fn set_a_adv_str_escape() {
    // Escaping-heavy (Pattern D/E): shell the real binary with raw-string
    // argv so the shell/DSL escaping under test isn't masked by an
    // in-process Rust string literal.
    let expr = r#"text == "say \"hi\"""#;
    let input = r#"{"text":"say \"hi\""}"#;
    let (stdout, success) = run_binary(expr, input);
    assert!(success, "parsm should exit 0");
    assert_eq!(stdout, input);
}

// ---------------------------------------------------------------------------
// Set B - correct only because the fallback rescued them from a pest parse
// failure; derived by before/after diff on the release binary (see hand-back
// disposition summary for the full derivation).
// ---------------------------------------------------------------------------

#[test]
fn set_b_foss_contains_tilde() {
    assert_eq!(render(r#"email ~ "@example.com""#, CANONICAL), CANONICAL);
}

#[test]
fn set_b_foss_tilde_tmpl() {
    assert_eq!(
        render(r#"email ~ "@example" [${name}]"#, CANONICAL),
        "Alice"
    );
}

#[test]
#[ignore = "restore in P1.3: '!active && age > 25' (cov-bool-not-and-cmp) — grammar rule: bare '!field' (no '?') as one term of a chain isn't valid in not_expr/comparison_expr; this is the open design question in CORPUS.md section 1a and needs a deliberate call, not an accident"]
fn set_b_cov_bool_not_and_cmp() {
    assert_eq!(render("!active && age > 25", CANONICAL), "");
}

#[test]
#[ignore = "restore in P1.3: '!active && premium?' (cov-bool-not-and-truthy) — grammar rule, same bare-'!field'-in-chain design question"]
fn set_b_cov_bool_not_and_truthy() {
    assert_eq!(render("!active && premium?", CANONICAL), "");
}

#[test]
#[ignore = "restore in P1.3: '!active || age > 25' (cov-bool-not-or-cmp) — grammar rule, same bare-'!field'-in-chain design question, '||'"]
fn set_b_cov_bool_not_or_cmp() {
    assert_eq!(render("!active || age > 25", CANONICAL), CANONICAL);
}

#[test]
#[ignore = "restore in P1.3: '{a {nested} $name}' (cov-tmpl-nested-brace-var) — grammar rule: braced_template_content/interpolated_content doesn't accept a nested unescaped '{literal}' alongside a '$var'"]
fn set_b_cov_tmpl_nested_brace_var() {
    assert_eq!(render("{a {nested} $name}", CANONICAL), "a {nested} Alice");
}

#[test]
#[ignore = "restore in P1.3: '{a{nested}b}' (cov-tmpl-nested-brace-literal) — grammar rule, nested unescaped '{literal}' with no '$' fails EOI inside braced_template_content"]
fn set_b_cov_tmpl_nested_brace_literal() {
    assert_eq!(render("{a{nested}b}", CANONICAL), "a{nested}b");
}

#[test]
fn set_b_cov_tmpl_dollar_digits() {
    assert_eq!(render("$20", CANONICAL), "$20");
}

#[test]
#[ignore = "restore in P1.3: '$1abc' (cov-tmpl-digit-prefixed-var) — grammar rule: simple_variable requires non_numeric_field_path (alpha/'_' leading char), rejecting a digit-prefixed bare variable"]
fn set_b_cov_tmpl_digit_prefixed_var() {
    assert_eq!(render("$1abc", r#"{"1abc":"weird"}"#), "weird");
}

#[test]
fn set_b_cov_tmpl_dollar_zero() {
    assert_eq!(render("${0}", CANONICAL), CANONICAL);
}

#[test]
#[ignore = "restore in P1.3: '123abc' (cov-field-numeric) — grammar rule / FieldPath: digit-leading bare field selectors aren't a valid field_path (identifier requires alpha/'_' first, numeric_identifier requires all-digit)"]
fn set_b_cov_field_numeric() {
    assert_eq!(render("123abc", CANONICAL), "");
}

#[test]
#[ignore = "restore in P1.3: '1field == 5' (previously-unconfirmed branch: parse_simple_filter's non-'~' operator arms, fallback.rs pre-deletion) — grammar rule / FieldPath: digit-leading field name in a comparison; same identifier gap as cov-field-numeric but in comparison position. Verified reachable for all 9 non-'~=' operators (==, !=, <, <=, >, >=, ^=, $=, *=) via the same code path; this test covers '=='"]
fn set_b_digit_leading_field_comparison() {
    assert_eq!(render("1field == 5", r#"{"1field":5}"#), r#"{"1field":5}"#);
}

/// `age>25[name]extra]` (previously-unconfirmed branches: try_manual_parsing's
/// DSLParser::parse_filter_only fallback at fallback.rs:188-193, and its basic
/// brace/bracket literal fallback at fallback.rs:204-220) is a malformed,
/// unbalanced-bracket template combined with a filter, not in `corpus.json`.
/// Reclassified from a Set B restore target after review: pest already
/// rejects this directly post-deletion (`expected EOI` at the second `]`) -
/// there is no fallback-derived "correct" output to restore, only leftover
/// first/last-byte-slicing garbage from the deleted code. It must keep
/// rejecting; not a bug, not a P1.3 fix target.
#[test]
fn stays_rejected_malformed_bracket_template_with_filter() {
    assert!(parsm::parse_command("age>25[name]extra]").is_err());
}
