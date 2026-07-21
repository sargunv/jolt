use jolt_test_support::{
    RepresentedTokenRemoval, collect_kotlin_files, kotlin_fixture_root, run_recovery_corpus,
};

mod common;

use common::KotlinCorpus;

#[test]
fn kotlin_recovery_formatter_snapshots() {
    let root = kotlin_fixture_root(env!("CARGO_MANIFEST_DIR"));
    let recovery_root = root.join("syntax/recovery");
    let files = collect_kotlin_files(&recovery_root);
    run_recovery_corpus(
        &KotlinCorpus,
        &recovery_root,
        &files,
        allowed_removed_tokens,
        |name, snapshot| {
            insta::assert_snapshot!(name, snapshot);
        },
    );
}

const NORMALIZATION_REMOVALS: &[RepresentedTokenRemoval] = &[RepresentedTokenRemoval {
    source: ";",
    count: usize::MAX,
}];

fn allowed_removed_tokens(path: &std::path::Path) -> &'static [RepresentedTokenRemoval] {
    if path
        .parent()
        .and_then(std::path::Path::file_name)
        .is_some_and(|name| name == "normalization")
    {
        NORMALIZATION_REMOVALS
    } else {
        &[]
    }
}
