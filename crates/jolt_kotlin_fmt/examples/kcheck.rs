// Single-file checker: runs the same conservation sweep as khunt against the
// files passed on the command line and prints failures with details.
// Run with `cargo run -p jolt_kotlin_fmt --example kcheck --release -- <files...>`.
//
// `<path>:<shape><n>` replays one probe position instead of the whole sweep and
// prints the probed source, for minimizing a repro khunt reported. The shape
// names are `jolt_test_support::PROBE_SHAPES`, e.g. `Foo.kt:ownline12`. Both
// the sweep and this replay read probe placement from `probe_boundaries`, so the
// position always matches the one khunt used.

use std::fs;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};

use jolt_test_support::{
    CorpusLanguage, SweepFailure, parse_probe_argument, probe_boundaries, probed_source,
    sweep_source, sweep_variant,
};

// The corpus policy is the test suite's, so the check cannot drift onto a
// stale one.
#[path = "../tests/common/mod.rs"]
mod common;

fn main() {
    std::panic::set_hook(Box::new(|_| {}));
    let mut any = false;
    for arg in std::env::args().skip(1) {
        let replay = arg
            .rsplit_once(':')
            .and_then(|(file, probe)| parse_probe_argument(probe).map(|probe| (file, probe)));
        let failures = match replay {
            Some((file, (shape, probe))) => {
                let path = PathBuf::from(file);
                let source = fs::read_to_string(&path).expect("read input");
                let boundaries = common::KotlinCorpus.token_end_offsets(&source);
                let selected = probe_boundaries(shape, &source, boundaries.len());
                let Some(&boundary) = selected.get(probe) else {
                    println!(
                        "{}: probe {}{probe} is out of range; this file has {} probe position(s) for that shape",
                        path.display(),
                        shape.name,
                        selected.len()
                    );
                    continue;
                };
                let probed = probed_source(&source, boundaries[boundary], shape);
                println!(
                    "=== probed source ({}{probe} at boundary {boundary} of {}):",
                    shape.name,
                    boundaries.len()
                );
                println!("{probed}");
                check(&path, move || {
                    sweep_variant(
                        &common::KotlinCorpus,
                        &probed,
                        &jolt_fmt_ir::FormatOptions::default(),
                        &format!("{}{probe}", shape.name),
                    )
                })
            }
            None => {
                let path = PathBuf::from(&arg);
                let source = fs::read_to_string(&path).expect("read input");
                check(&path, move || sweep_source(&common::KotlinCorpus, &source))
            }
        };
        for failure in failures {
            any = true;
            println!("[{}]\n{}\n", failure.category, failure.detail);
        }
    }
    if !any {
        println!("no failures");
    }
}

fn check(path: &Path, run: impl FnOnce() -> Vec<SweepFailure> + Send) -> Vec<SweepFailure> {
    catch_unwind(AssertUnwindSafe(run)).unwrap_or_else(|payload| {
        let message = payload
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| payload.downcast_ref::<&str>().copied())
            .unwrap_or("<non-string panic>");
        vec![SweepFailure {
            category: "panic".to_owned(),
            detail: format!("{}: {message}", path.display()),
        }]
    })
}
