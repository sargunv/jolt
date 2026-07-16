use jolt_diagnostics::{Diagnostic, DiagnosticStage};
use jolt_test_support::{
    PhysicalNodeAudit, SchemaAudit, assert_exact_structural_ownership_requiring,
    collect_kotlin_files, fixture_snapshot_name, kotlin_fixture_root, read_to_string,
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

macro_rules! kotlin_audit_fixed_field {
    ($slot:ident, $missing:ident, required, $matcher:tt) => {
        match $slot {
            jolt_syntax::SyntaxSlot::Empty => $missing = true,
            _ if kotlin_audit_matches!($slot, $matcher) => {}
            _ => return PhysicalNodeAudit::Unexpected,
        }
    };
    ($slot:ident, $missing:ident, optional, $matcher:tt) => {
        match $slot {
            jolt_syntax::SyntaxSlot::Empty => {}
            _ if kotlin_audit_matches!($slot, $matcher) => {}
            _ => return PhysicalNodeAudit::Unexpected,
        }
    };
}

macro_rules! kotlin_audit_required {
    (required) => {
        true
    };
    ($cardinality:ident) => {
        false
    };
}

macro_rules! kotlin_audit_node {
    ($node:ident, valid; $($field:ident: $cardinality:ident $matcher:tt $(=> $role:ident)? $([$($policy:tt)*])?;)*) => {{
        let mut cursor = 0;
        #[allow(unused_mut)]
        let mut missing = false;
        $(
            let Some(slot) = $node.slot_at(cursor) else {
                return PhysicalNodeAudit::Unexpected;
            };
            kotlin_audit_fixed_field!(slot, missing, $cardinality, $matcher);
            cursor += 1;
        )*
        if cursor != $node.slot_count() {
            PhysicalNodeAudit::Unexpected
        } else if missing {
            PhysicalNodeAudit::MissingRequired
        } else {
            PhysicalNodeAudit::Exact
        }
    }};
    ($node:ident, constructed; $($fields:tt)*) => {
        kotlin_audit_node!($node, valid; $($fields)*)
    };
    ($node:ident, list; $field:ident: $cardinality:ident $matcher:tt $(=> $role:ident)?;) => {{
        let mut missing = false;
        for index in 0..$node.slot_count() {
            let slot = $node.slot_at(index).expect("physical list slot");
            match slot {
                jolt_syntax::SyntaxSlot::Empty => missing = true,
                _ if kotlin_audit_matches!(slot, $matcher) => {}
                _ => return PhysicalNodeAudit::Unexpected,
            }
        }
        if missing {
            PhysicalNodeAudit::MissingRequired
        } else {
            PhysicalNodeAudit::Exact
        }
    }};
    ($node:ident, list; $field:ident: $cardinality:ident $matcher:tt $(=> $role:ident)? [disambiguate $policy:ident];) => {
        kotlin_audit_node!($node, list; $field: $cardinality $matcher $(=> $role)?;)
    };
    ($node:ident, list; $field:ident: $cardinality:ident $matcher:tt $(=> $role:ident)? [separated $separator:tt, minimum $minimum:literal, trailing $trailing:ident, recovery bogus_owner];) => {{
        let mut missing = false;
        for index in 0..$node.slot_count() {
            let slot = $node.slot_at(index).expect("physical separated-list slot");
            if matches!(slot, jolt_syntax::SyntaxSlot::Empty) {
                missing = true;
            } else if index % 2 == 0 {
                if !kotlin_audit_matches!(slot, $matcher) {
                    return PhysicalNodeAudit::Unexpected;
                }
            } else if !kotlin_audit_matches!(slot, $separator) {
                return PhysicalNodeAudit::Unexpected;
            }
        }
        if missing {
            PhysicalNodeAudit::MissingRequired
        } else {
            PhysicalNodeAudit::Exact
        }
    }};
    ($node:ident, malformed; $($fields:tt)*) => {
        PhysicalNodeAudit::Malformed
    };
}

macro_rules! kotlin_required_slot {
    ($slot:ident, valid; $($field:ident: $cardinality:ident $matcher:tt $(=> $role:ident)? $([$($policy:tt)*])?;)*) => {
        [$(kotlin_audit_required!($cardinality)),*]
            .get($slot)
            .copied()
            .unwrap_or(false)
    };
    ($slot:ident, constructed; $($fields:tt)*) => {
        kotlin_required_slot!($slot, valid; $($fields)*)
    };
    ($slot:ident, list; $($fields:tt)*) => {
        true
    };
    ($slot:ident, malformed; $($fields:tt)*) => {
        false
    };
}

macro_rules! define_kotlin_physical_audit {
    (
        tokens { $($token:ident,)* }
        categories { $($family:ident => $bogus:ident { $($member:ident,)* })* }
        nodes {
            $($kind:ident => $wrapper:ident [$module:ident $class:ident] { $($fields:tt)* })*
        }
    ) => {
        fn audit_category_accepts(category: &str, kind: KotlinSyntaxKind) -> bool {
            match category {
                $(stringify!($family) => matches!(kind, KotlinSyntaxKind::$bogus $(| KotlinSyntaxKind::$member)*),)*
                _ => false,
            }
        }

        fn audit_physical_node(
            node: jolt_syntax::SyntaxNode<'_, crate::KotlinLanguage>,
        ) -> PhysicalNodeAudit {
            match node.kind() {
                $(KotlinSyntaxKind::$kind => kotlin_audit_node!(node, $class; $($fields)*),)*
                $(KotlinSyntaxKind::$bogus => PhysicalNodeAudit::Malformed,)*
                _ => PhysicalNodeAudit::Unexpected,
            }
        }

        fn is_required_slot(
            node: jolt_syntax::SyntaxNode<'_, crate::KotlinLanguage>,
            slot: usize,
        ) -> bool {
            match node.kind() {
                $(KotlinSyntaxKind::$kind => kotlin_required_slot!(slot, $class; $($fields)*),)*
                $(KotlinSyntaxKind::$bogus => false,)*
                _ => false,
            }
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
