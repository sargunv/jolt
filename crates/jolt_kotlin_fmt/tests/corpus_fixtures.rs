use std::path::PathBuf;

use jolt_kotlin_fmt::{FormatOptions, FormatSinkResult, format_source_to_sink};
use jolt_kotlin_syntax::parse_kotlin_file;
use jolt_test_support::{
    DeferredReason, ImportedFormatterSummary, ObservedDeferredPath, StringSink,
    assert_deferred_import_manifest, collect_kotlin_files, read_to_string, workspace_root,
};

#[test]
fn imported_fixture_inputs_format_idempotently_and_parse() {
    let mut deferred = Vec::new();
    let ktfmt_summary = assert_corpus("ktfmt", &mut deferred);
    let maplibre_summary = assert_corpus("maplibre-compose", &mut deferred);

    let workspace = workspace_root(env!("CARGO_MANIFEST_DIR"));
    assert_deferred_import_manifest(
        &workspace,
        &workspace.join("tools/import/.imports"),
        &["ktfmt/source", "maplibre-compose/source"],
        &[],
        &deferred,
    );

    insta::assert_snapshot!("ktfmt_formatter_summary", ktfmt_summary.render());
    insta::assert_snapshot!(
        "maplibre_compose_formatter_summary",
        maplibre_summary.render()
    );
}

fn assert_corpus(
    suite: &str,
    deferred: &mut Vec<ObservedDeferredPath>,
) -> ImportedFormatterSummary {
    let root = fixture_root(suite);
    let files = collect_kotlin_files(&root);
    let suite_name = format!("{suite}/source");
    let mut summary = ImportedFormatterSummary::new(suite, files.len());
    let options = FormatOptions::default();

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
            summary.note_syntax_blocked();
            reasons.push(DeferredReason::ParserDiagnostics);
        }
        if !reasons.is_empty() {
            deferred.push(ObservedDeferredPath::new(
                &suite_name,
                &root,
                &path,
                reasons,
            ));
            continue;
        }
        let _syntax = syntax.expect("active imported path has syntax");

        let formatted = match format_source(&source, options) {
            Ok(formatted) => formatted,
            Err(diagnostics) => panic!(
                "formatter refused clean imported input {}: {diagnostics:#?}",
                path.display()
            ),
        };
        summary.note_formatted();

        let formatted_parse = parse_kotlin_file(&formatted);
        assert!(
            formatted_parse.diagnostics().is_empty(),
            "formatted output did not parse cleanly for {}: {:#?}\n{}",
            path.display(),
            formatted_parse.diagnostics(),
            formatted
        );
        let formatted_syntax = formatted_parse.syntax().unwrap_or_else(|| {
            panic!(
                "formatted output produced no syntax tree for {}",
                path.display()
            )
        });
        assert_eq!(
            formatted_syntax.source_text(),
            formatted,
            "formatted output did not reconstruct exactly for {}",
            path.display()
        );

        let formatted_again = format_source(&formatted, options).unwrap_or_else(|diagnostics| {
            panic!(
                "formatted output was not accepted by formatter for {}: {diagnostics:#?}",
                path.display()
            )
        });
        assert_eq!(
            formatted_again,
            formatted,
            "formatted output was not idempotent for {}",
            path.display()
        );

        let repeated = format_source(&source, options).unwrap_or_else(|diagnostics| {
            panic!(
                "repeated formatting produced diagnostic(s) for {}: {diagnostics:#?}",
                path.display()
            )
        });
        assert_eq!(
            repeated,
            formatted,
            "formatting was not deterministic for {}",
            path.display()
        );
    }

    summary
}

fn format_source(
    source: &str,
    options: FormatOptions,
) -> Result<String, Vec<jolt_diagnostics::Diagnostic>> {
    let mut sink = StringSink::default();
    match format_source_to_sink(source, &options, &mut sink) {
        FormatSinkResult::Complete => Ok(sink.into_string()),
        FormatSinkResult::Halted => panic!("formatter unexpectedly halted with StringSink"),
        FormatSinkResult::Blocked { diagnostics } => Err(diagnostics),
    }
}

fn fixture_root(suite: &str) -> PathBuf {
    workspace_root(env!("CARGO_MANIFEST_DIR"))
        .join("tools/import/.imports")
        .join(suite)
        .join("source")
}
