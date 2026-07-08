use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::PathBuf;

use jolt_diagnostics::DiagnosticStage;
use jolt_kotlin_fmt::{KotlinFormatOptions, KotlinFormatSinkResult, format_source_to_sink};
use jolt_kotlin_syntax::parse_kotlin_file;
use jolt_test_support::{StringSink, collect_kotlin_files, read_to_string, workspace_root};

#[test]
fn imported_fixture_inputs_format_idempotently_and_parse() {
    let ktfmt_summary = assert_corpus("ktfmt", 72);
    let maplibre_summary = assert_corpus("maplibre-compose", 484);

    insta::assert_snapshot!("ktfmt_formatter_summary", ktfmt_summary.render());
    insta::assert_snapshot!(
        "maplibre_compose_formatter_summary",
        maplibre_summary.render()
    );
}

fn assert_corpus(suite: &str, expected_files: usize) -> ImportedFormatterSummary {
    let root = fixture_root(suite);
    let files = collect_kotlin_files(&root);

    assert_eq!(
        files.len(),
        expected_files,
        "expected the pinned {suite} Kotlin source fixture corpus"
    );

    let mut summary = ImportedFormatterSummary::new(suite, files.len());
    let options = KotlinFormatOptions::default();

    for path in files {
        let source = read_to_string(&path);
        let parse = parse_kotlin_file(&source);
        let syntax = parse
            .syntax()
            .unwrap_or_else(|| panic!("parser aborted in {}", path.display()));
        if syntax.source_text() != source {
            summary.reconstructed_changed += 1;
        }
        summary.record_diagnostics(parse.diagnostics());

        if !parse.diagnostics().is_empty() {
            summary.syntax_blocked += 1;
            continue;
        }

        let formatted = match format_source(&source, &options) {
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
                summary.formatter_blocked += 1;
                continue;
            }
        };
        summary.formatted += 1;

        let formatted_parse = parse_kotlin_file(&formatted);
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

        let formatted_again = format_source(&formatted, &options).unwrap_or_else(|diagnostics| {
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

        let repeated = format_source(&source, &options).unwrap_or_else(|diagnostics| {
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
    options: &KotlinFormatOptions,
) -> Result<String, Vec<jolt_diagnostics::Diagnostic>> {
    let mut sink = StringSink::default();
    match format_source_to_sink(source, options, &mut sink) {
        KotlinFormatSinkResult::Complete | KotlinFormatSinkResult::Halted => Ok(sink.into_string()),
        KotlinFormatSinkResult::Blocked { diagnostics } => Err(diagnostics),
    }
}

fn fixture_root(suite: &str) -> PathBuf {
    workspace_root(env!("CARGO_MANIFEST_DIR"))
        .join("tools/import/.imports")
        .join(suite)
        .join("source")
}

struct ImportedFormatterSummary {
    suite: String,
    files: usize,
    formatted: usize,
    syntax_blocked: usize,
    formatter_blocked: usize,
    reconstructed_changed: usize,
    diagnostics: BTreeMap<String, usize>,
}

impl ImportedFormatterSummary {
    fn new(suite: &str, files: usize) -> Self {
        Self {
            suite: suite.to_owned(),
            files,
            formatted: 0,
            syntax_blocked: 0,
            formatter_blocked: 0,
            reconstructed_changed: 0,
            diagnostics: BTreeMap::new(),
        }
    }

    fn record_diagnostics(&mut self, diagnostics: &[jolt_diagnostics::Diagnostic]) {
        for diagnostic in diagnostics {
            let key = format!("{:?}:{}", diagnostic.stage, diagnostic.code.as_str());
            *self.diagnostics.entry(key).or_default() += 1;
        }
    }

    fn render(&self) -> String {
        let mut output = String::new();
        writeln!(&mut output, "suite: {}", self.suite).expect("write summary");
        writeln!(&mut output, "files: {}", self.files).expect("write summary");
        writeln!(&mut output, "formatted: {}", self.formatted).expect("write summary");
        writeln!(&mut output, "syntax blocked: {}", self.syntax_blocked).expect("write summary");
        writeln!(&mut output, "formatter blocked: {}", self.formatter_blocked)
            .expect("write summary");
        writeln!(
            &mut output,
            "reconstructed changed: {}",
            self.reconstructed_changed
        )
        .expect("write summary");
        output.push_str("\ndiagnostics:\n");
        if self.diagnostics.is_empty() {
            output.push_str("  <none>: 0\n");
        } else {
            for (kind, count) in &self.diagnostics {
                writeln!(&mut output, "  {kind}: {count}").expect("write summary");
            }
        }
        output
    }
}
