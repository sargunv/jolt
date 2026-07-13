use std::path::PathBuf;

use jolt_java_syntax::parse_compilation_unit;
use jolt_test_support::{
    CorpusSummary, DeferredReason, ObservedDeferredPath, assert_deferred_import_manifest,
    collect_java_files, read_to_string, workspace_root,
};

#[test]
fn fixture_java_inputs_parse_without_loss() {
    let mut deferred = Vec::new();
    let google_summary = assert_corpus("google-java-format", &mut deferred);
    let palantir_summary = assert_corpus("palantir-java-format", &mut deferred);
    let prettier_summary = assert_corpus("prettier-java", &mut deferred);

    let workspace = workspace_root(env!("CARGO_MANIFEST_DIR"));
    assert_deferred_import_manifest(
        &workspace,
        &workspace.join("tools/import/.imports"),
        &[
            "google-java-format/input",
            "palantir-java-format/input",
            "prettier-java/input",
        ],
        &[
            DeferredReason::ParserDiagnostics,
            DeferredReason::NoSyntaxTree,
            DeferredReason::SyntaxReconstructionMismatch,
        ],
        &deferred,
    );

    insta::assert_snapshot!("google_java_format_parser_summary", google_summary.render());
    insta::assert_snapshot!(
        "palantir_java_format_parser_summary",
        palantir_summary.render()
    );
    insta::assert_snapshot!("prettier_java_parser_summary", prettier_summary.render());
}

fn assert_corpus(suite: &str, deferred: &mut Vec<ObservedDeferredPath>) -> CorpusSummary {
    let root = fixture_root(suite);
    let files = collect_java_files(&root);
    let suite_name = format!("{suite}/input");
    let mut summary = CorpusSummary::new(suite, files.len());
    for path in files {
        let source = read_to_string(&path);
        let parse = parse_compilation_unit(&source);
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
        .join("input")
}
