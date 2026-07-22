use jolt_test_support::{collect_java_files, java_fixture_root, run_recovery_corpus};

mod common;

use common::JavaCorpus;

#[test]
fn java_recovery_formatter_snapshots() {
    let root = java_fixture_root(env!("CARGO_MANIFEST_DIR"));
    let recovery_root = root.join("syntax/recovery");
    let files = collect_java_files(&recovery_root);
    run_recovery_corpus(&JavaCorpus, &recovery_root, &files, |name, snapshot| {
        insta::assert_snapshot!(name, snapshot);
    });
}
