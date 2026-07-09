use std::path::PathBuf;

use jolt_diagnostics::DiagnosticStage;
use jolt_java_fmt::{FormatOptions, FormatSinkResult, format_source_to_sink};
use jolt_java_syntax::parse_compilation_unit;
use jolt_test_support::{
    ImportedFormatterSummary, StringSink, collect_java_files, read_to_string, workspace_root,
};

#[test]
fn imported_fixture_inputs_format_idempotently_and_parse() {
    let google_summary = assert_corpus("google-java-format", 209);
    let palantir_summary = assert_corpus("palantir-java-format", 226);
    let prettier_summary = assert_corpus("prettier-java", 86);

    insta::assert_snapshot!(
        "google_java_format_formatter_summary",
        google_summary.render()
    );
    insta::assert_snapshot!(
        "palantir_java_format_formatter_summary",
        palantir_summary.render()
    );
    insta::assert_snapshot!("prettier_java_formatter_summary", prettier_summary.render());
}

fn assert_corpus(suite: &str, expected_files: usize) -> ImportedFormatterSummary {
    let root = fixture_root(suite);
    let files = collect_java_files(&root);

    assert_eq!(
        files.len(),
        expected_files,
        "expected the pinned {suite} Java input fixture corpus"
    );

    let mut summary = ImportedFormatterSummary::new(suite, files.len());
    let options = FormatOptions::default();

    for path in files {
        let source = read_to_string(&path);
        let parse = parse_compilation_unit(&source);
        let syntax = parse
            .syntax()
            .unwrap_or_else(|| panic!("parser aborted in {}", path.display()));
        if syntax.source_text() != source {
            summary.note_reconstruction_changed();
        }
        summary.record_diagnostics(parse.diagnostics());

        if !parse.diagnostics().is_empty() {
            summary.note_syntax_blocked();
            continue;
        }

        let formatted = match format_source(&source, options) {
            Ok(formatted) => formatted,
            Err(diagnostics) => {
                assert!(
                    diagnostics
                        .iter()
                        .all(|diagnostic| diagnostic.stage == DiagnosticStage::Formatter),
                    "non-formatter diagnostic(s) after clean parse in {}: {diagnostics:#?}",
                    path.display()
                );
                summary.record_diagnostics(&diagnostics);
                summary.note_formatter_blocked();
                continue;
            }
        };
        summary.note_formatted();

        let formatted_parse = parse_compilation_unit(&formatted);
        assert!(
            formatted_parse.diagnostics().is_empty(),
            "formatted output did not parse cleanly for {}: {:#?}\n{}",
            path.display(),
            formatted_parse.diagnostics(),
            formatted
        );
        assert!(
            formatted_parse.syntax().is_some(),
            "formatted output produced no syntax tree for {}",
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
        FormatSinkResult::Complete | FormatSinkResult::Halted => Ok(sink.into_string()),
        FormatSinkResult::Blocked { diagnostics } => Err(diagnostics),
    }
}

fn fixture_root(suite: &str) -> PathBuf {
    workspace_root(env!("CARGO_MANIFEST_DIR"))
        .join("tools/import/.imports")
        .join(suite)
        .join("input")
}
