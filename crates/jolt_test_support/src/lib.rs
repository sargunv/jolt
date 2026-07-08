#![allow(clippy::missing_panics_doc)]

use std::collections::BTreeMap;
use std::convert::Infallible;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use jolt_diagnostics::{Diagnostic, DiagnosticStage};
use jolt_fmt_ir::{RenderControl, RenderSink};

#[derive(Default)]
pub struct StringSink {
    text: String,
}

impl StringSink {
    #[must_use]
    pub fn into_string(self) -> String {
        self.text
    }
}

impl RenderSink for StringSink {
    type Error = Infallible;

    fn write_str(&mut self, text: &str) -> Result<RenderControl, Self::Error> {
        self.text.push_str(text);
        Ok(RenderControl::Continue)
    }
}

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
pub fn kotlin_fixture_root(manifest_dir: &str) -> PathBuf {
    workspace_root(manifest_dir).join("fixtures/kotlin")
}

#[must_use]
pub fn collect_java_files(root: &Path) -> Vec<PathBuf> {
    assert!(
        root.is_dir(),
        "required Java fixture directory is missing: {}",
        root.display()
    );

    let mut files = Vec::new();
    collect_java_files_into(root, &mut files);
    files.sort();
    assert!(
        !files.is_empty(),
        "expected at least one Java fixture under {}",
        root.display()
    );
    files
}

#[must_use]
pub fn collect_kotlin_files(root: &Path) -> Vec<PathBuf> {
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

fn collect_java_files_into(root: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(root)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", root.display()))
    {
        let path = entry.expect("valid directory entry").path();
        if path.is_dir() {
            collect_java_files_into(&path, files);
        } else if path
            .extension()
            .is_some_and(|extension| extension == "java")
        {
            files.push(path);
        }
    }
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
            "code={} severity={:?} stage={:?} range={:?} message={}",
            diagnostic.code.as_str(),
            diagnostic.severity,
            diagnostic.stage,
            diagnostic.range,
            diagnostic.message
        )
        .expect("write diagnostics");
    }
    output
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CorpusSummary {
    suite: String,
    files: usize,
    diagnostics: BTreeMap<String, usize>,
}

impl CorpusSummary {
    #[must_use]
    pub fn new(suite: &str, files: usize) -> Self {
        Self {
            suite: suite.to_owned(),
            files,
            diagnostics: BTreeMap::new(),
        }
    }

    pub fn record_diagnostics(&mut self, diagnostics: &[Diagnostic]) {
        for diagnostic in diagnostics {
            let key = format!("{:?}:{}", diagnostic.stage, diagnostic.code.as_str());
            *self.diagnostics.entry(key).or_default() += 1;
        }
    }

    #[must_use]
    pub fn render(&self) -> String {
        let mut output = String::new();
        writeln!(&mut output, "suite: {}", self.suite).expect("write summary");
        writeln!(&mut output, "files: {}", self.files).expect("write summary");
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

#[must_use]
pub fn diagnostic_stage_counts(diagnostics: &[Diagnostic], stage: DiagnosticStage) -> usize {
    diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.stage == stage)
        .count()
}
