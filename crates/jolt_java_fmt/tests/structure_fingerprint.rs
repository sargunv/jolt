//! Pins what the corpus structure guard tolerates and what it must still reject.
//!
//! The corpus only ever compares a fixture against real formatter output, so it can
//! show that today's output is accepted but never that an unauthorized tree edit would
//! be caught. These cases exercise the fingerprint contract directly: authorized
//! normalizations must compare equal, and edits outside that vocabulary must not.

use jolt_java_syntax::{JavaSyntaxView, parse_compilation_unit};
use jolt_test_support::structure_fingerprint;

mod common;

fn fingerprint(source: &str) -> String {
    let parse = parse_compilation_unit(source);
    assert!(
        parse.diagnostics().is_empty(),
        "fingerprint input must parse cleanly: {source}\n{:#?}",
        parse.diagnostics()
    );
    let root = parse
        .syntax()
        .and_then(|unit| unit.syntax_node())
        .expect("compilation unit");
    structure_fingerprint(root, &common::STRUCTURE_POLICY)
}

fn in_method(body: &str) -> String {
    format!("class T {{ void f() {{ {body} }} }}")
}

#[test]
fn promoting_a_bare_control_body_to_a_block_is_tolerated() {
    for (bare, braced) in [
        ("if (c) g();", "if (c) { g(); }"),
        ("if (c) g(); else h();", "if (c) { g(); } else { h(); }"),
        ("while (c) g();", "while (c) { g(); }"),
        ("for (;;) g();", "for (;;) { g(); }"),
        ("for (int v : a) g();", "for (int v : a) { g(); }"),
        ("do g(); while (c);", "do { g(); } while (c);"),
    ] {
        assert_eq!(
            fingerprint(&in_method(bare)),
            fingerprint(&in_method(braced)),
            "brace promotion should be transparent for {bare:?}"
        );
    }
}

#[test]
fn flattening_a_represented_block_is_rejected() {
    // Not a control body, so these braces are the programmer's. Dropping them changes
    // what the inner declaration scopes over and makes the method invalid.
    assert_ne!(
        fingerprint(&in_method("{ int x = 1; } int x = 2;")),
        fingerprint(&in_method("int x = 1; int x = 2;")),
        "a represented block must stay visible in the fingerprint"
    );
}

#[test]
fn canonicalizing_modifier_keyword_order_is_tolerated() {
    // The formatter sorts keywords and may move them across an annotation.
    assert_eq!(
        fingerprint("class T { @A static @B public final void n() {} }"),
        fingerprint("class T { @A public static final @B void n() {} }"),
    );
}

#[test]
fn moving_an_annotation_across_a_node_modifier_is_tolerated() {
    // `non-sealed` is a child node, not a keyword token, but the formatter still
    // sorts an annotation ahead of it.
    assert_eq!(
        fingerprint("non-sealed @A class T {}"),
        fingerprint("@A non-sealed class T {}"),
    );
}

#[test]
fn swapping_annotations_across_a_node_modifier_is_rejected() {
    // The partition that tolerates crossing `non-sealed` is stable: declaration
    // order among annotations still reaches the fingerprint.
    assert_ne!(
        fingerprint("@A non-sealed @B class T {}"),
        fingerprint("@B non-sealed @A class T {}"),
    );
}

#[test]
fn reordering_repeated_annotations_is_rejected() {
    // Reflection exposes repeatable annotations in declaration order.
    assert_ne!(
        fingerprint(r#"class T { @Tag("first") @Tag("second") void n() {} }"#),
        fingerprint(r#"class T { @Tag("second") @Tag("first") void n() {} }"#),
    );
}

#[test]
fn dropping_a_modifier_is_rejected() {
    assert_ne!(
        fingerprint("class T { public static void n() {} }"),
        fingerprint("class T { public void n() {} }"),
    );
}

#[test]
fn sorting_imports_is_tolerated_but_dropping_one_is_not() {
    assert_eq!(
        fingerprint("import b.B; import a.A; class T {}"),
        fingerprint("import a.A; import b.B; class T {}"),
    );
    assert_ne!(
        fingerprint("import a.A; import b.B; class T {}"),
        fingerprint("import a.A; class T {}"),
    );
    // Imports sort among their own positions, so a class must not drift past one.
    assert_ne!(
        fingerprint("import a.A; class T {} class U {}"),
        fingerprint("import a.A; class U {} class T {}"),
    );
}

#[test]
fn redundant_parentheses_are_tolerated_but_precedence_changes_are_not() {
    assert_eq!(
        fingerprint(&in_method("int p = ((a + b));")),
        fingerprint(&in_method("int p = a + b;")),
    );
    assert_ne!(
        fingerprint(&in_method("int p = (a + b) * c;")),
        fingerprint(&in_method("int p = a + b * c;")),
        "removing a paren that carries precedence reshapes the operator spine"
    );
}

#[test]
fn dropping_a_redundant_semicolon_is_tolerated_but_losing_a_statement_is_not() {
    assert_eq!(
        fingerprint(&in_method("g();;")),
        fingerprint(&in_method("g();")),
    );
    assert_ne!(
        fingerprint(&in_method("g(); h();")),
        fingerprint(&in_method("g();")),
    );
}

#[test]
fn relocating_method_reference_type_arguments_is_rejected() {
    // The defect this guard first caught: receiver type arguments moved past `::`.
    assert_ne!(
        fingerprint(&in_method("var r = ArrayList<String>::new;")),
        fingerprint(&in_method("var r = ArrayList::<String>new;")),
    );
}

#[test]
fn reordering_parameter_modifiers_is_rejected() {
    // Unlike a declaration `ModifierList`, a `ParameterModifierList` admits only `final`
    // plus annotations, so there is no keyword order to canonicalize and the formatter
    // preserves the source spelling. Both orders appear in the corpus.
    assert_ne!(
        fingerprint("class T { void n(final @A int x) {} }"),
        fingerprint("class T { void n(@A final int x) {} }"),
    );
}
