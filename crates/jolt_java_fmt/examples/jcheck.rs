//! Replays the sweep for the files on the command line, or one variant with
//! `<path>:<variant>` (e.g. `Foo.java:ownline12`, `Foo.java:comment3@40`,
//! `Foo.java:delete7`), printing the variant's source and every failure,
//! the JDK's reading of its doc comments included. Run with
//! `cargo run --release -p jolt_java_fmt --example jcheck -- <args>`.

use std::fs;
use std::path::{Path, PathBuf};

use jolt_test_support::{
    Depth, Variant, catching, doc_oracle, sweep_source, sweep_variant, workspace_root,
};

#[path = "../tests/common/mod.rs"]
mod common;

/// The depth of the sweep suite holding `path` (sampled outside every suite).
fn depth_of(path: &Path) -> Depth {
    let root = workspace_root(env!("CARGO_MANIFEST_DIR"));
    let path = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    common::SWEEP_SUITES
        .iter()
        .find(|suite| fs::canonicalize(root.join(suite.dir)).is_ok_and(|dir| path.starts_with(dir)))
        .map_or(Depth::Sampled, |suite| suite.depth)
}

fn main() {
    std::panic::set_hook(Box::new(|_| {}));
    doc_oracle::enable(&workspace_root(env!("CARGO_MANIFEST_DIR")));
    let mut any = false;
    for arg in std::env::args().skip(1) {
        let replay = arg
            .rsplit_once(':')
            .and_then(|(file, label)| Variant::parse(label).map(|variant| (file, variant)));
        doc_oracle::set_file(Path::new(
            arg.rsplit_once(':').map_or(&*arg, |(file, _)| file),
        ));
        let failures = if let Some((file, variant)) = replay {
            let path = PathBuf::from(file);
            let source = fs::read_to_string(&path).expect("read input");
            let Some(varied) = variant.source(&common::JavaCorpus, &source, depth_of(&path)) else {
                println!("{}: {} is out of range", path.display(), variant.label());
                continue;
            };
            println!("=== variant source ({}):\n{varied}", variant.label());
            let options = variant.options();
            catching(&path, || {
                sweep_variant(&common::JavaCorpus, &varied, &options, &variant.label())
            })
        } else {
            let path = PathBuf::from(&arg);
            let source = fs::read_to_string(&path).expect("read input");
            let depth = depth_of(&path);
            catching(&path, || sweep_source(&common::JavaCorpus, &source, depth))
        };
        for failure in failures {
            any = true;
            println!("[{}]\n{}\n", failure.category, failure.detail);
        }
    }
    for failure in doc_oracle::resolve().failures {
        any = true;
        println!(
            "[{}:{}] {}\n{}\n",
            failure.label,
            failure.check,
            failure.path.display(),
            failure.detail
        );
    }
    if !any {
        println!("no failures");
    }
}
