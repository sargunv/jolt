use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use jolt_diagnostics::DiagnosticStage;
use jolt_java_syntax::{JavaParse, parse_compilation_unit};

#[test]
fn fixture_java_inputs_parse_without_loss() {
    let google_summary = assert_corpus("google-java-format", 209);
    let palantir_summary = assert_corpus("palantir-java-format", 226);

    insta::assert_snapshot!("google_java_format_parser_summary", google_summary.render());
    insta::assert_snapshot!(
        "palantir_java_format_parser_summary",
        palantir_summary.render()
    );
}

fn assert_corpus(suite: &str, expected_files: usize) -> CorpusSummary {
    let root = fixture_root(suite);
    let mut files = Vec::new();
    collect_java_files(&root, &mut files);

    files.sort();
    assert_eq!(
        files.len(),
        expected_files,
        "expected the pinned {suite} Java input fixture corpus"
    );

    let mut summary = CorpusSummary::new(suite, files.len());
    for path in files {
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
        let parse = parse_compilation_unit(&source);
        let syntax = parse
            .syntax()
            .unwrap_or_else(|| panic!("parser aborted in {}", path.display()));

        assert_eq!(
            syntax.source_text(),
            source,
            "parser reconstruction changed source in {}",
            path.display()
        );

        summary.record(&parse);

        if allows_syntax_diagnostics(&path) {
            assert!(
                !parse.diagnostics().is_empty(),
                "allowlisted syntax diagnostic fixture parsed cleanly and should be removed from the allowlist: {}",
                path.display()
            );
        } else {
            assert!(
                parse.diagnostics().is_empty(),
                "syntax diagnostic(s) in {}: {:#?}",
                path.display(),
                parse.diagnostics()
            );
        }
    }

    summary
}

fn allows_syntax_diagnostics(path: &Path) -> bool {
    // Intentionally invalid upstream Java: these fixtures place explicit
    // constructor invocations outside their valid constructor-body position.
    path.file_name().is_some_and(|file_name| {
        matches!(
            file_name.to_str(),
            Some("B26952926.java" | "palantir-expression-lambda-2.java")
        )
    })
}

fn fixture_root(suite: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("tools/import/.imports")
        .join(suite)
        .join("input")
}

fn collect_java_files(root: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(root).unwrap_or_else(|error| {
        panic!(
            "failed to read fixture directory {}: {error}",
            root.display()
        )
    }) {
        let path = entry.expect("valid directory entry").path();
        if path.is_dir() {
            collect_java_files(&path, files);
        } else if path
            .extension()
            .is_some_and(|extension| extension == "java")
        {
            files.push(path);
        }
    }
}

struct CorpusSummary {
    suite: String,
    files: usize,
    parser_diagnostics: BTreeMap<String, usize>,
    lexer_diagnostics: BTreeMap<String, usize>,
}

impl CorpusSummary {
    fn new(suite: &str, files: usize) -> Self {
        Self {
            suite: suite.to_owned(),
            files,
            parser_diagnostics: BTreeMap::new(),
            lexer_diagnostics: BTreeMap::new(),
        }
    }

    fn record(&mut self, parse: &JavaParse) {
        for diagnostic in parse.diagnostics() {
            match diagnostic.stage {
                DiagnosticStage::Parser => {
                    increment_rendered(
                        &mut self.parser_diagnostics,
                        diagnostic.code.as_str().to_owned(),
                    );
                }
                DiagnosticStage::Lexer => {
                    increment_rendered(
                        &mut self.lexer_diagnostics,
                        diagnostic.code.as_str().to_owned(),
                    );
                }
                DiagnosticStage::Formatter => {}
            }
        }
    }

    fn render(&self) -> String {
        let mut output = String::new();
        writeln!(&mut output, "suite: {}", self.suite).expect("write summary");
        writeln!(&mut output, "files: {}", self.files).expect("write summary");
        output.push_str("\nparser diagnostics:\n");
        push_counts(&mut output, &self.parser_diagnostics);
        output.push_str("\nlexer diagnostics:\n");
        push_counts(&mut output, &self.lexer_diagnostics);
        output
    }
}

fn increment_rendered(counts: &mut BTreeMap<String, usize>, key: String) {
    *counts.entry(key).or_default() += 1;
}

fn push_counts(output: &mut String, counts: &BTreeMap<String, usize>) {
    if counts.is_empty() {
        output.push_str("  <none>: 0\n");
        return;
    }

    for (kind, count) in counts {
        writeln!(output, "  {kind}: {count}").expect("write summary");
    }
}
