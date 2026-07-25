// Shared by several test binaries, each of which uses a different part.
#![allow(dead_code)]

use jolt_fmt_ir::FormatOptions;
use jolt_kotlin_fmt::format_source_to_sink;
use jolt_kotlin_syntax::{KotlinSyntaxKind, KotlinSyntaxView, parse_kotlin_file};
use jolt_test_support::{
    CorpusLanguage, CorpusParseFacts, StructurePolicy, corpus_parse_facts, format_source_or_panic,
};

/// The only tree edits the Kotlin formatter is allowed to make: it sorts the import
/// list, adds clarifying precedence parentheses, and may normalize separators.
pub(crate) const STRUCTURE_POLICY: StructurePolicy<KotlinSyntaxKind> = StructurePolicy {
    normalizable_punctuation: &[
        KotlinSyntaxKind::Comma,
        KotlinSyntaxKind::Semicolon,
        KotlinSyntaxKind::DoubleSemicolon,
        KotlinSyntaxKind::EolOrSemicolon,
        KotlinSyntaxKind::LParen,
        KotlinSyntaxKind::RParen,
    ],
    unordered_nodes: &[KotlinSyntaxKind::ImportDirectiveList],
    unordered_keywords: &[],
    reorderable_children: &[],
    // Eliding `ParenthesizedExpression` costs no precedence coverage: operator nesting
    // lives in the `BinaryExpression` spine, so a paren that actually mattered still
    // reshapes that spine and still fails.
    elidable_wrappers: &[KotlinSyntaxKind::ParenthesizedExpression],
    promoted_body_wrapper: None,
    brace_promoting_parents: &[],
    elidable_nodes: &[],
};

pub(crate) struct KotlinCorpus;

impl CorpusLanguage for KotlinCorpus {
    fn language_name(&self) -> &'static str {
        "Kotlin"
    }

    fn parse_facts(&self, source: &str) -> CorpusParseFacts {
        let parse = parse_kotlin_file(source);
        let root = parse.syntax().and_then(|file| file.syntax_node());
        corpus_parse_facts(root, parse.diagnostics(), &STRUCTURE_POLICY)
    }

    fn format(&self, source: &str, label: &str) -> String {
        format_source_or_panic(
            format_source_to_sink,
            source,
            &FormatOptions::default(),
            label,
        )
    }

    fn expects_parser_diagnostics(&self, relative: &str) -> bool {
        let Some(name) = relative.strip_prefix("syntax/parser/") else {
            return false;
        };
        name.starts_with("diagnoses-")
            || name.starts_with("recovers-")
            || name == "parses-destructuring-square-preview.kt"
    }
}
