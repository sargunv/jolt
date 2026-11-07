//! Fixture collection and snapshot helpers, the formatter oracle, the fixture
//! corpus drivers, and the corpus sweep.

#![allow(clippy::missing_panics_doc)]

mod corpus;
mod oracle;
mod sweep;

pub use corpus::{ORACLE_WIDTHS, oracle_over_files, run_formatter_corpus, run_recovery_corpus};
pub use oracle::{
    CorpusLanguage, ParseFacts, StructurePolicy, canonical_comment_text, check_source,
    comment_inventory, describe_divergence, diagnostic_codes, parse_facts, structure_fingerprint,
};
pub use sweep::{
    CorpusSweep, PROBE_SHAPES, ProbePosition, ProbeShape, SweepFailure, catching,
    parse_probe_argument, probe_boundaries, probed_source, run_corpus_sweep, sweep_source,
    sweep_variant,
};

use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use jolt_syntax::Diagnostic;

#[derive(Default)]
pub struct SnapshotBuilder {
    output: String,
}

impl SnapshotBuilder {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn section(mut self, name: &str, content: impl AsRef<str>) -> Self {
        if !self.output.is_empty() {
            self.output.push('\n');
        }
        writeln!(&mut self.output, "{name}:").expect("write snapshot section header");
        let content = content.as_ref();
        if content.is_empty() {
            self.output.push_str("<empty>\n");
        } else {
            self.output.push_str(content);
            if !content.ends_with('\n') {
                self.output.push('\n');
            }
        }
        self
    }

    #[must_use]
    pub fn finish(self) -> String {
        self.output
    }
}

#[must_use]
pub fn workspace_root(manifest_dir: &str) -> PathBuf {
    Path::new(manifest_dir)
        .ancestors()
        .nth(2)
        .expect("crate manifest dir should be under workspace crates directory")
        .to_path_buf()
}

#[must_use]
pub fn java_fixture_root(manifest_dir: &str) -> PathBuf {
    workspace_root(manifest_dir).join("fixtures/java")
}

#[must_use]
pub fn collect_java_files(root: &Path) -> Vec<PathBuf> {
    collect_fixture_files(root, &["java"], "Java")
}

#[must_use]
pub fn collect_fixture_files(root: &Path, extensions: &[&str], language: &str) -> Vec<PathBuf> {
    assert!(
        root.is_dir(),
        "required {language} fixture directory is missing: {}",
        root.display()
    );

    let mut files = Vec::new();
    collect_fixture_files_into(root, extensions, &mut files);
    files.sort();
    assert!(
        !files.is_empty(),
        "expected at least one {language} fixture under {}",
        root.display()
    );
    files
}

fn collect_fixture_files_into(root: &Path, extensions: &[&str], files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(root)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", root.display()))
    {
        let path = entry.expect("valid directory entry").path();
        if path.is_dir() {
            collect_fixture_files_into(&path, extensions, files);
        } else if path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extensions.contains(&extension))
        {
            files.push(path);
        }
    }
}

#[must_use]
pub fn fixture_snapshot_name(root: &Path, path: &Path) -> String {
    let relative = path.strip_prefix(root).unwrap_or_else(|error| {
        panic!(
            "{} should be under {}: {error}",
            path.display(),
            root.display()
        )
    });
    let without_extension = relative.with_extension("");
    without_extension
        .components()
        .map(|component| component.as_os_str().to_string_lossy().replace('-', "_"))
        .collect::<Vec<_>>()
        .join("__")
}

#[must_use]
pub fn fixture_manifest(root: &Path, paths: &[PathBuf]) -> String {
    paths
        .iter()
        .map(|path| {
            path.strip_prefix(root)
                .unwrap_or(path)
                .to_string_lossy()
                .replace('\\', "/")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[must_use]
pub fn read_to_string(path: &Path) -> String {
    fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()))
}

#[must_use]
pub fn render_diagnostics(diagnostics: &[Diagnostic]) -> String {
    if diagnostics.is_empty() {
        return "(none)\n".to_owned();
    }

    let mut output = String::new();
    for diagnostic in diagnostics {
        writeln!(
            &mut output,
            "code={} range={} message={}",
            diagnostic.code, diagnostic.range, diagnostic.message
        )
        .expect("write diagnostics");
    }
    output
}
