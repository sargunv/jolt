use std::path::PathBuf;

use jolt_kotlin_syntax::parse_kotlin_file;
use jolt_test_support::{collect_kotlin_files, read_to_string, workspace_root};

#[test]
fn fixture_kotlin_sources_parse_without_loss() {
    for suite in ["ktfmt", "maplibre-compose"] {
        let root = fixture_root(suite);
        for path in collect_kotlin_files(&root) {
            let source = read_to_string(&path);
            let parse = parse_kotlin_file(&source);
            let syntax = parse.syntax().unwrap_or_else(|| {
                panic!(
                    "parser produced no represented tree for {}: {:#?}",
                    path.display(),
                    parse.diagnostics()
                )
            });
            assert_eq!(
                syntax.source_text(),
                source,
                "syntax tree did not reconstruct exactly for {}",
                path.display()
            );
            assert!(
                parse.diagnostics().is_empty(),
                "imported Kotlin source produced diagnostics for {}: {:#?}",
                path.display(),
                parse.diagnostics()
            );
        }
    }
}

fn fixture_root(suite: &str) -> PathBuf {
    workspace_root(env!("CARGO_MANIFEST_DIR"))
        .join("tools/import/.imports")
        .join(suite)
        .join("source")
}
