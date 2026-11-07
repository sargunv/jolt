//! Bug-hunting sweep over the imported corpora in `tools/import/.imports`:
//! every file at several widths and with every `PROBE_SHAPES` comment shape
//! injected at token boundaries, checked with the oracle. Run with
//! `mise run hunt` (release mode). `jcheck` replays one file or probe.

use std::path::Path;

use jolt_test_support::{CorpusSweep, run_corpus_sweep};

#[path = "../tests/common/mod.rs"]
mod common;

/// Failure categories known to fire. Must stay empty: a failure is a bug to
/// fix, not debt to carry.
const KNOWN_OPEN: &[&str] = &[];

fn main() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("repository root above the crate directory");
    let code = run_corpus_sweep(
        &common::JavaCorpus,
        repo_root,
        &CorpusSweep {
            suites: &[
                "google-java-format/input",
                "palantir-java-format/input",
                "prettier-java/input",
                "spring-framework",
            ],
            extensions: &["java"],
            skip_suffixes: &["B26952926.java"],
            out_dir: "java",
            known_open: KNOWN_OPEN,
        },
    );
    std::process::exit(code);
}
