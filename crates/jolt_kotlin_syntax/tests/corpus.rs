use std::fs;
use std::path::{Path, PathBuf};

use jolt_kotlin_syntax::parse_kotlin_file;
use jolt_test_support::{
    SnapshotBuilder, fixture_snapshot_name, read_to_string, render_diagnostics, workspace_root,
};

const EXPECTED_KOTLIN_FIXTURE_COUNT: usize = 193;

#[test]
fn kotlin_corpus_syntax_snapshots() {
    let root = kotlin_fixture_root(env!("CARGO_MANIFEST_DIR"));
    let paths = collect_kotlin_files(&root);
    assert_eq!(
        paths.len(),
        EXPECTED_KOTLIN_FIXTURE_COUNT,
        "expected {EXPECTED_KOTLIN_FIXTURE_COUNT} Kotlin fixtures under {}",
        root.display()
    );

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

        let snapshot = SnapshotBuilder::new()
            .section("input", &source)
            .section("syntax", format!("{parse:#?}"))
            .section("diagnostics", render_diagnostics(parse.diagnostics()))
            .finish();

        insta::assert_snapshot!(fixture_snapshot_name(&root, &path), snapshot);
    }
}

fn kotlin_fixture_root(manifest_dir: &str) -> PathBuf {
    workspace_root(manifest_dir).join("fixtures/kotlin")
}

fn collect_kotlin_files(root: &Path) -> Vec<PathBuf> {
    assert!(
        root.is_dir(),
        "required Kotlin fixture directory is missing: {}",
        root.display()
    );

    let mut files = Vec::new();
    collect_kotlin_files_into(root, &mut files);
    files.sort();
    assert!(
        !files.is_empty(),
        "expected at least one Kotlin fixture under {}",
        root.display()
    );
    files
}

fn collect_kotlin_files_into(root: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(root)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", root.display()))
    {
        let path = entry.expect("valid directory entry").path();
        if path.is_dir() {
            collect_kotlin_files_into(&path, files);
        } else if path.extension().is_some_and(|extension| extension == "kt") {
            files.push(path);
        }
    }
}
