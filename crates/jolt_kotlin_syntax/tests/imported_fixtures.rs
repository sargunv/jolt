use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use jolt_kotlin_syntax::parse_kotlin_file;
use jolt_test_support::{read_to_string, workspace_root};

#[test]
fn fixture_kotlin_sources_parse_without_loss() {
    let ktfmt_summary = assert_corpus("ktfmt", 72);
    let maplibre_summary = assert_corpus("maplibre-compose", 484);

    insta::assert_snapshot!("ktfmt_parser_summary", ktfmt_summary.render());
    insta::assert_snapshot!("maplibre_compose_parser_summary", maplibre_summary.render());
}

fn assert_corpus(suite: &str, expected_files: usize) -> ImportedParserSummary {
    let root = fixture_root(suite);
    let files = collect_kotlin_files(&root);

    assert_eq!(
        files.len(),
        expected_files,
        "expected the pinned {suite} Kotlin source fixture corpus"
    );

    let mut summary = ImportedParserSummary::new(suite, files.len());
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
    }

    summary
}

fn fixture_root(suite: &str) -> PathBuf {
    workspace_root(env!("CARGO_MANIFEST_DIR"))
        .join("tools/import/.imports")
        .join(suite)
        .join("source")
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
        } else if path
            .extension()
            .is_some_and(|extension| extension == "kt" || extension == "kts")
        {
            files.push(path);
        }
    }
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
