use jolt_test_support::{collect_java_files, java_fixture_root, run_formatter_corpus};

mod common;

use common::JavaCorpus;

#[test]
fn java_corpus_formatter_snapshots() {
    let root = java_fixture_root(env!("CARGO_MANIFEST_DIR"));
    let files = collect_java_files(&root);
    run_formatter_corpus(&JavaCorpus, &root, &files, |name, snapshot| {
        insta::assert_snapshot!(name, snapshot);
    });
}
