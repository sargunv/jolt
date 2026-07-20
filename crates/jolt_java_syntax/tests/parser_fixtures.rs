use std::path::PathBuf;

use jolt_diagnostics::{DiagnosticCodeId, DiagnosticStage};
use jolt_java_syntax::{JavaSyntaxView, parse_compilation_unit};
use jolt_test_support::{
    assert_bidirectional_diagnostic_ownership, collect_java_files, read_to_string, workspace_root,
};

#[test]
fn fixture_java_inputs_parse_without_loss() {
    for suite in [
        "google-java-format",
        "palantir-java-format",
        "prettier-java",
    ] {
        let root = fixture_root(suite);
        for path in collect_java_files(&root) {
            let source = read_to_string(&path);
            let parse = parse_compilation_unit(&source);
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
            assert_bidirectional_diagnostic_ownership(
                syntax.syntax_node().expect("represented compilation unit"),
                parse.diagnostics(),
                parse.structural_diagnostic_owners(),
                java_diagnostic_requires_owner,
                path.display(),
            );
        }
    }
}

fn java_diagnostic_requires_owner(diagnostic: &jolt_diagnostics::Diagnostic) -> bool {
    diagnostic.stage == DiagnosticStage::Parser
        && diagnostic.code
            != DiagnosticCodeId::new("java.parse.unqualified_yield_method_invocation")
        && diagnostic.code != DiagnosticCodeId::new("java.parse.decimal_integer_boundary_literal")
}

fn fixture_root(suite: &str) -> PathBuf {
    workspace_root(env!("CARGO_MANIFEST_DIR"))
        .join("tools/import/.imports")
        .join(suite)
        .join("input")
}
