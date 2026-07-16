use jolt_diagnostics::{Diagnostic, DiagnosticStage};
use jolt_test_support::{
    SchemaAudit, assert_exact_structural_ownership_requiring, collect_java_files,
    fixture_snapshot_name, java_fixture_root, read_to_string,
};

use crate::{
    parse_compilation_unit,
    parser::JavaParseDiagnosticCode,
    shape::{audit_physical_node, is_required_slot},
};

#[test]
fn declared_schema_matches_represented_corpus() {
    let root = java_fixture_root(env!("CARGO_MANIFEST_DIR"));
    let paths = collect_java_files(&root);
    let mut audit = SchemaAudit::new("java");

    for path in paths {
        let source = read_to_string(&path);
        let parse = parse_compilation_unit(&source);
        let syntax = parse.syntax().unwrap_or_else(|| {
            panic!(
                "parser produced no represented tree for {}: {:?}",
                path.display(),
                parse.diagnostics()
            )
        });
        audit.visit(
            &fixture_snapshot_name(&root, &path),
            *syntax.syntax(),
            !parse.diagnostics().is_empty(),
            audit_physical_node,
        );
        assert_exact_structural_ownership_requiring(
            *syntax.syntax(),
            parse.diagnostics(),
            parse.structural_diagnostic_owners(),
            is_required_slot,
            diagnostic_requires_owner,
            path.display(),
        );
    }

    insta::with_settings!({ snapshot_path => "../tests/snapshots" }, {
        insta::assert_snapshot!("schema_audit", audit.render());
    });
}

fn diagnostic_requires_owner(diagnostic: &Diagnostic) -> bool {
    diagnostic.stage == DiagnosticStage::Parser
        && diagnostic.code != JavaParseDiagnosticCode::DecimalIntegerBoundaryLiteral.id()
}
