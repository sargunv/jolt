//! Fixture corpus drivers: formatter snapshots and recovery snapshots, both
//! checked with the oracle.

use std::path::{Path, PathBuf};

use crate::oracle::{CorpusLanguage, check_source, parse_facts};
use crate::{
    SnapshotBuilder, fixture_snapshot_name, options, read_to_string, relative, render_diagnostics,
};

/// The widths every corpus fixture is checked at.
const ORACLE_WIDTHS: &[u16] = &[40, 80, 120];

fn oracle_failures<C: CorpusLanguage>(
    language: &C,
    source: &str,
    label: &str,
    widths: &[u16],
) -> Vec<String> {
    let mut failures = Vec::new();
    for &width in widths {
        for (check, detail) in check_source(language, source, &options(width)) {
            failures.push(format!("{label} (w{width}): {check}\n{detail}"));
        }
    }
    failures
}

/// Formats every fixture under `root` except `syntax/lexer` and
/// `syntax/recovery`, checks the oracle at [`ORACLE_WIDTHS`], and
/// snapshots the output at width 80. Fixtures expected to carry diagnostics
/// get the oracle without a snapshot.
///
/// # Panics
///
/// On any oracle failure, or when a fixture's diagnostics disagree with
/// [`CorpusLanguage::expects_diagnostics`].
pub fn run_formatter_corpus<C: CorpusLanguage>(
    language: &C,
    root: &Path,
    files: &[PathBuf],
    mut snapshot: impl FnMut(&str, &str),
) {
    let mut failures = Vec::new();
    let mut formatted_cases = 0;
    for path in files {
        let relative = relative(root, path);
        if relative.starts_with("syntax/lexer") || relative.starts_with("syntax/recovery") {
            continue;
        }
        let source = read_to_string(path);
        let facts = parse_facts(language, &source);
        let expected = language.expects_diagnostics(&relative);
        assert_eq!(
            !facts.diagnostics.is_empty(),
            expected,
            "{} fixture route changed for {relative}: {:#?}",
            language.name(),
            facts.diagnostics
        );
        failures.extend(oracle_failures(language, &source, &relative, ORACLE_WIDTHS));
        if expected {
            continue;
        }
        formatted_cases += 1;
        let formatted = language.format(&source, &options(80));
        let body = SnapshotBuilder::new()
            .section("formatted", &formatted)
            .section("diagnostics", render_diagnostics(&[]))
            .finish();
        snapshot(&fixture_snapshot_name(root, path), &body);
    }
    assert!(
        formatted_cases > 0,
        "expected at least one formatter fixture"
    );
    assert!(
        failures.is_empty(),
        "{} oracle failures:\n{}",
        language.name(),
        failures.join("\n")
    );
}

/// Formats every recovery fixture, checks the oracle at width 80, and
/// snapshots input, output and diagnostics.
///
/// # Panics
///
/// On any oracle failure.
pub fn run_recovery_corpus<C: CorpusLanguage>(
    language: &C,
    root: &Path,
    files: &[PathBuf],
    mut snapshot: impl FnMut(&str, &str),
) {
    assert!(!files.is_empty(), "expected at least one recovery fixture");
    let mut failures = Vec::new();
    for path in files {
        let source = read_to_string(path);
        let label = relative(root, path);
        failures.extend(oracle_failures(language, &source, &label, &[80]));
        let facts = parse_facts(language, &source);
        let formatted = language.format(&source, &options(80));
        let body = SnapshotBuilder::new()
            .section("input", &source)
            .section("formatted", &formatted)
            .section("diagnostics", render_diagnostics(&facts.diagnostics))
            .finish();
        snapshot(&fixture_snapshot_name(root, path), &body);
    }
    assert!(
        failures.is_empty(),
        "{} recovery oracle failures:\n{}",
        language.name(),
        failures.join("\n")
    );
}

/// Checks the oracle at width 80 over every file, returning the failures.
pub fn oracle_over_files<C: CorpusLanguage>(
    language: &C,
    root: &Path,
    files: &[PathBuf],
) -> Vec<String> {
    let mut failures = Vec::new();
    for path in files {
        let source = read_to_string(path);
        failures.extend(oracle_failures(
            language,
            &source,
            &relative(root, path),
            &[80],
        ));
    }
    failures
}
