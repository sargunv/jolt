use std::path::PathBuf;

use jolt_kotlin_syntax::parse_kotlin_file;
use jolt_test_support::{CorpusSummary, collect_kotlin_files, read_to_string, workspace_root};

#[test]
fn fixture_kotlin_sources_parse_without_loss() {
    let ktfmt_summary = assert_corpus("ktfmt", 72);
    let maplibre_summary = assert_corpus("maplibre-compose", 485);

    insta::assert_snapshot!("ktfmt_parser_summary", ktfmt_summary.render());
    insta::assert_snapshot!("maplibre_compose_parser_summary", maplibre_summary.render());
}

fn assert_corpus(suite: &str, expected_files: usize) -> CorpusSummary {
    let root = fixture_root(suite);
    let files = collect_kotlin_files(&root);

    assert_eq!(
        files.len(),
        expected_files,
        "expected the pinned {suite} Kotlin source fixture corpus"
    );

    let mut summary = CorpusSummary::new(suite, files.len());
    for path in files {
        let source = read_to_string(&path);
        let parse = parse_kotlin_file(&source);
        let syntax = parse
            .syntax()
            .unwrap_or_else(|| panic!("parser aborted in {}", path.display()));

        if syntax.source_text() != source {
            summary.note_reconstruction_changed();
        }

        summary.record_diagnostics(parse.diagnostics());
    }

    summary
}

fn fixture_root(suite: &str) -> PathBuf {
    workspace_root(env!("CARGO_MANIFEST_DIR"))
        .join("tools/import/.imports")
        .join(suite)
        .join("source")
}
