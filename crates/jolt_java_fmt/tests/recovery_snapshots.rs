use jolt_test_support::{collect_java_files, java_fixture_root, run_recovery_corpus};

mod common;

#[test]
fn java_recovery_formatter_snapshots() {
    let root = java_fixture_root(env!("CARGO_MANIFEST_DIR")).join("syntax/recovery");
    let files = collect_java_files(&root);
    run_recovery_corpus(&common::JavaCorpus, &root, &files, |name, snapshot| {
        insta::assert_snapshot!(name, snapshot);
    });
}
