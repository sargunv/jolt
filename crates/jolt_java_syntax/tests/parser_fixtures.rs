use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::PathBuf;

use jolt_java_syntax::parse_compilation_unit;
use jolt_test_support::{collect_java_files, read_to_string, workspace_root};

#[test]
fn fixture_java_inputs_parse_without_loss() {
    let google_summary = assert_corpus("google-java-format", 209);
    let palantir_summary = assert_corpus("palantir-java-format", 226);
    let prettier_summary = assert_corpus("prettier-java", 86);

    insta::assert_snapshot!("google_java_format_parser_summary", google_summary.render());
    insta::assert_snapshot!(
        "palantir_java_format_parser_summary",
        palantir_summary.render()
    );
    insta::assert_snapshot!("prettier_java_parser_summary", prettier_summary.render());
}

fn assert_corpus(suite: &str, expected_files: usize) -> ImportedParserSummary {
    let root = fixture_root(suite);
    let files = collect_java_files(&root);

    assert_eq!(
        files.len(),
        expected_files,
        "expected the pinned {suite} Java input fixture corpus"
    );

    let mut summary = ImportedParserSummary::new(suite, files.len());
    for path in files {
        let source = read_to_string(&path);
        let parse = parse_compilation_unit(&source);
        let syntax = parse
            .syntax()
            .unwrap_or_else(|| panic!("parser aborted in {}", path.display()));

        if syntax.source_text() != source {
            summary.reconstructed_changed += 1;
        }

        summary.record_diagnostics(parse.diagnostics());
    }

    summary
}

fn fixture_root(suite: &str) -> PathBuf {
    workspace_root(env!("CARGO_MANIFEST_DIR"))
        .join("tools/import/.imports")
        .join(suite)
        .join("input")
}

struct ImportedParserSummary {
    suite: String,
    files: usize,
    reconstructed_changed: usize,
    diagnostics: BTreeMap<String, usize>,
}

impl ImportedParserSummary {
    fn new(suite: &str, files: usize) -> Self {
        Self {
            suite: suite.to_owned(),
            files,
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
