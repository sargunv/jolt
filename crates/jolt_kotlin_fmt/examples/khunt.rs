// Bug-hunting harness: runs the shared conservation sweep over the imported
// real-world Kotlin corpora in tools/import/.imports.
// Not part of the test suite; run with `mise run hunt:kotlin`, or
// `cargo run -p jolt_kotlin_fmt --example khunt --release`.
//
// The sweep lives in `jolt_test_support::run_corpus_sweep`, so `kcheck` replays
// the exact same checks and probe placement and this file is only the corpus to
// point it at. Every file is checked at several line widths and with every
// `PROBE_SHAPES` comment shape injected at token boundaries, for clean reparse,
// structure-fingerprint and comment-inventory conservation, and idempotence.

use std::path::Path;

use jolt_test_support::{CorpusSweep, run_corpus_sweep};

// The corpus policy is the test suite's, so the sweep cannot drift onto a stale
// one.
#[path = "../tests/common/mod.rs"]
mod common;

fn main() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("repository root above the crate directory");
    let code = run_corpus_sweep(
        &common::KotlinCorpus,
        repo_root,
        &CorpusSweep {
            suites: &["ktfmt/source", "maplibre-compose/source"],
            extensions: &["kt", "kts"],
            skip_suffixes: &[],
            out_dir: "kotlin",
            known_open: KNOWN_OPEN,
        },
    );
    std::process::exit(code);
}

/// Failure categories this corpus is already known to produce.
///
/// The sweep exits non-zero on any category outside this list, and on any entry
/// here that stops firing -- a stale entry would let the bug it records be
/// forgotten, and a missing one is a regression. Each entry names the issue that
/// tracks it, so growing the list is a deliberate act with a paper trail.
const KNOWN_OPEN: &[&str] = &[
    // #225: a line comment before an argument list's open paren gains a space on
    // the second pass.
    "linecomment:not-idempotent",
    // #224: an own-line comment is dropped or duplicated at a few positions.
    "ownline:comments-changed",
    // #223: a leading comment takes a line of its own at every site that has not
    // been taught otherwise, and the reparse reads it back as a trailing comment
    // of the preceding token and inlines it, so the first pass is not the
    // fixpoint. Systemic: every leading-comment site on a token that does not
    // begin its line.
    "ownline:not-idempotent",
    "ownlinecomment:comments-changed",
    "ownlinecomment:not-idempotent",
];
