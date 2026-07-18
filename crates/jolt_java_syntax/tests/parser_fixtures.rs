use std::path::PathBuf;

use jolt_diagnostics::{DiagnosticCodeId, DiagnosticStage};
use jolt_java_syntax::{JavaSyntaxView, parse_compilation_unit};
use jolt_test_support::{
    CorpusSummary, assert_bidirectional_diagnostic_ownership, collect_java_files, read_to_string,
    workspace_root,
};

#[test]
fn fixture_java_inputs_parse_without_loss() {
    let google_summary = assert_corpus("google-java-format");
    let palantir_summary = assert_corpus("palantir-java-format");
    let prettier_summary = assert_corpus("prettier-java");

    insta::assert_snapshot!("google_java_format_parser_summary", google_summary.render());
    insta::assert_snapshot!(
        "palantir_java_format_parser_summary",
        palantir_summary.render()
    );
    insta::assert_snapshot!("prettier_java_parser_summary", prettier_summary.render());
}

fn assert_corpus(suite: &str) -> CorpusSummary {
    let root = fixture_root(suite);
    let files = collect_java_files(&root);
    let mut summary = CorpusSummary::new(suite, files.len());
    for path in files {
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
        summary.record_diagnostics(parse.diagnostics());
    }

    summary
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
