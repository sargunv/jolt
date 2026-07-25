//! Pins what the corpus structure guard tolerates and what it must still reject.
//!
//! The corpus only ever compares a fixture against real formatter output, so it can
//! show that today's output is accepted but never that an unauthorized tree edit would
//! be caught. These cases exercise the fingerprint contract directly.

use jolt_kotlin_syntax::{KotlinSyntaxView, parse_kotlin_file};
use jolt_test_support::structure_fingerprint;

mod common;

fn fingerprint(source: &str) -> String {
    let parse = parse_kotlin_file(source);
    assert!(
        parse.diagnostics().is_empty(),
        "fingerprint input must parse cleanly: {source}\n{:#?}",
        parse.diagnostics()
    );
    let root = parse
        .syntax()
        .and_then(|file| file.syntax_node())
        .expect("kotlin file");
    structure_fingerprint(root, &common::STRUCTURE_POLICY)
}

#[test]
fn breaking_before_an_indexing_suffix_is_rejected() {
    // The defect this guard exists for: a newline before `[` ends the postfix chain,
    // so the subscript becomes a separate collection-literal statement.
    assert_ne!(
        fingerprint("fun f() {\n    x[\"k\"]\n}\n"),
        fingerprint("fun f() {\n    x\n    [\"k\"]\n}\n"),
    );
}

#[test]
fn breaking_before_a_navigation_operator_is_tolerated() {
    // `memberAccessOperator` is the one postfix suffix that admits a leading newline.
    assert_eq!(
        fingerprint("fun f() {\n    x.y.z\n}\n"),
        fingerprint("fun f() {\n    x\n        .y\n        .z\n}\n"),
    );
}

#[test]
fn breaking_before_a_trailing_lambda_or_reference_is_rejected() {
    assert_ne!(
        fingerprint("fun f() {\n    x.map { it }\n}\n"),
        fingerprint("fun f() {\n    x.map\n    { it }\n}\n"),
    );
    assert_ne!(
        fingerprint("fun f() {\n    x::y\n}\n"),
        fingerprint("fun f() {\n    x\n    ::y\n}\n"),
    );
}

#[test]
fn sorting_imports_is_tolerated_but_dropping_one_is_not() {
    assert_eq!(
        fingerprint("import b.B\nimport a.A\n"),
        fingerprint("import a.A\nimport b.B\n"),
    );
    assert_ne!(
        fingerprint("import a.A\nimport b.B\n"),
        fingerprint("import a.A\n"),
    );
}

#[test]
fn clarifying_parentheses_are_tolerated_but_precedence_changes_are_not() {
    // An infix function call binds looser than `+`, so these parentheses are redundant
    // and the formatter is free to add them.
    assert_eq!(
        fingerprint("fun f() {\n    val v = bits context mask + Bits.Enabled\n}\n"),
        fingerprint("fun f() {\n    val v = bits context (mask + Bits.Enabled)\n}\n"),
    );
    assert_ne!(
        fingerprint("fun f() {\n    val v = (a + b) * c\n}\n"),
        fingerprint("fun f() {\n    val v = a + b * c\n}\n"),
    );
}

#[test]
fn losing_a_statement_is_rejected() {
    assert_ne!(
        fingerprint("fun f() {\n    g()\n    h()\n}\n"),
        fingerprint("fun f() {\n    g()\n}\n"),
    );
}

#[test]
fn a_long_postfix_chain_does_not_exhaust_the_stack() {
    // Postfix chains are built by a parser loop, so their trees nest as deeply as the
    // source is long: `excessive_syntax_nesting` never fires and a recursive walk
    // overflows. Deliberately far past any depth the nesting guard bounds.
    let source = format!(
        "fun f() {{\n    val v = base{}\n}}\n",
        ".m()".repeat(20_000)
    );
    let parse = parse_kotlin_file(&source);
    assert!(parse.diagnostics().is_empty(), "{:#?}", parse.diagnostics());
    let root = parse
        .syntax()
        .and_then(|file| file.syntax_node())
        .expect("kotlin file");
    let rendered = structure_fingerprint(root, &common::STRUCTURE_POLICY);
    assert_eq!(rendered.matches("NavigationExpression").count(), 20_000);
}
