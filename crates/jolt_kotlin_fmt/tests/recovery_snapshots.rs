use jolt_test_support::{collect_kotlin_files, kotlin_fixture_root, run_recovery_corpus};

mod common;

use common::KotlinCorpus;

#[test]
fn kotlin_recovery_formatter_snapshots() {
    let root = kotlin_fixture_root(env!("CARGO_MANIFEST_DIR"));
    let recovery_root = root.join("syntax/recovery");
    let files = collect_kotlin_files(&recovery_root);
    run_recovery_corpus(&KotlinCorpus, &recovery_root, &files, |name, snapshot| {
        insta::assert_snapshot!(name, snapshot);
    });
}
