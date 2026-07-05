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

/// `!field` (no `?`) as one term of a chain was the "open design question" in
/// CORPUS.md section 1a: silently accepted as `NOT(FieldTruthy)` by the
/// deleted fallback (`try_boolean_with_truthy_fields`), while the same bare
/// shape as the *entire* expression was already a hard rejection. Resolved:
/// bare `!field` is rejected in every position, everywhere, so the
/// conservative-parsing invariant (bare identifiers are ambiguous; require
/// explicit `?`) is consistent - not restored as `NOT(FieldTruthy)`.
/// Reclassified from a Set B restore target to a stays-rejected guard.
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

/// Disposition rule (nested, undocumented fallback quirk): a bare, unescaped
/// `{nested}` span inside a `{...}` template alongside a `$var` was only ever
/// a P1.1-invented coverage input exercising `fallback.rs`'s naive
/// string-splitter - no test, doc, or `--examples` case documents nested
/// unescaped braces as intended syntax. Restoring it would commit new grammar
/// complexity (a brace-depth-aware content rule) to preserve a fallback
/// accident. Reclassified to a stays-rejected guard rather than restored.
#[test]
fn stays_rejected_nested_unescaped_brace_with_var() {
    assert!(parsm::parse_command("{a {nested} $name}").is_err());
}

/// Same disposition: a bare nested `{literal}` span with no `$var` at all.
#[test]
fn stays_rejected_nested_unescaped_brace_literal() {
    assert!(parsm::parse_command("{a{nested}b}").is_err());
}

#[test]
fn set_b_cov_tmpl_dollar_digits() {
    assert_eq!(render("$20", CANONICAL), "$20");
}

/// Disposition rule: a digit-leading `$1abc` bare variable was only ever a
/// P1.1-invented coverage input exercising the deleted fallback's manual
/// splitter - no test or doc documents digit-leading bare variables as
/// intended syntax, and digit-leading names collide with number-literal
/// parsing generally. Reclassified to a stays-rejected guard.
#[test]
fn stays_rejected_digit_prefixed_variable() {
    assert!(parsm::parse_command("$1abc").is_err());
}

#[test]
fn set_b_cov_tmpl_dollar_zero() {
    assert_eq!(render("${0}", CANONICAL), CANONICAL);
}

/// Disposition rule: a digit-leading bare field selector (`123abc`) was only
/// ever a P1.1-invented coverage input exercising the deleted fallback's
/// field-selector branch - no test or doc documents digit-leading field
/// selectors as intended syntax, and digit-leading names collide with
/// number-literal parsing generally. Reclassified to a stays-rejected guard.
#[test]
fn stays_rejected_digit_leading_field_selector() {
    assert!(parsm::parse_command("123abc").is_err());
}

/// Disposition rule: a digit-leading field name on the LHS of a comparison
/// (`1field == 5`) is the same identifier gap as `cov-field-numeric`, just in
/// comparison position - no test or doc documents it as intended syntax, and
/// digit-leading names collide with number-literal parsing generally.
/// Reclassified to a stays-rejected guard.
#[test]
fn stays_rejected_digit_leading_field_comparison() {
    assert!(parsm::parse_command("1field == 5").is_err());
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

// ---------------------------------------------------------------------------
// Fuzz sweep - the top-level ordered choice (and every grammar/translation
// change above) must only ever return Ok or a graceful Err, never panic.
// ---------------------------------------------------------------------------

/// Adversarial inputs covering unbalanced brackets/braces/parens, lone
/// operators, dangling `${`/`$`/`~`/`:` fragments, malformed template
/// conditionals, unterminated strings/regexes, deep nesting, empty string,
/// and mixed template+filter shapes. Each must return a `Result` (`Ok` or
/// `Err`) without panicking - this guards against a grammar edit (e.g. the
/// new `template_conditional` arm, the new top-level `template_expr`
/// alternatives) turning a graceful parse error into an
/// `unwrap`/`unreachable!` panic. Hermetic: no network, no files.
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
    // input (verified separately to overflow well beyond this) is a
    // pre-existing architectural characteristic of the recursive parser,
    // predating this arc's grammar edits, and is out of scope here.
    inputs.push("(".repeat(100));
    inputs.push(")".repeat(100));
    inputs.push("{".repeat(100));
    inputs.push("!".repeat(100));
    inputs.push("${".repeat(100));
    inputs.push(format!("{}active?{}", "(".repeat(40), ")".repeat(40)));

    // Silence the default panic-hook noise for this sweep - a panic here is
    // data (a failing input), not an unhandled test-process crash.
    let previous_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let first_panic = inputs.iter().find_map(|input| {
        let owned = input.clone();
        std::panic::catch_unwind(|| {
            let _ = parsm::parse_command(&owned);
        })
        .err()
        .map(|_| input.clone())
    });
    std::panic::set_hook(previous_hook);

    assert!(
        first_panic.is_none(),
        "parse_command panicked on input: {:?}",
        first_panic.unwrap_or_default()
    );
}
