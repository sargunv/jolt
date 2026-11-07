//! Replays the sweep for the files on the command line, or one probe with
//! `<path>:<shape><n>` (e.g. `Foo.java:ownline12`), printing the probed source
//! and every failure. Run with
//! `cargo run --release -p jolt_java_fmt --example jcheck -- <args>`.

use std::fs;
use std::path::PathBuf;

use jolt_fmt::FormatOptions;
use jolt_test_support::{
    CorpusLanguage, catching, parse_probe_argument, probe_boundaries, probed_source, sweep_source,
    sweep_variant,
};

#[path = "../tests/common/mod.rs"]
mod common;

fn main() {
    std::panic::set_hook(Box::new(|_| {}));
    let mut any = false;
    for arg in std::env::args().skip(1) {
        let replay = arg
            .rsplit_once(':')
            .and_then(|(file, probe)| parse_probe_argument(probe).map(|probe| (file, probe)));
        let failures = if let Some((file, (shape, probe))) = replay {
            let path = PathBuf::from(file);
            let source = fs::read_to_string(&path).expect("read input");
            let boundaries = common::JavaCorpus.token_end_offsets(&source);
            let selected = probe_boundaries(shape, &source, boundaries.len());
            let Some(&boundary) = selected.get(probe) else {
                println!(
                    "{}: probe {}{probe} is out of range ({} positions)",
                    path.display(),
                    shape.name,
                    selected.len()
                );
                continue;
            };
            let probed = probed_source(&source, boundaries[boundary], shape);
            println!("=== probed source ({}{probe}):\n{probed}", shape.name);
            catching(&path, || {
                sweep_variant(
                    &common::JavaCorpus,
                    &probed,
                    &FormatOptions::default(),
                    &format!("{}{probe}", shape.name),
                )
            })
        } else {
            let path = PathBuf::from(&arg);
            let source = fs::read_to_string(&path).expect("read input");
            catching(&path, || sweep_source(&common::JavaCorpus, &source))
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
