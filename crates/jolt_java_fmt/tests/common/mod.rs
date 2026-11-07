// Shared by several test binaries and examples, each of which uses a part.
#![allow(dead_code)]

use jolt_fmt::FormatOptions;
use jolt_java_syntax::{JavaLanguage, JavaSyntaxKind as K};
use jolt_syntax::{Diagnostic, SyntaxNode};
use jolt_test_support::{CorpusLanguage, StructurePolicy};

/// The tree edits the Java formatter may make (N1–N8): import, module
/// directive, `requires` modifier and declaration modifier reordering, brace
/// promotion, removal of redundant `;` and parentheses, and separator, brace
/// and parenthesis punctuation.
pub const STRUCTURE_POLICY: StructurePolicy<K> = StructurePolicy {
    normalizable_punctuation: &[
        K::Comma,
        K::Semicolon,
        K::LBrace,
        K::RBrace,
        K::LParen,
        K::RParen,
    ],
    unordered_nodes: &[K::ModuleDirectiveList, K::RequiresModifierList],
    unordered_keywords: &[K::ModifierList],
    unordered_keywords_ordered_children: &[K::Annotation],
    reorderable_children: &[K::ImportDeclaration],
    elidable_wrappers: &[
        K::BlockStatementList,
        K::BlockStatement,
        K::ParenthesizedExpression,
    ],
    promoted_body_wrapper: Some(K::Block),
    brace_promoting_parents: &[
        K::IfStatement,
        K::WhileStatement,
        K::DoStatement,
        K::BasicForStatement,
        K::EnhancedForStatement,
    ],
    elidable_nodes: &[K::EmptyStatement, K::EmptyDeclaration],
};

pub struct JavaCorpus;

impl CorpusLanguage for JavaCorpus {
    type Syntax = JavaLanguage;

    fn name(&self) -> &'static str {
        "Java"
    }

    fn with_parse<R>(
        &self,
        source: &str,
        visit: impl FnOnce(SyntaxNode<'_>, &[Diagnostic]) -> R,
    ) -> R {
        let parse = jolt_java_syntax::parse(source);
        visit(parse.syntax(), parse.diagnostics())
    }

    fn format(&self, source: &str, options: &FormatOptions) -> String {
        format(source, *options)
    }

    fn structure_policy(&self) -> &StructurePolicy<K> {
        &STRUCTURE_POLICY
    }

    fn expects_diagnostics(&self, relative: &str) -> bool {
        let Some(name) = relative.strip_prefix("syntax/parser/") else {
            return false;
        };
        name.starts_with("diagnoses-")
            || name.starts_with("recovers-")
            || name == "disambiguates-when-in-switch-labels--invalid-guard.java"
    }
}

pub fn format(source: &str, options: FormatOptions) -> String {
    let parse = jolt_java_syntax::parse(source);
    let mut output = String::new();
    let _ = jolt_java_fmt::format(&parse, &options, &mut output);
    output
}
