//! Bug-hunting sweep over the repository fixtures and the imported corpora in
//! `tools/import/.imports` (`common::SWEEP_SUITES`): every file at several
//! widths, with every `PROBE_SHAPES` comment shape injected at token
//! boundaries, and (for small files of the exhaustive suites) with each token
//! deleted, checked with the oracle. Run with `mise run hunt` (release mode).
//! `jcheck` replays one file or variant.

use jolt_test_support::{CorpusSweep, run_corpus_sweep, workspace_root};

#[path = "../tests/common/mod.rs"]
mod common;

/// Failure categories known to fire. Must stay empty: a failure is a bug to
/// fix, not debt to carry.
const KNOWN_OPEN: &[&str] = &[];

fn main() {
    let code = run_corpus_sweep(
        &common::JavaCorpus,
        &workspace_root(env!("CARGO_MANIFEST_DIR")),
        &CorpusSweep {
            suites: common::SWEEP_SUITES,
            known_open: KNOWN_OPEN,
        },
    );
    std::process::exit(code);
}
