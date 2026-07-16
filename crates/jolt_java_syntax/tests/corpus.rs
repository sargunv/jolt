use jolt_diagnostics::{DiagnosticCodeId, DiagnosticStage};
use jolt_java_syntax::{JavaSyntaxView, parse_compilation_unit};
use jolt_syntax::SyntaxSlot;
use jolt_test_support::{
    SnapshotBuilder, assert_bidirectional_diagnostic_ownership, collect_java_files,
    fixture_manifest, fixture_snapshot_name, java_fixture_root, read_to_string, render_diagnostics,
};

#[test]
fn java_corpus_syntax_snapshots() {
    let root = java_fixture_root(env!("CARGO_MANIFEST_DIR"));
    let paths = collect_java_files(&root);
    insta::assert_snapshot!("fixture_manifest", fixture_manifest(&root, &paths));

    for path in paths {
        let is_lexer_fixture = path
            .strip_prefix(&root)
            .is_ok_and(|relative| relative.starts_with("syntax/lexer"));
        let source = read_to_string(&path);
        let parse = parse_compilation_unit(&source);

        if let Some(syntax) = parse.syntax() {
            assert_recovery_cores(
                syntax.syntax_node().expect("represented compilation unit"),
                path.display(),
            );
            assert_bidirectional_diagnostic_ownership(
                syntax.syntax_node().expect("represented compilation unit"),
                parse.diagnostics(),
                parse.structural_diagnostic_owners(),
                java_diagnostic_requires_owner,
                path.display(),
            );
        }
        if !is_lexer_fixture {
            let syntax = parse
                .syntax()
                .unwrap_or_else(|| panic!("parser aborted in {}", path.display()));
            assert_eq!(
                syntax.source_text(),
                source,
                "parser reconstruction changed source in {}",
                path.display()
            );
        }

        let snapshot = SnapshotBuilder::new()
            .section("input", &source)
            .section("syntax", format!("{parse:#?}"))
            .section("diagnostics", render_diagnostics(parse.diagnostics()))
            .finish();

        insta::assert_snapshot!(fixture_snapshot_name(&root, &path), snapshot);
    }
}

fn assert_recovery_cores(
    root: jolt_java_syntax::JavaSyntaxNode<'_>,
    context: impl std::fmt::Display,
) {
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        if node.is_directly_malformed() {
            assert!(
                node.malformed_verbatim_core().is_some(),
                "directly malformed Java node lacked verbatim recovery in {context}: {:?}",
                node.kind(),
            );
        }
        for slot in 0..node.slot_count() {
            if matches!(node.slot_at(slot), Some(SyntaxSlot::Empty)) {
                assert!(
                    node.missing_verbatim_core(slot).is_some(),
                    "missing Java slot lacked zero-width recovery in {context}: {:?}[{slot}]",
                    node.kind(),
                );
            }
        }
        stack.extend(node.children());
    }
}

fn java_diagnostic_requires_owner(diagnostic: &jolt_diagnostics::Diagnostic) -> bool {
    diagnostic.stage == DiagnosticStage::Parser
        && diagnostic.code
            != DiagnosticCodeId::new("java.parse.unqualified_yield_method_invocation")
        && diagnostic.code != DiagnosticCodeId::new("java.parse.decimal_integer_boundary_literal")
}
