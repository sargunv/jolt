use std::path::PathBuf;

use jolt_diagnostics::{DiagnosticCodeId, DiagnosticStage};
use jolt_java_syntax::{
    BogusName, JavaFamily, JavaNode, JavaSyntaxField, JavaSyntaxListPart, JavaSyntaxView,
    ModuleDeclaration, ModuleDirective, ModuleImplementationClause, ModuleTargetClause, NameSyntax,
    parse_compilation_unit,
};
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

#[test]
fn malformed_module_name_stops_before_directives_when_open_brace_is_missing() {
    let source = "module recovered.\n  uses z.Service;\n  requires a.module;\n}";
    let parse = parse_compilation_unit(source);
    let root = parse
        .syntax()
        .expect("represented compilation unit")
        .syntax_node()
        .expect("physical compilation unit");
    let mut nodes = vec![root];
    let module = loop {
        let node = nodes.pop().expect("module declaration");
        nodes.extend(node.children());
        if let Some(module) = ModuleDeclaration::cast(node) {
            break module;
        }
    };

    let JavaSyntaxField::Present(NameSyntax::BogusName(name)) =
        module.name().expect("module name field")
    else {
        panic!("module name must be diagnostic-owned bogus syntax");
    };
    assert_eq!(
        name.token_iter()
            .map(|token| token.text())
            .collect::<Vec<_>>(),
        ["recovered", "."]
    );
    assert!(matches!(
        module.open_brace().expect("module open brace field"),
        JavaSyntaxField::Missing(_)
    ));
    assert!(matches!(
        module.close_brace().expect("module close brace field"),
        JavaSyntaxField::Present(_)
    ));

    let JavaSyntaxField::Present(directives) =
        module.directives().expect("module directives field")
    else {
        panic!("module directives must remain structured");
    };
    let directives = directives
        .parts()
        .filter_map(
            |part| match part.expect("structured module directive list") {
                JavaSyntaxListPart::Item(directive) => Some(directive),
                JavaSyntaxListPart::Separator(_) => None,
                JavaSyntaxListPart::Missing(_) | JavaSyntaxListPart::Malformed(_) => {
                    panic!("module directive list must not recover its entries")
                }
            },
        )
        .collect::<Vec<_>>();
    assert!(matches!(
        directives.as_slice(),
        [
            ModuleDirective::UsesDirective(_),
            ModuleDirective::RequiresDirective(_)
        ]
    ));
}

#[test]
fn malformed_qualified_names_are_bounded_in_every_module_name_role() {
    let source = "\
module m. {
  requires dependency.;
  exports api. to target.;
  opens internal. to friend.;
  uses service.;
  provides service. with implementation., other.;
}";
    let parse = parse_compilation_unit(source);
    let root = parse
        .syntax()
        .expect("represented compilation unit")
        .syntax_node()
        .expect("physical compilation unit");
    let mut nodes = vec![root];
    let mut bogus_names = Vec::new();
    let mut directive_kinds = Vec::new();
    while let Some(node) = nodes.pop() {
        nodes.extend(node.children());
        if let Some(name) = BogusName::cast(node) {
            bogus_names.push(
                name.token_iter()
                    .map(|token| token.text())
                    .collect::<String>(),
            );
        }
        if let Some(directive) = ModuleDirective::cast(node) {
            directive_kinds.push(match directive {
                ModuleDirective::RequiresDirective(_) => "requires",
                ModuleDirective::ExportsDirective(_) => "exports",
                ModuleDirective::OpensDirective(_) => "opens",
                ModuleDirective::UsesDirective(_) => "uses",
                ModuleDirective::ProvidesDirective(_) => "provides",
                ModuleDirective::BogusModuleDirective(_) => "bogus",
            });
        }
    }

    bogus_names.sort_unstable();
    assert_eq!(
        bogus_names,
        [
            "api.",
            "dependency.",
            "friend.",
            "implementation.",
            "internal.",
            "m.",
            "other.",
            "service.",
            "service.",
            "target.",
        ]
    );
    directive_kinds.sort_unstable();
    assert_eq!(
        directive_kinds,
        ["exports", "opens", "provides", "requires", "uses"]
    );
}

#[test]
fn module_name_boundaries_preserve_following_directives() {
    let source = "\
module m {
  exports api. requires dep;
  opens api. uses Service;
  provides Service. requires dep;
  exports api requires dep;
  provides Service uses Other;
  exports api to requires dep;
  provides Service with Impl, uses Other;
  exports ordinary. to target.;
  opens ordinary. to friend.;
  provides Ordinary. with Implementation.;
  exports a.requires;
  opens a.uses;
  provides a.Service with b.provides;
  exports valid to requires;
  provides Valid with uses;
  exports exports to target;
  opens opens to friend;
  provides provides with Impl;
  exports api requires.foo;
}";
    let parse = parse_compilation_unit(source);
    let root = parse
        .syntax()
        .expect("represented compilation unit")
        .syntax_node()
        .expect("physical compilation unit");
    let mut nodes = vec![root.clone()];
    let mut directive_kinds = Vec::new();
    let mut target_clause_count = 0;
    let mut implementation_clause_count = 0;
    while let Some(node) = nodes.pop() {
        nodes.extend(node.children());
        if let Some(directive) = ModuleDirective::cast(node) {
            directive_kinds.push(match directive {
                ModuleDirective::RequiresDirective(_) => "requires",
                ModuleDirective::ExportsDirective(_) => "exports",
                ModuleDirective::OpensDirective(_) => "opens",
                ModuleDirective::UsesDirective(_) => "uses",
                ModuleDirective::ProvidesDirective(_) => "provides",
                ModuleDirective::BogusModuleDirective(_) => "bogus",
            });
        }
        target_clause_count += usize::from(ModuleTargetClause::cast(node).is_some());
        implementation_clause_count +=
            usize::from(ModuleImplementationClause::cast(node).is_some());
    }

    directive_kinds.sort_unstable();
    assert_eq!(
        directive_kinds,
        [
            "exports", "exports", "exports", "exports", "exports", "exports", "exports", "exports",
            "opens", "opens", "opens", "opens", "provides", "provides", "provides", "provides",
            "provides", "provides", "provides", "requires", "requires", "requires", "requires",
            "uses", "uses", "uses",
        ]
    );
    assert_eq!(target_clause_count, 7);
    assert_eq!(implementation_clause_count, 5);
    assert_bidirectional_diagnostic_ownership(
        root,
        parse.diagnostics(),
        parse.structural_diagnostic_owners(),
        java_diagnostic_requires_owner,
        "module qualified-name boundary ownership",
    );
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
