use jolt_java_syntax::parse_compilation_unit;
use jolt_test_support::{
    SnapshotBuilder, collect_java_files, fixture_manifest, fixture_snapshot_name,
    java_fixture_root, read_to_string, render_diagnostics,
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

        if let Some(syntax) = parse.syntax().filter(|_| !is_lexer_fixture) {
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
