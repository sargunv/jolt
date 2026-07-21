use jolt_test_support::{
    RepresentedTokenRemoval, collect_java_files, java_fixture_root, run_recovery_corpus,
};

mod common;

use common::JavaCorpus;

#[test]
fn java_recovery_formatter_snapshots() {
    let root = java_fixture_root(env!("CARGO_MANIFEST_DIR"));
    let recovery_root = root.join("syntax/recovery");
    let files = collect_java_files(&recovery_root);
    run_recovery_corpus(
        &JavaCorpus,
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
        return NORMALIZATION_REMOVALS;
    }
    match path.file_name().and_then(|name| name.to_str()) {
        Some("empty-statement-comments.java" | "top-level-empty-declaration-comments.java") => {
            &[RepresentedTokenRemoval {
                source: ";",
                count: 1,
            }]
        }
        _ => &[],
    }
}
