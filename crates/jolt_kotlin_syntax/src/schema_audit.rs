use jolt_diagnostics::{Diagnostic, DiagnosticStage};
use jolt_test_support::{
    SchemaAudit, assert_exact_structural_ownership_requiring, collect_kotlin_files,
    fixture_snapshot_name, kotlin_fixture_root, read_to_string,
};

use crate::{
    KotlinSyntaxKind, KotlinSyntaxView, parse_kotlin_file, parser::KotlinParseDiagnosticCode,
};

macro_rules! kotlin_audit_matches {
    ($slot:ident, (token $kind:ident)) => {
        matches!($slot, jolt_syntax::SyntaxSlot::Token(token) if token.kind() == KotlinSyntaxKind::$kind)
    };
    ($slot:ident, (token_set [$($kind:ident),*])) => {
        matches!($slot, jolt_syntax::SyntaxSlot::Token(token) if matches!(token.kind(), $(KotlinSyntaxKind::$kind)|*))
    };
    ($slot:ident, (element_set [$($kind:ident),*])) => {
        match $slot {
            jolt_syntax::SyntaxSlot::Node(node) => matches!(node.kind(), $(KotlinSyntaxKind::$kind)|*),
            jolt_syntax::SyntaxSlot::Token(token) => matches!(token.kind(), $(KotlinSyntaxKind::$kind)|*),
            jolt_syntax::SyntaxSlot::Empty => false,
        }
    };
    ($slot:ident, (contextual $text:literal)) => {
        matches!($slot, jolt_syntax::SyntaxSlot::Token(token) if token.kind() == KotlinSyntaxKind::Identifier && token.text() == $text)
    };
    ($slot:ident, (node $kind:ident)) => {
        matches!($slot, jolt_syntax::SyntaxSlot::Node(node) if node.kind() == KotlinSyntaxKind::$kind)
    };
    ($slot:ident, (constructed $kind:ident)) => {
        kotlin_audit_matches!($slot, (node $kind))
    };
    ($slot:ident, (list $kind:ident)) => {
        kotlin_audit_matches!($slot, (node $kind))
    };
    ($slot:ident, (node_set [$($kind:ident),*])) => {
        matches!($slot, jolt_syntax::SyntaxSlot::Node(node) if matches!(node.kind(), $(KotlinSyntaxKind::$kind)|*))
    };
    ($slot:ident, (category $category:ident)) => {
        matches!($slot, jolt_syntax::SyntaxSlot::Node(node) if audit_category_accepts(stringify!($category), node.kind()))
    };
    ($slot:ident, (any_node)) => {
        matches!($slot, jolt_syntax::SyntaxSlot::Node(_))
    };
    ($slot:ident, (any_element)) => {
        !matches!($slot, jolt_syntax::SyntaxSlot::Empty)
    };
    ($slot:ident, (choice [$($matcher:tt),*])) => {
        false $(|| kotlin_audit_matches!($slot, $matcher))*
    };
}

macro_rules! define_kotlin_physical_audit {
    (
        tokens { $($token:ident,)* }
        categories { $($family:ident => $bogus:ident { $($member:ident,)* })* }
        nodes { $($kind:ident => $wrapper:ident [$module:ident $class:ident] { $($fields:tt)* })* }
    ) => {
        fn audit_category_accepts(category: &str, kind: KotlinSyntaxKind) -> bool {
            match category {
                $(stringify!($family) => matches!(
                    kind,
                    KotlinSyntaxKind::$bogus $(| KotlinSyntaxKind::$member)*
                ),)*
                _ => false,
            }
        }

        jolt_test_support::__define_physical_schema_audit! {
            kind: KotlinSyntaxKind,
            language: crate::KotlinLanguage,
            matches: kotlin_audit_matches,
            accepts_malformed: |slot| matches!(
                slot,
                jolt_syntax::SyntaxSlot::Node(child) if child.is_directly_malformed()
            ),
            visibility:,
            tokens { $($token,)* }
            categories { $($family => $bogus { $($member,)* })* }
            nodes { $($kind => $wrapper [$module $class] { $($fields)* })* }
        }
    };
}

kotlin_syntax_schema!(define_kotlin_physical_audit);

#[test]
fn declared_schema_matches_represented_corpus() {
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
            &fixture_snapshot_name(&root, &path),
            syntax
                .syntax_node()
                .expect("typed Kotlin root must have a physical syntax node"),
            !parse.diagnostics().is_empty(),
            audit_physical_node,
        );
        assert_exact_structural_ownership_requiring(
            syntax
                .syntax_node()
                .expect("typed Kotlin root must have a physical syntax node"),
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
        && !matches!(
            diagnostic.code,
            code
                if code == KotlinParseDiagnosticCode::InvalidWhenGuard.id()
                    || code == KotlinParseDiagnosticCode::ReservedCallableReferenceCall.id()
        )
}
