use std::path::PathBuf;

use jolt_kotlin_syntax::parse_kotlin_file;
use jolt_test_support::{
    CorpusSummary, DeferredReason, ObservedDeferredPath, assert_deferred_import_manifest,
    collect_kotlin_files, read_to_string, workspace_root,
};

#[test]
fn fixture_kotlin_sources_parse_without_loss() {
    let mut deferred = Vec::new();
    let ktfmt_summary = assert_corpus("ktfmt", &mut deferred);
    let maplibre_summary = assert_corpus("maplibre-compose", &mut deferred);

    let workspace = workspace_root(env!("CARGO_MANIFEST_DIR"));
    assert_deferred_import_manifest(
        &workspace,
        &workspace.join("tools/import/.imports"),
        &["ktfmt/source", "maplibre-compose/source"],
        &[
            DeferredReason::ParserDiagnostics,
            DeferredReason::NoSyntaxTree,
            DeferredReason::SyntaxReconstructionMismatch,
        ],
        &deferred,
    );

    insta::assert_snapshot!("ktfmt_parser_summary", ktfmt_summary.render());
    insta::assert_snapshot!("maplibre_compose_parser_summary", maplibre_summary.render());
}

fn assert_corpus(suite: &str, deferred: &mut Vec<ObservedDeferredPath>) -> CorpusSummary {
    let root = fixture_root(suite);
    let files = collect_kotlin_files(&root);
    let suite_name = format!("{suite}/source");
    let mut summary = CorpusSummary::new(suite, files.len());
    for path in files {
        let source = read_to_string(&path);
        let parse = parse_kotlin_file(&source);
        let syntax = parse.syntax();
        let mut reasons = Vec::new();

        if syntax.is_none() {
            reasons.push(DeferredReason::NoSyntaxTree);
        } else if syntax.is_some_and(|syntax| syntax.source_text() != source) {
            summary.note_reconstruction_changed();
            reasons.push(DeferredReason::SyntaxReconstructionMismatch);
        }
        summary.record_diagnostics(parse.diagnostics());
        if !parse.diagnostics().is_empty() {
            reasons.push(DeferredReason::ParserDiagnostics);
        }
        if !reasons.is_empty() {
            deferred.push(ObservedDeferredPath::new(
                &suite_name,
                &root,
                &path,
                reasons,
            ));
        }
    }

    summary
}

fn fixture_root(suite: &str) -> PathBuf {
    workspace_root(env!("CARGO_MANIFEST_DIR"))
        .join("tools/import/.imports")
        .join(suite)
        .join("source")
}
