use jolt_kotlin_syntax::{KotlinSyntaxView, parse_kotlin_file};
use jolt_syntax::SyntaxSlot;
use jolt_test_support::{
    SnapshotBuilder, assert_bidirectional_diagnostic_ownership, collect_kotlin_files,
    fixture_manifest, fixture_snapshot_name, kotlin_fixture_root, read_to_string,
    render_diagnostics,
};

#[test]
fn kotlin_corpus_syntax_snapshots() {
    let root = kotlin_fixture_root(env!("CARGO_MANIFEST_DIR"));
    let paths = collect_kotlin_files(&root);
    insta::assert_snapshot!("fixture_manifest", fixture_manifest(&root, &paths));

    for path in paths {
        let source = read_to_string(&path);
        let parse = parse_kotlin_file(&source);
        let syntax = parse
            .syntax()
            .unwrap_or_else(|| panic!("parser aborted in {}", path.display()));

        assert_eq!(
            syntax.source_text(),
            source,
            "parser reconstruction changed source in {}",
            path.display()
        );
        assert_recovery_cores(
            syntax.syntax_node().expect("physical Kotlin root"),
            path.display(),
        );
        if path.ends_with("syntax/recovery/phase-16-program.kt")
            || path.ends_with("syntax/recovery/phase-17-types-and-parameters.kt")
            || path.ends_with("syntax/recovery/phase-18-declarations.kt")
            || path.ends_with("syntax/recovery/phase-19-expressions.kt")
            || path.ends_with("syntax/recovery/phase-20-statements-and-control-flow.kt")
        {
            assert_bidirectional_diagnostic_ownership(
                syntax.syntax_node().expect("physical Kotlin root"),
                parse.diagnostics(),
                parse.structural_diagnostic_owners(),
                |_| true,
                path.display(),
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
    root: jolt_kotlin_syntax::KotlinSyntaxNode<'_>,
    context: impl std::fmt::Display,
) {
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        if node.is_directly_malformed() {
            assert!(
                node.malformed_verbatim_core().is_some(),
                "directly malformed Kotlin node lacked verbatim recovery in {context}: {:?}",
                node.kind(),
            );
        }
        for slot in 0..node.slot_count() {
            if matches!(node.slot_at(slot), Some(SyntaxSlot::Empty)) {
                assert!(
                    node.missing_verbatim_core(slot).is_some(),
                    "missing Kotlin slot lacked zero-width recovery in {context}: {:?}[{slot}]",
                    node.kind(),
                );
            }
        }
        stack.extend(node.children());
    }
}
