//! Bug-hunting sweep over the repository fixtures and the imported corpora in
//! `tools/import/.imports` (`common::SWEEP_SUITES`): every file at several
//! widths, with every `PROBE_SHAPES` comment shape injected at token
//! boundaries, and (for small files of the exhaustive suites) with each token
//! deleted, checked with the oracle, and the doc comments of every variant
//! checked against what the JDK reads in them (`doc_oracle`, which needs the
//! JDK that `mise.toml` pins). Run with `mise run hunt` (release mode).
//! `jcheck` replays one file or variant.

use jolt_test_support::{CorpusSweep, doc_oracle, run_corpus_sweep, workspace_root};

#[path = "../tests/common/mod.rs"]
mod common;

/// Failure categories known to fire. Must stay empty: a failure is a bug to
/// fix, not debt to carry.
const KNOWN_OPEN: &[&str] = &[];

fn main() {
    let root = workspace_root(env!("CARGO_MANIFEST_DIR"));
    doc_oracle::enable(&root);
    let code = run_corpus_sweep(
        &common::JavaCorpus,
        &root,
        &CorpusSweep {
            suites: common::SWEEP_SUITES,
            known_open: KNOWN_OPEN,
        },
    );
    std::process::exit(code);
}
