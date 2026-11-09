//! Fixture collection and snapshot helpers, the formatter oracle, the fixture
//! corpus drivers, and the corpus sweep.

#![allow(
    clippy::missing_panics_doc,
    reason = "test support panics on a broken fixture or environment, so the test fails"
)]

mod corpus;
pub mod doc_oracle;
mod oracle;
mod sweep;

pub use corpus::{oracle_over_files, run_formatter_corpus, run_recovery_corpus};
pub use oracle::{CorpusLanguage, StructurePolicy, check_source, parse_facts};
pub use sweep::{
    CorpusSweep, Depth, PROBE_SHAPES, ProbeShape, Suite, Variant, catching, probed_source,
    run_corpus_sweep, sweep_source, sweep_variant,
};

use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use jolt_fmt::FormatOptions;
use jolt_syntax::Diagnostic;
use rayon::prelude::*;

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

/// The Java files under `root`, sorted.
///
/// # Panics
///
/// When `root` is missing, holds no Java file, or can't be read.
#[must_use]
pub fn collect_java_files(root: &Path) -> Vec<PathBuf> {
    assert!(
        root.is_dir(),
        "required Java fixture directory is missing: {}",
        root.display()
    );
    let files = files_with_extensions(root, &["java"]);
    assert!(
        !files.is_empty(),
        "expected at least one Java fixture under {}",
        root.display()
    );
    files
}

/// The files under `dir` with one of `extensions`, sorted.
///
/// # Panics
///
/// When a directory can't be read.
#[must_use]
pub fn files_with_extensions(dir: &Path, extensions: &[&str]) -> Vec<PathBuf> {
    fn walk(dir: &Path, extensions: &[&str], files: &mut Vec<PathBuf>) {
        let entries = fs::read_dir(dir)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", dir.display()));
        for entry in entries {
            let path = entry
                .unwrap_or_else(|error| panic!("failed to read {}: {error}", dir.display()))
                .path();
            if path.is_dir() {
                walk(&path, extensions, files);
            } else if path
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| extensions.contains(&extension))
            {
                files.push(path);
            }
        }
    }
    let mut files = Vec::new();
    walk(dir, extensions, &mut files);
    files.sort();
    files
}

/// Folds `items` on every core and merges the results in item order, so the
/// result does not depend on scheduling when `merge` keeps order.
pub fn par_fold<T: Sync, A: Default + Send>(
    items: &[T],
    fold: impl Fn(&mut A, &T) + Sync + Send,
    merge: impl Fn(&mut A, A) + Sync + Send,
) -> A {
    items
        .par_iter()
        .fold(A::default, |mut acc, item| {
            fold(&mut acc, item);
            acc
        })
        .reduce(A::default, |mut left, right| {
            merge(&mut left, right);
            left
        })
}

/// `path` relative to `root`, with `/` separators.
fn relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

/// The default options at `width`.
fn options(width: u16) -> FormatOptions {
    FormatOptions {
        line_width: width,
        ..FormatOptions::default()
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
        .map(|path| relative(root, path))
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
