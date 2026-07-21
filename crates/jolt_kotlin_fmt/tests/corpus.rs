use jolt_test_support::{collect_kotlin_files, kotlin_fixture_root, run_formatter_corpus};

mod common;

use common::KotlinCorpus;

#[test]
fn kotlin_corpus_formatter_snapshots() {
    let root = kotlin_fixture_root(env!("CARGO_MANIFEST_DIR"));
    let files = collect_kotlin_files(&root);
    run_formatter_corpus(&KotlinCorpus, &root, &files, |name, snapshot| {
        insta::assert_snapshot!(name, snapshot);
    });
}
