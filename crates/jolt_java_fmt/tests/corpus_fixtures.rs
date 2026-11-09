//! The oracle over the imported upstream formatter fixture suites and the
//! Markdown spec examples as `///` comments (`commonmark-spec`).

use jolt_test_support::{collect_java_files, oracle_over_files, workspace_root};

mod common;

#[test]
fn imported_fixture_suites_pass_the_oracle() {
    let imports = workspace_root(env!("CARGO_MANIFEST_DIR")).join("tools/import/.imports");
    let mut failures = Vec::new();
    for suite in [
        "google-java-format",
        "palantir-java-format",
        "prettier-java",
        "commonmark-spec",
    ] {
        let root = imports.join(suite).join("input");
        let files = collect_java_files(&root);
        failures.extend(oracle_over_files(&common::JavaCorpus, &root, &files));
    }
    assert!(
        failures.is_empty(),
        "oracle failures:\n{}",
        failures.join("\n")
    );
}
