use jolt_test_support::{
    SchemaAudit, assert_schema_deterministic, collect_java_files, fixture_snapshot_name,
    java_fixture_root, read_to_string,
};

use crate::{parse_compilation_unit, shape::SCHEMA};

#[test]
fn declared_schema_matches_represented_corpus() {
    assert_schema_deterministic(&SCHEMA);
    let root = java_fixture_root(env!("CARGO_MANIFEST_DIR"));
    let paths = collect_java_files(&root);
    let mut audit = SchemaAudit::new("java");

    for path in paths {
        let source = read_to_string(&path);
        let parse = parse_compilation_unit(&source);
        let syntax = parse.syntax().unwrap_or_else(|| {
            panic!("parser produced no represented tree for {}", path.display())
        });
        audit.visit(
            &SCHEMA,
            &fixture_snapshot_name(&root, &path),
            *syntax.syntax(),
            !parse.diagnostics().is_empty(),
        );
    }

    insta::with_settings!({ snapshot_path => "../tests/snapshots" }, {
        insta::assert_snapshot!("schema_audit", audit.render());
    });
}
