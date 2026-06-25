// Java SE 26 grammar and syntax specification:
// https://docs.oracle.com/javase/specs/jls/se26/html/jls-2.html
// https://docs.oracle.com/javase/specs/jls/se26/html/jls-19.html
//
// Java parser focused-test bar. Focused tests should cover:
//
// - every Java syntax grammar declaration, using small representative programs
//   rather than every possible production combination;
// - every known parser ambiguity, including contextual keywords, type-vs-
//   expression boundaries, lambda parameters, casts, patterns, switch labels,
//   and greater-than token splitting in type contexts;
// - error recovery shapes when a diagnostic or recovery boundary is part of
//   parser behavior the formatter depends on.
// - regression tests grounded in actual bugs we have written
//
// Focused tests should not try to enumerate the combinatorial product of the
// grammar. Each test should make one source-shape claim obvious.

use jolt_syntax::green_text;

use super::parse_compilation_unit;
use crate::JavaSyntaxKind;

#[test]
fn parser_shell_wraps_source_in_compilation_unit() {
    let parse = parse_compilation_unit("package a;\nclass A {}\n");

    assert_eq!(parse.syntax().kind(), JavaSyntaxKind::CompilationUnit);
    assert!(parse.diagnostics().is_empty());
    assert!(parse.lexer_diagnostics().is_empty());
}

#[test]
fn parser_shell_preserves_source_text() {
    let source = "class A {\n  // hello\n}\n";
    let parse = parse_compilation_unit(source);

    assert_eq!(green_text(parse.syntax().green()), source);
}
