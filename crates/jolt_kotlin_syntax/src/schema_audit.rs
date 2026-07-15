use jolt_test_support::{
    SchemaAudit, assert_schema_deterministic, collect_kotlin_files, fixture_snapshot_name,
    kotlin_fixture_root, read_to_string,
};

use crate::{KotlinSyntaxView, parse_kotlin_file, shape::SCHEMA};

#[test]
fn declared_schema_matches_represented_corpus() {
    assert_schema_deterministic(&SCHEMA);
    let root = kotlin_fixture_root(env!("CARGO_MANIFEST_DIR"));
    let paths = collect_kotlin_files(&root);
    let mut audit = SchemaAudit::new("kotlin");

    for path in paths {
        let source = read_to_string(&path);
        let parse = parse_kotlin_file(&source);
        let syntax = parse.syntax().unwrap_or_else(|| {
            panic!("parser produced no represented tree for {}", path.display())
        });
        audit.visit(
            &SCHEMA,
            &fixture_snapshot_name(&root, &path),
            syntax
                .syntax_node()
                .expect("typed Kotlin root must have a physical syntax node"),
            !parse.diagnostics().is_empty(),
        );
    }

    insta::with_settings!({ snapshot_path => "../tests/snapshots" }, {
        insta::assert_snapshot!("schema_audit", audit.render());
    });
}
